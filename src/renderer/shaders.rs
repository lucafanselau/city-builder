/// This module will provide the pipeline and will check to recompiler after the file changed.
use gfx_hal::{device::Device, Backend};
use std::sync::Arc;

use log::*;

use std::fmt;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::fs;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::renderer::RenderPass;

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

pub struct ShaderSystem<B: Backend> {
    device: Arc<B::Device>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    rx: Receiver<Pipeline<B>>,
    pipeline: Rc<Pipeline<B>>,
}

impl<B: Backend> fmt::Debug for ShaderSystem<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("device", &self.device)
            .finish()
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
) -> Result<Pipeline<B>, Box<dyn Error>> {
    let pipeline_layout = unsafe {
        let pipeline_layout = device.create_pipeline_layout(&[], &[])?;
        Ok::<B::PipelineLayout, Box<dyn Error>>(pipeline_layout)
    }?;

    let pipeline = unsafe {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face, GraphicsPipelineDesc,
            GraphicsShaderSet, Primitive, Rasterizer, Specialization,
        };

        // let vertex_shader = include_str!("../../assets/shaders/triangle.vert");
        // let fragment_shader = include_str!("../../assets/shaders/triangle.frag");
        // we need to load the files at runtime

        let path = std::env::current_dir()?;
        println!("The current directory is {}", path.display());

        let vertex_shader = fs::read_to_string("assets/shaders/triangle.vert")?;
        let fragment_shader = fs::read_to_string("assets/shaders/triangle.frag")?;

        let vertex_shader_module = device.create_shader_module(&compile_shader(
            vertex_shader,
            shaderc::ShaderKind::Vertex,
            Some("triangle.vert"),
        )?)?;

        let fragment_shader_module = device.create_shader_module(&compile_shader(
            fragment_shader,
            shaderc::ShaderKind::Fragment,
            Some("triangle.frag"),
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

impl<B: Backend> ShaderSystem<B> {
    pub fn new(device: Arc<B::Device>, render_pass: Arc<RenderPass<B>>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        // this will be the channel for the newly created pipelines
        let (tx, rx) = channel();

        // Construct the first pipeline
        let pipeline = Rc::new(
            build_pipeline(&device, &render_pass).expect("failed to create inital pipeline"),
        );

        let handle = {
            let running = running.clone();
            let device = device.clone();
            std::thread::spawn(move || {
                // This is our construction thread
                // First we will start a file watcher
                // Create a channel to receive the events.
                let (notify_tx, notify_rx) = channel();
                // Automatically select the best implementation for your platform.
                // You can also access each implementation directly e.g. INotifyWatcher.
                let mut watcher: RecommendedWatcher =
                    Watcher::new(notify_tx, Duration::from_secs(2))
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
                        Ok(event) => match event {
                            Write(path) => {
                                info!("got write to path: {:#?}", path);
                                if let Some(file_name_os) = path.file_name() {
                                    if let Some(file_name) = file_name_os.to_str() {
                                        info!("file_name was {}", file_name);
                                        if file_name == "triangle.frag"
                                            || file_name == "triangle.vert"
                                        {
                                            // -> we need to rebuilt the pipeline
                                            if let Ok(pipeline) =
                                                build_pipeline(&device, &render_pass)
                                            {
                                                info!("rebuild pipeline!");
                                                tx.send(pipeline);
                                            } else {
                                                error!("failed to rebuild pipeline");
                                            }
                                        }
                                    }
                                }
                            }
                            _ => (),
                        },
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

        // test for notify
        // use notify::{RecursiveMode, Watcher};
        // Automatically select the best implementation for your platform.
        // let watcherDevice = device.clone();
        // let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
        // 		error!("happened");
        //     // let device = device.clone();
        // 		use gfx_hal::device::Device;

        //     match res {
        //         Ok(event) => println!("event: {:?}", event),
        //         Err(e) => println!("watch error: {:?}", e),
        //     };
        // })
        // .expect("");

        // // Add a path to be watched. All files and directories at that path and
        // // below will be monitored for changes.
        // watcher
        //     .watch("src/shaders", RecursiveMode::Recursive)
        //     .expect("");

        Self {
            device,
            running,
            handle: Some(handle),
            rx: rx,
            pipeline: pipeline,
        }
    }

    pub fn get_pipeline(&mut self) -> Rc<Pipeline<B>> {
        // check if we got a new one
        match self.rx.try_recv() {
            Ok(pipeline) => self.pipeline = Rc::new(pipeline),
            _ => (),
        };

        return self.pipeline.clone();
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
