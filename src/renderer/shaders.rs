use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::mem::ManuallyDrop;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

/// This module will provide the pipeline and will check to recompiler after the file changed.
use gfx_hal::{Backend, device::Device, pso};
use gfx_hal::pso::{AttributeDesc, VertexBufferDesc};
use log::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::renderer::RenderPass;

// use std::fmt;

#[derive(Debug)]
pub struct Pipeline<B: Backend> {
    device: Arc<B::Device>,
    pub pipeline_layout: ManuallyDrop<B::PipelineLayout>,
    pub pipeline: ManuallyDrop<B::GraphicsPipeline>,
}

impl<B: Backend> Drop for Pipeline<B> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_graphics_pipeline(ManuallyDrop::take(&mut self.pipeline));
            self.device
                .destroy_pipeline_layout(ManuallyDrop::take(&mut self.pipeline_layout));
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstructData {
    vertex_file: String,
    fragment_file: String,
    vertex_buffers: Vec<VertexBufferDesc>,
    attributes: Vec<AttributeDesc>,
    push_constants: Vec<(pso::ShaderStageFlags, Range<u32>)>,
}

impl ConstructData {
    pub fn new(vertex_file: String, fragment_file: String, vertex_buffers: Vec<VertexBufferDesc>,
               attributes: Vec<AttributeDesc>,
               push_constants: Vec<(pso::ShaderStageFlags, Range<u32>)>) -> Self {
        ConstructData {
            vertex_file,
            fragment_file,
            vertex_buffers,
            attributes,
            push_constants,
        }
    }
}

fn compile_shader(
    glsl: String,
    shader_type: shaderc::ShaderKind,
    shader_name: Option<&str>,
) -> Result<Vec<u32>, Box<dyn Error>> {
    use shaderc::*;
    use std::io::Cursor;

    // for now we will create the compiler inplace
    // should probably be shared between compilations
    let mut compiler = Compiler::new().ok_or("failed to create shaderc compiler")?;
    // let mut options = shaderc::CompileOptions::new().ok_or("failed to create compile options")?;
    let binary_result: shaderc::CompilationArtifact = compiler.compile_into_spirv(
        &glsl,
        shader_type,
        shader_name.unwrap_or("shader.glsl"),
        "main",
        None,
    )?;

    let spirv = gfx_hal::pso::read_spirv(Cursor::new(binary_result.as_binary_u8().to_vec()))?;

    Ok(spirv)
}

fn build_pipeline<B: Backend>(
    device: &Arc<B::Device>,
    render_pass: &Arc<RenderPass<B>>,
    data: ConstructData,
) -> Result<Pipeline<B>, Box<dyn Error>> {
    let pipeline_layout = unsafe {
        // let push_constant_bytes = std::mem::size_of::<PushConstants>() as u32;

        // let pipeline_layout = device
        //     .create_pipeline_layout(&[], &)?;

        let pipeline_layout = device.create_pipeline_layout(&[], &data.push_constants)?;

        Ok::<B::PipelineLayout, Box<dyn Error>>(pipeline_layout)
    }?;

    let pipeline = unsafe {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face,
            GraphicsPipelineDesc, GraphicsShaderSet, Primitive, Rasterizer, Specialization,
        };

        // let vertex_shader = include_str!("../../assets/shaders/triangle.vert");
        // let fragment_shader = include_str!("../../assets/shaders/triangle.frag");
        // we need to load the files at runtime

        let path = std::env::current_dir()?;
        println!("The current directory is {}", path.display());

        let vertex_shader = fs::read_to_string(format!("assets/shaders/{}", data.vertex_file))?;
        let fragment_shader = fs::read_to_string(format!("assets/shaders/{}", data.fragment_file))?;

        let vertex_shader_module = device.create_shader_module(&compile_shader(
            vertex_shader,
            shaderc::ShaderKind::Vertex,
            Some(data.vertex_file.as_str()),
        )?)?;

        let fragment_shader_module = device.create_shader_module(&compile_shader(
            fragment_shader,
            shaderc::ShaderKind::Fragment,
            Some(data.fragment_file.as_str()),
        )?)?;

        let (vs_entry, fs_entry) = (
            EntryPoint {
                entry: "main",
                module: &vertex_shader_module,
                specialization: Specialization::default(),
            },
            EntryPoint {
                entry: "main",
                module: &fragment_shader_module,
                specialization: Specialization::default(),
            },
        );

        let shader_set = GraphicsShaderSet {
            vertex: vs_entry,
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(fs_entry),
        };

        let rp: &B::RenderPass = &(*render_pass.render_pass);

        let mut pipeline_desc = GraphicsPipelineDesc::new(
            shader_set,
            Primitive::TriangleList,
            Rasterizer {
                cull_face: Face::BACK,
                ..Rasterizer::FILL
            },
            &pipeline_layout,
            Subpass {
                index: 0,
                main_pass: rp,
            },
        );

        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA),
        });

        // Vertex Buffer description
        {
            pipeline_desc.vertex_buffers = data.vertex_buffers;
            pipeline_desc.attributes = data.attributes;
        }

        // Depth Stencil
        {
            use gfx_hal::pso::{Comparison, DepthTest};

            pipeline_desc.depth_stencil.depth = Some(DepthTest {
                fun: Comparison::Less,
                write: true,
            });

            pipeline_desc.depth_stencil.depth_bounds = false;
            // Maybe that is the default... Who knows
            pipeline_desc.depth_stencil.stencil = None;
        }

        let pipeline = device.create_graphics_pipeline(&pipeline_desc, None)?;

        device.destroy_shader_module(vertex_shader_module);
        device.destroy_shader_module(fragment_shader_module);

        Ok::<B::GraphicsPipeline, Box<dyn Error>>(pipeline)
    }?;

    Ok(Pipeline {
        device: device.clone(),
        pipeline_layout: ManuallyDrop::new(pipeline_layout),
        pipeline: ManuallyDrop::new(pipeline),
    })
}

