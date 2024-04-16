vertex_shader!("src/graphics/renderers/deferred/rectangle/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/rectangle/fragment_shader.glsl");

use std::sync::Arc;

use korangar_debug::profile;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Constants;
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;
use crate::interface::layout::{ScreenPosition, ScreenSize};

pub struct RectangleRenderer {
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    pipeline: Arc<GraphicsPipeline>,
}

impl RectangleRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
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
            .blend_alpha()
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

    #[profile("render rectangle")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Rectangle) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();

        let half_screen = window_size / 2.0;
        let screen_position = screen_position / half_screen;
        let screen_size = screen_size / half_screen;

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            color: color.into(),
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
