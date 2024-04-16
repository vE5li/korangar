vertex_shader!("src/graphics/renderers/deferred/sprite/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/sprite/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Vector2;
use korangar_debug::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Constants;
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;
use crate::interface::layout::{ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::loaders::{GameFileLoader, TextureLoader};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub struct SpriteRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    #[cfg(feature = "debug")]
    object_marker_texture: Arc<ImageView>,
    #[cfg(feature = "debug")]
    light_source_marker_texture: Arc<ImageView>,
    #[cfg(feature = "debug")]
    sound_source_marker_texture: Arc<ImageView>,
    #[cfg(feature = "debug")]
    effect_source_marker_texture: Arc<ImageView>,
    #[cfg(feature = "debug")]
    entity_marker_texture: Arc<ImageView>,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl SpriteRenderer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        subpass: Subpass,
        viewport: Viewport,
        #[cfg(feature = "debug")] game_file_loader: &mut GameFileLoader,
        #[cfg(feature = "debug")] texture_loader: &mut TextureLoader,
    ) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);

        #[cfg(feature = "debug")]
        let object_marker_texture = texture_loader.get("object.png", game_file_loader).unwrap();
        #[cfg(feature = "debug")]
        let light_source_marker_texture = texture_loader.get("light.png", game_file_loader).unwrap();
        #[cfg(feature = "debug")]
        let sound_source_marker_texture = texture_loader.get("sound.png", game_file_loader).unwrap();
        #[cfg(feature = "debug")]
        let effect_source_marker_texture = texture_loader.get("effect.png", game_file_loader).unwrap();
        #[cfg(feature = "debug")]
        let entity_marker_texture = texture_loader.get("entity.png", game_file_loader).unwrap();

        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
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
            pipeline,
        }
    }

    #[korangar_debug::profile]
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
        PipelineBuilder::<_, { DeferredRenderer::lighting_subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .blend_alpha()
            .build(device, subpass)
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    fn build(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Arc<ImageView>,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        color: Color,
        smooth: bool,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Sprite) {
            self.bind_pipeline(render_target);
        }

        let sampler = match smooth {
            true => self.linear_sampler.clone(),
            false => self.nearest_sampler.clone(),
        };

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view_sampler(0, texture, sampler),
        ]);

        let constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            texture_position: texture_position.into(),
            texture_size: texture_size.into(),
            color: color.into(),
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap()
            .push_constants(layout, 0, constants)
            .unwrap()
            .draw(6, 1, 0, 0)
            .unwrap();
    }

    #[profile("render sprite indexed")]
    pub fn render_indexed(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Arc<ImageView>,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        column_count: usize,
        cell_index: usize,
        smooth: bool,
    ) {
        let half_screen = ScreenSize {
            width: window_size.width / 2.0,
            height: window_size.height / 2.0,
        };
        let screen_position = ScreenPosition {
            left: screen_position.left / half_screen.width,
            top: screen_position.top / half_screen.height,
        };
        let screen_size = ScreenSize {
            width: screen_size.width / half_screen.width,
            height: screen_size.height / half_screen.height,
        };

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
    #[profile("render marker")]
    pub fn render_marker(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        marker_identifier: MarkerIdentifier,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        hovered: bool,
    ) {
        let (texture, color) = match marker_identifier {
            MarkerIdentifier::Object(..) if hovered => (self.object_marker_texture.clone(), Color::rgb_u8(235, 180, 52)),
            MarkerIdentifier::Object(..) => (self.object_marker_texture.clone(), Color::rgb_u8(235, 103, 52)),
            MarkerIdentifier::LightSource(..) if hovered => (self.light_source_marker_texture.clone(), Color::rgb_u8(150, 52, 235)),
            MarkerIdentifier::LightSource(..) => (self.light_source_marker_texture.clone(), Color::rgb_u8(52, 235, 217)),
            MarkerIdentifier::SoundSource(..) if hovered => (self.sound_source_marker_texture.clone(), Color::rgb_u8(128, 52, 235)),
            MarkerIdentifier::SoundSource(..) => (self.sound_source_marker_texture.clone(), Color::rgb_u8(235, 52, 140)),
            MarkerIdentifier::EffectSource(..) if hovered => (self.effect_source_marker_texture.clone(), Color::rgb_u8(235, 52, 52)),
            MarkerIdentifier::EffectSource(..) => (self.effect_source_marker_texture.clone(), Color::rgb_u8(52, 235, 156)),
            MarkerIdentifier::Entity(..) if hovered => (self.entity_marker_texture.clone(), Color::rgb_u8(235, 92, 52)),
            MarkerIdentifier::Entity(..) => (self.entity_marker_texture.clone(), Color::rgb_u8(189, 235, 52)),
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
