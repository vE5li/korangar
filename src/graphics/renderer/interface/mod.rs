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
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::render_pass::Subpass;
use vulkano::buffer::BufferUsage;

use graphics::*;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::vertex_shader::ty::Constants;

pub struct RectangleRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: VertexShader,
    fragment_shader: FragmentShader,
    vertex_buffer: ScreenVertexBuffer,
}

impl RectangleRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();
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

        return Self { pipeline, vertex_shader, fragment_shader, vertex_buffer };
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &VertexShader, fragment_shader: &FragmentShader) -> Arc<GraphicsPipeline> {

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<ScreenVertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(viewport))
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(subpass)
            .build(device)
            .unwrap();

        return Arc::new(pipeline);
    }

    pub fn render(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, screen_position: Vector2<f32>, screen_size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color) {

        let layout = self.pipeline.layout().clone();

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let pixel_size = 1.0 / window_size.y as f32;
        let corner_radius = Vector4::new(corner_radius.x * pixel_size, corner_radius.y * pixel_size, corner_radius.z * pixel_size, corner_radius.w * pixel_size);

        //println!("{:?}", corner_radius);

        //let corner_radius = Vector4::new(0.0025, 0.0025, 0.0025, 0.0025);

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            corner_radius: [corner_radius.x, corner_radius.y, corner_radius.z, corner_radius.w],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }

    /*pub fn render_shadow(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color) {

        let layout = self.pipeline.layout().clone();

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }*/
}
