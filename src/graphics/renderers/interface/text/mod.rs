vertex_shader!("src/graphics/renderers/interface/text/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/interface/text/fragment_shader.glsl");

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::{Vector2, Vector4};
use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Constants;
use super::InterfaceSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;
use crate::loaders::FontLoader;

pub struct TextRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    font_loader: Rc<RefCell<FontLoader>>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    nearest_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl TextRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport, font_loader: Rc<RefCell<FontLoader>>) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            font_loader,
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
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

    #[profile("render text")]
    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        text: &str,
        window_size: Vector2<usize>,
        screen_position: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        font_size: f32,
    ) -> f32 {
        if render_target.bind_subrenderer(InterfaceSubrenderer::Text) {
            self.bind_pipeline(render_target);
        }

        let mut font_loader = self.font_loader.borrow_mut();
        let texture = font_loader.get_font_atlas();
        let (character_layout, heigth) = font_loader.get(text, color, font_size, clip_size.z - screen_position.x);
        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.nearest_sampler.clone()),
        ]);

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap();

        character_layout.iter().for_each(|(texture_coordinates, position, color)| {
            let screen_position = Vector2::new(
                (screen_position.x + position.min.x as f32) / half_screen.x,
                (screen_position.y + position.min.y as f32) / half_screen.y,
            );

            let screen_size = Vector2::new(
                position.width() as f32 / half_screen.x,
                position.height() as f32 / half_screen.y,
            );

            let texture_position = texture_coordinates.min;
            let texture_size = texture_coordinates.max - texture_coordinates.min; // TODO: use absolute instead

            let constants = Constants {
                screen_position: screen_position.into(),
                screen_size: screen_size.into(),
                clip_size: clip_size.into(),
                texture_position: [texture_position.x, texture_position.y],
                texture_size: [texture_size.x, texture_size.y],
                color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
            };

            render_target
                .state
                .get_builder()
                .push_constants(layout.clone(), 0, constants)
                .unwrap()
                .draw(6, 1, 0, 0)
                .unwrap();
        });

        heigth
    }
}
