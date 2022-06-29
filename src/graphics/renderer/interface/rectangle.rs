mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/rectangle_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/rectangle_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use cgmath::{ Vector4, Vector2 };

use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::{ GraphicsPipeline, Pipeline };
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::buffer::BufferUsage;

use graphics::*;

use self::vertex_shader::ty::Constants;

pub struct RectangleRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ScreenVertexBuffer,
}

impl RectangleRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let vertices = vec![
            ScreenVertex::new(Vector2::new(0.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 1.0))
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

        Self { pipeline, vertex_shader, fragment_shader, vertex_buffer }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ScreenVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(1).blend_alpha())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, screen_position: Vector2<f32>, screen_size: Vector2<f32>, clip_size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color) {

        let layout = self.pipeline.layout().clone();

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let pixel_size = 1.0 / window_size.y as f32;
        let corner_radius = Vector4::new(corner_radius.x * pixel_size, corner_radius.y * pixel_size, corner_radius.z * pixel_size, corner_radius.w * pixel_size);

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            clip_size: clip_size.into(),
            corner_radius: corner_radius.into(),
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
            _dummy0: [0, 0, 0, 0, 0, 0, 0, 0],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }
}
