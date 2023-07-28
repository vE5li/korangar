// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/rectangle/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/rectangle/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::Vector2;
use procedural::profile;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;

use self::vertex_shader::ty::Constants;
use super::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct RectangleRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
}

impl RectangleRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
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
            .color_blend_state(ColorBlendState::new(1).blend_alpha())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target.state.get_builder().bind_pipeline_graphics(self.pipeline.clone());
    }

    #[profile("render rectangle")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        window_size: Vector2<usize>,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        color: Color,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Rectangle) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
        };

        render_target
            .state
            .get_builder()
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
