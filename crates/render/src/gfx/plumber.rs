use crate::gfx::compat::ToHalType;
use crate::resource::pipeline::GraphicsPipelineDescriptor;
use gfx_hal::pass::Subpass;
use gfx_hal::pso::{
    AttributeDesc, BasePipeline, DepthStencilDesc, EntryPoint, GraphicsPipelineDesc,
    InputAssemblerDesc, PipelineCreationFlags, PrimitiveAssemblerDesc, ShaderStageFlags,
    VertexBufferDesc,
};
use gfx_hal::{device::Device, Backend};
use std::iter;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

// This is the system that provides Pipelines and everything related (like RenderPasses, Shader Compilation, etc. )
struct Plumber<B: Backend> {
    device: Arc<B::Device>,
    empty_layout: ManuallyDrop<B::PipelineLayout>,
}

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

        Self {
            device,
            empty_layout,
        }
    }

    fn create_shader_module(&self, spirv: Option<&Vec<u32>>) -> Option<B::ShaderModule> {
        spirv.map(|spirv| unsafe {
            self.device
                .create_shader_module(spirv.as_slice())
                .expect("[Plumber] (create_shader_module) failed to create a shader!")
        })
    }

    pub(crate) fn create_pipeline(&self, desc: &GraphicsPipelineDescriptor) -> B::GraphicsPipeline {
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
                primitive: desc.primitive.convert(),
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
            depth: desc.depth.map(|d| d.clone().convert()),
            depth_bounds: false,
            stencil: None,
        };

        let hal_desc = GraphicsPipelineDesc {
            primitive_assembler,
            rasterizer: desc.rasterizer.convert(),
            fragment: Some(EntryPoint {
                entry: "main",
                module: &fragment_shader,
                specialization: Default::default(),
            }),
            blender: Default::default(),
            depth_stencil,
            multisampling: None,
            baked_states: Default::default(),
            layout: self.empty_layout.deref(),
            subpass: Subpass {
                index: 0,
                main_pass: &(),
            },
            flags: PipelineCreationFlags::empty(),
            parent: BasePipeline::None,
        };

        let pipeline = unsafe {
            self.device
                .create_graphics_pipeline(&hal_desc, None)
                .expect("[Plumber] (create_pipeline) failed to create graphics pipeline")
        };

        // Now we can drop the shader modules
        unsafe {
            self.device.destroy_shader_module(vertex_shader);
            self.device.destroy_shader_module(fragment_shader);
        }

        pipeline
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
