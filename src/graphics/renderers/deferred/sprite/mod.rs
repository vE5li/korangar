// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/sprite/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/sprite/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::Vector2;
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;
use vulkano::sync::GpuFuture;

use self::vertex_shader::ty::Constants;
use crate::graphics::*;
use crate::loaders::{GameFileLoader, TextureLoader};
use crate::world::MarkerIdentifier;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct SpriteRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ScreenVertexBuffer,
    #[cfg(feature = "debug")]
    object_marker_texture: Texture,
    #[cfg(feature = "debug")]
    light_source_marker_texture: Texture,
    #[cfg(feature = "debug")]
    sound_source_marker_texture: Texture,
    #[cfg(feature = "debug")]
    effect_source_marker_texture: Texture,
    #[cfg(feature = "debug")]
    entity_marker_texture: Texture,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
}

impl SpriteRenderer {
    pub fn new(
        device: Arc<Device>,
        subpass: Subpass,
        viewport: Viewport,
        #[cfg(feature = "debug")] game_file_loader: &mut GameFileLoader,
        #[cfg(feature = "debug")] texture_loader: &mut TextureLoader,
        #[cfg(feature = "debug")] texture_future: &mut Box<dyn GpuFuture + 'static>,
    ) -> Self {
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let vertices = vec![
            ScreenVertex::new(Vector2::new(0.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 1.0)),
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage {
    vertex_buffer: true,
    ..Default::default()
}, false, vertices.into_iter()).unwrap();

        #[cfg(feature = "debug")]
        let object_marker_texture = texture_loader.get("object.png", game_file_loader, texture_future).unwrap();
        #[cfg(feature = "debug")]
        let light_source_marker_texture = texture_loader.get("light.png", game_file_loader, texture_future).unwrap();
        #[cfg(feature = "debug")]
        let sound_source_marker_texture = texture_loader.get("sound.png", game_file_loader, texture_future).unwrap();
        #[cfg(feature = "debug")]
        let effect_source_marker_texture = texture_loader.get("effect.png", game_file_loader, texture_future).unwrap();
        #[cfg(feature = "debug")]
        let entity_marker_texture = texture_loader.get("entity.png", game_file_loader, texture_future).unwrap();

        let nearest_sampler = Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            ..Default::default()
        })
        .unwrap();

        let linear_sampler = Sampler::new(device, SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap();

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            vertex_buffer,
            #[cfg(feature = "debug")]
            object_marker_texture,
            #[cfg(feature = "debug")]
            light_source_marker_texture,
            #[cfg(feature = "debug")]
            sound_source_marker_texture,
            #[cfg(feature = "debug")]
            effect_source_marker_texture,
            #[cfg(feature = "debug")]
            entity_marker_texture,
            nearest_sampler,
            linear_sampler,
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

    fn build(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Texture,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        color: Color,
        smooth: bool,
    ) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let sampler = match smooth {
            true => self.linear_sampler.clone(),
            false => self.nearest_sampler.clone(),
        };

        let set = PersistentDescriptorSet::new(descriptor_layout, [WriteDescriptorSet::image_view_sampler(0, texture, sampler)]).unwrap();

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
            color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
        };

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0)
            .unwrap();
    }

    pub fn render_indexed(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Texture,
        window_size: Vector2<usize>,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        color: Color,
        column_count: usize,
        cell_index: usize,
        smooth: bool,
    ) {
        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);
        let screen_position = Vector2::new(screen_position.x / half_screen.x, screen_position.y / half_screen.y);
        let screen_size = Vector2::new(screen_size.x / half_screen.x, screen_size.y / half_screen.y);

        let unit = 1.0 / column_count as f32;
        let offset_x = unit * (cell_index % column_count) as f32;
        let offset_y = unit * (cell_index / column_count) as f32;

        self.build(
            render_target,
            texture,
            screen_position,
            screen_size,
            Vector2::new(offset_x, offset_y),
            Vector2::new(unit, unit),
            color,
            smooth,
        );
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        marker_identifier: MarkerIdentifier,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        hovered: bool,
    ) {
        let (texture, color) = match marker_identifier {
            MarkerIdentifier::Object(..) if hovered => (self.object_marker_texture.clone(), Color::rgb(235, 180, 52)),
            MarkerIdentifier::Object(..) => (self.object_marker_texture.clone(), Color::rgb(235, 103, 52)),
            MarkerIdentifier::LightSource(..) if hovered => (self.light_source_marker_texture.clone(), Color::rgb(150, 52, 235)),
            MarkerIdentifier::LightSource(..) => (self.light_source_marker_texture.clone(), Color::rgb(52, 235, 217)),
            MarkerIdentifier::SoundSource(..) if hovered => (self.sound_source_marker_texture.clone(), Color::rgb(128, 52, 235)),
            MarkerIdentifier::SoundSource(..) => (self.sound_source_marker_texture.clone(), Color::rgb(235, 52, 140)),
            MarkerIdentifier::EffectSource(..) if hovered => (self.effect_source_marker_texture.clone(), Color::rgb(235, 52, 52)),
            MarkerIdentifier::EffectSource(..) => (self.effect_source_marker_texture.clone(), Color::rgb(52, 235, 156)),
            MarkerIdentifier::Entity(..) if hovered => (self.entity_marker_texture.clone(), Color::rgb(235, 92, 52)),
            MarkerIdentifier::Entity(..) => (self.entity_marker_texture.clone(), Color::rgb(189, 235, 52)),
            _ => panic!(),
        };

        self.build(
            render_target,
            texture,
            screen_position,
            screen_size,
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
            color,
            true,
        );
    }
}
