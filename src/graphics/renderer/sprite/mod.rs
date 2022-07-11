mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/dynamic_sprite_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/dynamic_sprite_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::shader::ShaderModule;
use vulkano::render_pass::Subpass;
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use vulkano::buffer::BufferUsage;
use cgmath::Vector2;

use crate::graphics::*;

use self::vertex_shader::ty::Constants;

pub struct DynamicSpriteRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ScreenVertexBuffer,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
}

impl DynamicSpriteRenderer {

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

        //let nearest_sampler = create_sampler!(device.clone(), Nearest, MirroredRepeat);
        //let linear_sampler = create_sampler!(device, Linear, MirroredRepeat);
        
        let nearest_sampler = Sampler::start(device.clone())
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            //.mip_lod_bias(1.0)
            //.lod(0.0..=100.0)
            .build()
            .unwrap();
        
        let linear_sampler = Sampler::start(device)
            .filter(Filter::Linear)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            //.mip_lod_bias(1.0)
            //.lod(0.0..=100.0)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, nearest_sampler, linear_sampler }
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

    fn build(&self, builder: &mut CommandBuilder, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, texture_position: Vector2<f32>, texture_size: Vector2<f32>, color: Color, smooth: bool) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let sampler = match smooth {
            true => self.linear_sampler.clone(),
            false => self.nearest_sampler.clone(),
        };

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view_sampler(0, texture, sampler),
        ]).unwrap(); 

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
            color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
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

    pub fn render_direct(&self, builder: &mut CommandBuilder, texture: Texture, screen_position: Vector2<f32>, screen_size: Vector2<f32>, color: Color, smooth: bool) {
        self.build(builder, texture, screen_position, screen_size, Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0), color, smooth);
    }
}
