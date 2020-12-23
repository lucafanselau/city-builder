use crate::compat::ToHalType;
use crate::gfx_context::GfxContext;
use gfx_hal::pass::Subpass;
use gfx_hal::pso::{
    AttributeDesc, BakedStates, BasePipeline, BlendDesc, BlendState, ColorBlendDesc, ColorMask,
    DepthStencilDesc, EntryPoint, GraphicsPipelineDesc, InputAssemblerDesc, PipelineCreationFlags,
    PrimitiveAssemblerDesc, ShaderStageFlags, VertexBufferDesc,
};
use gfx_hal::{device::Device, Backend};
use parking_lot::Mutex;
use render::resource::pipeline::{
    GraphicsPipelineDescriptor, RenderContext, ShaderSource, ShaderType,
};
use shaderc::Compiler;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter, Result};
use std::iter;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;

use std::sync::Arc;

// This is the system that provides Pipelines and everything related (like RenderPasses, Shader Compilation, etc. )
#[derive(Debug)]
pub struct Plumber<B: Backend> {
    device: Arc<B::Device>,
    empty_layout: ManuallyDrop<B::PipelineLayout>,
    compiler: Arc<Mutex<ShaderCCompiler>>,
}

struct ShaderCCompiler {
    pub compiler: Compiler,
}

impl Debug for ShaderCCompiler {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("ShaderCCompiler").finish()
    }
}

// NOTE(luca): This is not tested and might break everything
unsafe impl Send for ShaderCCompiler {}

impl<B: Backend> Plumber<B> {
    pub(crate) fn new(device: Arc<B::Device>) -> Self {
        let empty_layout = ManuallyDrop::new(unsafe {
            device
                .create_pipeline_layout(
                    iter::empty::<B::DescriptorSetLayout>(),
                    iter::empty::<(ShaderStageFlags, std::ops::Range<u32>)>(),
                )
                .expect("[Plumber] (init) failed to construct empty pipeline layout")
        });

        let compiler = ShaderCCompiler {
            compiler: Compiler::new().expect("[Plumber] failed to create shaderc compiler"),
        };

        Self {
            device,
            empty_layout,
            compiler: Arc::new(Mutex::new(compiler)),
        }
    }

    fn create_shader_module(&self, spirv: Option<&Vec<u32>>) -> Option<B::ShaderModule> {
        spirv.map(|spirv| unsafe {
            self.device
                .create_shader_module(spirv.as_slice())
                .expect("[Plumber] (create_shader_module) failed to create a shader!")
        })
    }

