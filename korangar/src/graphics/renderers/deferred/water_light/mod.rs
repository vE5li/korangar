vertex_shader!("src/graphics/renderers/deferred/water_light/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/water_light/fragment_shader.glsl");

use std::sync::Arc;

use korangar_debug::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::Constants;
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::{allocate_descriptor_set, *};

pub struct WaterLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    pipeline: Arc<GraphicsPipeline>,
}

impl WaterLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            pipeline,
        }
    }

    #[korangar_debug::profile]
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
            .color_blend(WATER_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render water light")]
    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, water_level: f32) {
        if render_target.bind_subrenderer(DeferredSubrenderer::WaterLight) {
            self.bind_pipeline(render_target);
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view(0, render_target.water_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.depth_image.clone()),
        ]);

        let constants = Constants {
            screen_to_world_matrix: camera.get_screen_to_world_matrix().into(),
            water_level,
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap()
            .push_constants(layout, 0, constants)
            .unwrap()
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