#[derive(Debug)]
pub struct ShaderSystem<B: Backend> {
    device: Arc<B::Device>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    rx: Receiver<(String, Pipeline<B>)>,
    store: HashMap<String, Rc<Pipeline<B>>>,
    /// Instructions to build the Pipeline
    blueprints: Arc<RwLock<HashMap<String, ConstructData>>>,
    /// Just used to build the first pipeline when add pipeline is called
    render_pass: Arc<RenderPass<B>>
}

impl<B: Backend> ShaderSystem<B> {
    pub fn new(device: Arc<B::Device>, render_pass: Arc<RenderPass<B>>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        // this will be the channel for the newly created pipelines
        let (tx, rx) = channel();

        let blueprints = Arc::new(RwLock::new(HashMap::<String, ConstructData>::new()));

        let handle = {
            let running = running.clone();
            let device = device.clone();
            let render_pass = render_pass.clone();
            let blueprints = blueprints.clone();
            std::thread::spawn(move || {
                // This is our construction thread
                // First we will start a file watcher
                // Create a channel to receive the events.
                let (notify_tx, notify_rx) = channel();
                // Automatically select the best implementation for your platform.
                // You can also access each implementation directly e.g. INotifyWatcher.
                let mut watcher: RecommendedWatcher =
                    Watcher::new(notify_tx, Duration::from_secs(1))
                        .expect("failed to create file watcher");

                // Add a path to be watched. All files and directories at that path and
                // below will be monitored for changes.
                watcher
                    .watch("./assets/shaders", RecursiveMode::Recursive)
                    .expect("failed to create file watcher");
                // This is a simple loop, but you may want to use more complex logic here,
                // for example to handle I/O.
                loop {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }

                    use notify::DebouncedEvent::*;
                    match notify_rx.try_recv() {
                        Ok(event) => {
                            match event {
                                Write(path) => {
                                    info!("got write to path: {:#?}", path);
                                    if let Some(file_name_os) = path.file_name() {
                                        if let Some(file_name) = file_name_os.to_str() {
                                            let file_name = String::from(file_name);
                                            info!("file_name was {}", file_name);
                                            // Search for file name in the store
                                            let found = {
                                                blueprints.read().expect("shader_thread: failed to acquire read lock").iter().find(|(_, data)| {
                                                    data.vertex_file == file_name || data.fragment_file == file_name
                                                }).map(|(name, data)| (name.clone(), data.clone()))
                                            };

                                            if let Some((name, data)) = found {
                                                // -> we need to rebuilt the pipeline
                                                if let Ok(pipeline) =
                                                build_pipeline(&device, &render_pass, data)
                                                {
                                                    info!("rebuild pipeline!");
                                                    tx.send((name.clone(), pipeline)).expect("failed to send new pipeline");
                                                } else {
                                                    error!("failed to rebuild pipeline");
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => (),
                            }
                        }
                        Err(e) => {
                            match e {
                                // We will wait for an event
                                std::sync::mpsc::TryRecvError::Empty => {
                                    std::thread::sleep(Duration::from_millis(500))
                                }
                                std::sync::mpsc::TryRecvError::Disconnected => {
                                    error!("Sender got disconnected. Terminating now");
                                    running.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }

                ()
            })
        };

        Self {
            device,
            running,
            handle: Some(handle),
            rx,
            store: HashMap::new(),
            blueprints,
            render_pass
        }
    }

    pub fn add_pipeline(&mut self, name: String, data: ConstructData) {
        // Construct the first pipeline
        let pipeline = Rc::new(
            build_pipeline(&self.device, &self.render_pass, data.clone())
                .expect("failed to create inital pipeline"),
        );

        self.store.insert(name.clone(), pipeline);
        self.blueprints
            .write()
            .expect("failed to acquire write lock")
            .insert(name, data);
    }

    pub fn poll(&mut self) {
        while let Ok((name, pipeline)) = self.rx.try_recv() {
            match self
                .store
                .get_mut(&name)
            {
                Some(data) => *data = Rc::new(pipeline),
                None => error!("failed to get pipeline in store for name: {}", name),
            }
        }
    }

    pub fn get_pipeline(&self, name: String) -> Option<Rc<Pipeline<B>>> {
        match self
            .store
            .get(&name)
        {
            Some(data) => Some(data.clone()),
            None => None,
        }
    }
}

impl<B: Backend> Drop for ShaderSystem<B> {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.handle
            .take()
            .unwrap()
            .join()
            .expect("failed to join thread");
    }
}
