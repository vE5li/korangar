mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/sprite_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/sprite_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use cgmath::Vector2;

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::render_pass::Subpass;
use vulkano::sampler::Sampler;
use vulkano::buffer::BufferUsage;

use graphics::*;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::vertex_shader::ty::Constants;

pub struct SpriteRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: VertexShader,
    fragment_shader: FragmentShader,
    vertex_buffer: ScreenVertexBuffer,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
}

impl SpriteRenderer {

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

        let nearest_sampler = create_sampler!(device.clone(), Nearest, MirroredRepeat);
        let linear_sampler = create_sampler!(device, Linear, MirroredRepeat);

        return Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, nearest_sampler, linear_sampler };
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

    fn build(&self, builder: &mut CommandBuilder, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, texture_position: Vector2<f32>, texture_size: Vector2<f32>, color: Color, smooth: bool) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let mut set_builder = PersistentDescriptorSet::start(descriptor_layout);

        match smooth {
            true => set_builder.add_sampled_image(texture, self.linear_sampler.clone()).unwrap(),
            false => set_builder.add_sampled_image(texture, self.nearest_sampler.clone()).unwrap(),
        };

        let set = Arc::new(set_builder.build().unwrap());

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }

    pub fn render(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color, smooth: bool) {

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        self.build(builder, texture, screen_position, screen_size, Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0), color, smooth);
    }

    pub fn render_indexed(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color, column_count: usize, cell_index: usize, smooth: bool) {

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let unit = 1.0 / column_count as f32;
        let offset_x = unit * (cell_index % column_count) as f32;
        let offset_y = unit * (cell_index / column_count) as f32;

        self.build(builder, texture, screen_position, screen_size, Vector2::new(offset_x, offset_y), Vector2::new(unit, unit), color, smooth);
    }

    pub fn render_sheet(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color, cell_count: Vector2<usize>, cell_position: Vector2<usize>, smooth: bool) {

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let unit = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let offset = Vector2::new(unit.x * cell_position.x as f32, unit.y * cell_position.y as f32);

        self.build(builder, texture, screen_position, screen_size, offset, unit, color, smooth);
    }

    pub fn render_direct(&self, builder: &mut CommandBuilder, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color, smooth: bool) {
        self.build(builder, texture, screen_position, screen_size, Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0), color, smooth);
    }
}
