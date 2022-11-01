// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/shadow/geometry/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/shadow/geometry/fragment_shader.glsl"
    }
}

use std::sync::Arc;

use cgmath::Matrix4;
use vulkano::buffer::{BufferAccess, BufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::{ImageAccess, ImageViewAbstract};
use vulkano::memory::allocator::MemoryUsage;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::vertex_shader::ty::{Constants, Matrices};
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

unsafe impl bytemuck::Zeroable for Matrices {}
unsafe impl bytemuck::Pod for Matrices {}

pub struct GeometryRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    matrices_buffer: CpuBufferPool<Matrices, MemoryAllocator>,
    nearest_sampler: Arc<Sampler>,
}

impl GeometryRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, &vertex_shader, &fragment_shader);
        let matrices_buffer = CpuBufferPool::new(
            memory_allocator.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            MemoryUsage::Upload,
        );

        let nearest_sampler = Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            ..Default::default()
        })
        .unwrap();

        Self {
            memory_allocator,
            pipeline,
            matrices_buffer,
            nearest_sampler,
        }
    }

    fn create_pipeline(
        device: Arc<Device>,
        subpass: Subpass,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn bind_pipeline(&self, render_target: &mut <ShadowRenderer as Renderer>::Target, camera: &dyn Camera, time: f32) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.from_data(matrices).unwrap());
        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [WriteDescriptorSet::buffer(
            0,
            matrices_subbuffer,
        )])
        .unwrap();

        let dimensions = render_target
            .image
            .image()
            .dimensions()
            .width_height()
            .map(|component| component as f32);

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions,
            depth_range: 0.0..1.0,
        };

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .set_viewport(0, [viewport])
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, 0, set);
    }

    pub fn render(
        &self,
        render_target: &mut <ShadowRenderer as Renderer>::Target,
        _camera: &dyn Camera,
        vertex_buffer: ModelVertexBuffer,
        textures: &[Texture],
        world_matrix: Matrix4<f32>,
    ) {
        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 30;

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(1).unwrap().clone();

        let texture_count = textures.len();
        let mut samplers: Vec<(Arc<dyn ImageViewAbstract>, Arc<Sampler>)> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| (texture.clone() as _, self.nearest_sampler.clone()))
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            samplers.push((textures[0].clone() as _, self.nearest_sampler.clone()));
        }

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view_sampler_array(0, 0, samplers),
        ])
        .unwrap();

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 1, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(vertex_count as u32, 1, 0, 0)
            .unwrap();
    }
}
