// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/point/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/point/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::Vector3;
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
use vulkano::shader::ShaderModule;

use self::fragment_shader::ty::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

unsafe impl bytemuck::Zeroable for Matrices {}
unsafe impl bytemuck::Pod for Matrices {}

pub struct PointLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices, MemoryAllocator>,
}

impl PointLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);
        let matrices_buffer = CpuBufferPool::new(
            memory_allocator.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            MemoryUsage::Upload,
        );

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
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
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let screen_to_world_matrix = camera.get_screen_to_world_matrix();
        let matrices = Matrices {
            screen_to_world: screen_to_world_matrix.into(),
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.from_data(matrices).unwrap());
        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()),
            WriteDescriptorSet::buffer(3, matrices_subbuffer),
        ])
        .unwrap();

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, 0, set);
    }

    #[profile("render point light")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        position: Vector3<f32>,
        color: Color,
        range: f32,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::PointLight) {
            self.bind_pipeline(render_target, camera);
        }

        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, 10.0 * (range / 0.05).ln());

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > range {
            return;
        }

        let layout = self.pipeline.layout().clone();

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            position: [position.x, position.y, position.z],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
            range,
            _dummy0: Default::default(),
        };

        render_target
            .state
            .get_builder()
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