    pub(crate) fn create_pipeline(
        &self,
        desc: GraphicsPipelineDescriptor<GfxContext<B>>,
        render_context: RenderContext<GfxContext<B>>,
    ) -> (B::GraphicsPipeline, B::PipelineLayout) {
        let vertex_shader = self
            .create_shader_module(Some(&desc.shaders.vertex))
            .unwrap();
        let fragment_shader = self
            .create_shader_module(Some(&desc.shaders.fragment))
            .unwrap();

        let buffers: Vec<VertexBufferDesc> = desc
            .vertex_buffers
            .iter()
            .map(|b| b.clone().convert())
            .collect();
        let attributes: Vec<AttributeDesc> = desc
            .attributes
            .iter()
            .map(|a| a.clone().convert())
            .collect();

        let primitive_assembler = PrimitiveAssemblerDesc::Vertex {
            buffers: buffers.as_slice(),
            attributes: attributes.as_slice(),
            input_assembler: InputAssemblerDesc {
                primitive: desc.primitive.clone().convert(),
                with_adjacency: false,
                restart_index: None,
            },
            vertex: EntryPoint {
                entry: "main",
                module: &vertex_shader,
                specialization: Default::default(),
            },
            tessellation: None,
            geometry: None,
        };

        let depth_stencil = DepthStencilDesc {
            depth: desc.depth.as_ref().map(|d| d.clone().convert()),
            depth_bounds: false,
            stencil: None,
        };

        let baked_states: BakedStates = BakedStates {
            viewport: desc
                .pipeline_states
                .viewport
                .to_option()
                .map(|v| v.convert()),
            scissor: desc
                .pipeline_states
                .scissor
                .to_option()
                .map(|r| r.convert()),
            ..Default::default()
        };

        let layout = unsafe {
            let set_layouts: Vec<&B::DescriptorSetLayout> = desc
                .mixtures
                .into_iter()
                .map(|m| -> &<B as Backend>::DescriptorSetLayout { &m.gpu_layout.handle.1 })
                .collect();
            self.device
                .create_pipeline_layout(
                    set_layouts,
                    iter::empty::<(ShaderStageFlags, std::ops::Range<u32>)>(),
                )
                .expect(
                    format!(
                        "[Plumber] failed to create pipeline layout for pipeline: {}",
                        desc.name
                    )
                    .as_str(),
                )
        };

        // The subpass
        let subpass = match render_context {
            RenderContext::RenderPass((rp, id)) => Subpass {
                index: id,
                main_pass: rp,
            },
            // TODO: of course todo its unimplemented
            RenderContext::Attachments(_) => unimplemented!(),
        };

        let hal_desc = GraphicsPipelineDesc {
            primitive_assembler,
            rasterizer: desc.rasterizer.clone().convert(),
            fragment: Some(EntryPoint {
                entry: "main",
                module: &fragment_shader,
                specialization: Default::default(),
            }),
            blender: BlendDesc {
                logic_op: None,
                targets: desc
                    .blend_targets
                    .iter()
                    .map(|t| if *t { Some(BlendState::ALPHA) } else { None })
                    .map(|blend| ColorBlendDesc {
                        mask: ColorMask::ALL,
                        blend,
                    })
                    .collect(),
            },
            depth_stencil,
            multisampling: None,
            baked_states,
            layout: &layout,
            subpass,
            flags: PipelineCreationFlags::empty(),
            parent: BasePipeline::None,
        };

        let mut pipeline = unsafe {
            self.device
                .create_graphics_pipeline(&hal_desc, None)
                .expect("[Plumber] (create_pipeline) failed to create graphics pipeline")
        };

        unsafe {
            self.device
                .set_graphics_pipeline_name(&mut pipeline, desc.name.deref())
        }

        // Now we can drop the shader modules
        unsafe {
            self.device.destroy_shader_module(vertex_shader);
            self.device.destroy_shader_module(fragment_shader);
        }

        (pipeline, layout)
    }

    fn compile_glsl(
        &self,
        source: &str,
        shader_type: ShaderType,
        name: &str,
    ) -> anyhow::Result<Vec<u32>> {
        use shaderc::*;
        let shader_kind = match shader_type {
            ShaderType::Vertex => ShaderKind::Vertex,
            ShaderType::Fragment => ShaderKind::Fragment,
            ShaderType::Compute => ShaderKind::Compute,
            ShaderType::Geometry => ShaderKind::Geometry,
        };

        // for now we will create the compiler inplace
        // should probably be shared between compilations

        // let mut options = shaderc::CompileOptions::new().ok_or("failed to create compile options")?;

        let binary_result: shaderc::CompilationArtifact = {
            let mut compiler = self.compiler.lock();
            compiler
                .compiler
                .compile_into_spirv(source, shader_kind, name, "main", None)?
        };

        Ok(binary_result.as_binary().to_vec())
    }

    fn load_file(path: Cow<'static, Path>) -> String {
        use std::fs;
        fs::read_to_string(path).expect("[Plumber] failed to load shader file")
    }

    pub(crate) fn compile_shader(&self, source: ShaderSource) -> Vec<u32> {
        match source {
            ShaderSource::GlslFile(path) => {
                let source = Self::load_file(path.clone());
                let name = path
                    .file_name()
                    .map(|n| n.to_str().expect("[Plumber] OsString was invalid unicode"))
                    .unwrap_or("unknown-file-name");

                let shader_type = match path.extension() {
                    Some(extension) => match extension
                        .to_str()
                        .expect("[Plumber] OsStr was not valid unicode")
                    {
                        "vert" => ShaderType::Vertex,
                        "frag" => ShaderType::Fragment,
                        "geometry" => ShaderType::Geometry,
                        "compute" => ShaderType::Compute,
                        _ => panic!("[Plumber] unknown shader extension: {:#?}", extension),
                    },
                    None => panic!("[Plumber] path had no valid extension name"),
                };

                self.compile_glsl(source.as_str(), shader_type, name)
                    .unwrap()
            }
            ShaderSource::GlslSource((source, shader_type, name)) => self
                .compile_glsl(source, shader_type, name.unwrap_or("unknown-inline-shader"))
                .unwrap(),
            ShaderSource::Spirv(source) => source,
        }
    }
}

impl<B: Backend> Drop for Plumber<B> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(ManuallyDrop::take(&mut self.empty_layout));
        }
    }
}
