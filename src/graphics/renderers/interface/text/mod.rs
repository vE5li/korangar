// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/interface/text/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/interface/text/fragment_shader.glsl"
    }
}

use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::{Vector2, Vector4};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::vertex_shader::ty::Constants;
use crate::graphics::*;
use crate::loaders::FontLoader;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct TextRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    nearest_sampler: Arc<Sampler>,
    font_loader: Rc<RefCell<FontLoader>>,
}

impl TextRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport, font_loader: Rc<RefCell<FontLoader>>) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let nearest_sampler = Sampler::new(device, SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            ..Default::default()
        })
        .unwrap();

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
            font_loader,
        }
    }

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
            .color_blend_state(ColorBlendState::new(1).blend(INTERFACE_ATTACHMENT_BLEND))
            .multisample_state(MultisampleState {
                rasterization_samples: vulkano::image::SampleCount::Sample4,
                ..Default::default()
            })
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        text: &str,
        window_size: Vector2<usize>,
        screen_position: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        font_size: f32,
    ) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let mut font_loader = self.font_loader.borrow_mut();
        let texture = font_loader.get_font_atlas();
        let character_layout = font_loader.get(text, font_size);

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.nearest_sampler.clone()),
        ])
        .unwrap();

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set);

        character_layout.iter().for_each(|(texture_coordinates, position)| {
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
                .draw(6, 1, 0, 0)
                .unwrap();
        });
    }
}
