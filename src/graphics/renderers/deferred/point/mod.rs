vertex_shader!("src/graphics/renderers/deferred/point/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/point/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Vector3;
use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::padded::Padded;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;

pub struct PointLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    pipeline: Arc<GraphicsPipeline>,
}

impl PointLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            pipeline,
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
        vertex_shader: &EntryPoint,
        fragment_shader: &EntryPoint,
    ) -> Arc<GraphicsPipeline> {
        PipelineBuilder::<_, { DeferredRenderer::lighting_subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .color_blend(LIGHT_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {
        let screen_to_world_matrix = camera.get_screen_to_world_matrix();
        let buffer = self.matrices_buffer.allocate(Matrices {
            screen_to_world: screen_to_world_matrix.into(),
        });

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()),
            WriteDescriptorSet::buffer(3, buffer),
        ]);

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, set_id, set)
            .unwrap();
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
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            position: Padded(position.into()),
            color: color.into(),
            range,
        };

        render_target
            .state
            .get_builder()
            .push_constants(layout, 0, constants)
            .unwrap()
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
