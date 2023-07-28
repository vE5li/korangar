// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/directional/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/directional/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::{Matrix4, Vector3};
use procedural::profile;
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::memory::allocator::MemoryUsage;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Sampler, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::fragment_shader::ty::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

unsafe impl bytemuck::Zeroable for Matrices {}
unsafe impl bytemuck::Pod for Matrices {}

pub struct DirectionalLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices, MemoryAllocator>,
    linear_sampler: Arc<Sampler>,
}

impl DirectionalLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let matrices_buffer = CpuBufferPool::new(
            memory_allocator.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            MemoryUsage::Upload,
        );
        let linear_sampler = Sampler::new(device, SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap();

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            linear_sampler,
        }
    }

    #[profile]
    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(
        device: Arc<Device>,
        subpass: Subpass,
        viewport: Viewport,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(1).blend(LIGHT_ATTACHMENT_BLEND))
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target.state.get_builder().bind_pipeline_graphics(self.pipeline.clone());
    }

    #[profile("render directional light")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        shadow_image: ImageBuffer,
        light_matrix: Matrix4<f32>,
        direction: Vector3<f32>,
        color: Color,
        intensity: f32,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::DirectionalLight) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let matrices = Matrices {
            screen_to_world: camera.get_screen_to_world_matrix().into(),
            light: light_matrix.into(),
        };
        let matrices_subbuffer = Arc::new(self.matrices_buffer.from_data(matrices).unwrap());

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()),
            WriteDescriptorSet::image_view_sampler(3, shadow_image, self.linear_sampler.clone()),
            WriteDescriptorSet::buffer(4, matrices_subbuffer),
        ])
        .unwrap();

        let constants = Constants {
            direction: [direction.x, direction.y, direction.z],
            color: [
                color.red_f32() * intensity,
                color.green_f32() * intensity,
                color.blue_f32() * intensity,
            ],
            _dummy0: Default::default(),
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
