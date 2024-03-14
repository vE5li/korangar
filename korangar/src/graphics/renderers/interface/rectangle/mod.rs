vertex_shader!("src/graphics/renderers/interface/rectangle/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/interface/rectangle/fragment_shader.glsl");

use std::sync::Arc;

use korangar_procedural::profile;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Constants;
use super::InterfaceSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};

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
        PipelineBuilder::<_, { InterfaceRenderer::subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .multisample(SampleCount::Sample4)
            .color_blend(INTERFACE_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render rectangle")]
    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        corner_radius: CornerRadius,
        color: Color,
    ) {
        if render_target.bind_subrenderer(InterfaceSubrenderer::Rectangle) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();

        let half_screen = window_size / 2.0;
        let screen_position = screen_position / half_screen;
        let screen_size = screen_size / half_screen;

        let pixel_size = 1.0 / window_size.height;
        let corner_radius = corner_radius * pixel_size;

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            screen_clip: screen_clip.into(),
            corner_radius: corner_radius.into(),
            color: color.into(),
            aspect_ratio: window_size.height / window_size.width,
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
