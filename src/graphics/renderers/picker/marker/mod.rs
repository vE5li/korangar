vertex_shader!("src/graphics/renderers/picker/marker/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/picker/marker/fragment_shader.glsl");

use std::sync::Arc;

use procedural::profile;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Constants;
use super::PickerSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;
use crate::interface::{ScreenPosition, ScreenSize};
use crate::world::MarkerIdentifier;

pub struct MarkerRenderer {
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    pipeline: Arc<GraphicsPipeline>,
}

impl MarkerRenderer {
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
        PipelineBuilder::<_, { PickerRenderer::subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <PickerRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render marker")]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        marker_identifier: MarkerIdentifier,
    ) {
        if render_target.bind_subrenderer(PickerSubrenderer::Marker) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();
        let picker_target = PickerTarget::Marker(marker_identifier);

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            identifier: picker_target.into(),
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
