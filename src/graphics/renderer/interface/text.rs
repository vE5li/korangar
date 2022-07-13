mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/text_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/text_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::iter;
use rusttype::Rect;
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
use crate::loaders::FontLoader;

use self::vertex_shader::ty::Constants;

pub struct TextRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ScreenVertexBuffer,
    nearest_sampler: Arc<Sampler>,
    font_loader: Rc<RefCell<FontLoader>>,
}

impl TextRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport, font_loader: Rc<RefCell<FontLoader>>) -> Self {

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

        let nearest_sampler = Sampler::start(device.clone())
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, nearest_sampler, font_loader }
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

    pub fn render(&self, builder: &mut CommandBuilder, window_size: Vector2<usize>, screen_position: Vector2<f32>, screen_size: Vector2<f32>, clip_size: Vector2<f32>, color: Color) {

        let mut font_loader = self.font_loader.borrow_mut();

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let set = PersistentDescriptorSet::new(descriptor_layout.clone(), [
            WriteDescriptorSet::image_view_sampler(0, font_loader.get_font_atlas(), self.nearest_sampler.clone()),
        ]).unwrap();

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .bind_vertex_buffers(0, self.vertex_buffer.clone());

        let glyphs: Vec<(Vector2<f32>, Vector2<f32>, Rect<f32>)> = font_loader.get("sfuprrr?").into_iter().map(|(glyph, rect)| {

            let position = glyph.position();
            let size = glyph.unpositioned().scale();

            let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
            let position = vector2!(position.x / half_screen.x, position.y / half_screen.y);
            let size = vector2!(size.x / half_screen.x, size.y / half_screen.y);

            (position, size, rect)
        }).collect();

        for (position, size, rect) in glyphs {

            let constants = Constants {
                screen_position: position.into(),
                screen_size: size.into(),
                clip_size: clip_size.into(),
                texture_position: [rect.min.x, rect.min.y],
                texture_size: [rect.width(), rect.height()],
                color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
                _dummy0: Default::default(),
            };

            let set = PersistentDescriptorSet::new(descriptor_layout.clone(), [
                WriteDescriptorSet::image_view_sampler(0, font_loader.get_font_atlas(), self.nearest_sampler.clone()),
            ]).unwrap();

            builder
                .bind_pipeline_graphics(self.pipeline.clone())
                .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
                .bind_vertex_buffers(0, self.vertex_buffer.clone())
                .push_constants(layout.clone(), 0, constants)
                .draw(6, 1, 0, 0).unwrap();
        }
    }
}
