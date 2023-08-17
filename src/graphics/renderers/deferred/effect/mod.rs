vertex_shader!("src/graphics/renderers/deferred/effect/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/effect/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::{Matrix2, Vector2};
use procedural::profile;
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

pub struct EffectRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl EffectRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            linear_sampler,
            pipeline,
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
        PipelineBuilder::<_, { DeferredRenderer::lighting_subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .color_blend(EFFECT_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render effect")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Arc<ImageView>,
        window_size: Vector2<usize>,
        mut screen_positions: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        screen_space_position: Vector2<f32>,
        offset: Vector2<f32>,
        angle: f32,
        color: Color,
    ) {
        const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

        if render_target.bind_subrenderer(DeferredSubrenderer::Effect) {
            self.bind_pipeline(render_target);
        }

        let half_screen = Vector2::new(window_size.x as f32 / 2.0, window_size.y as f32 / 2.0);

        // TODO: move this calculation to the loading
        let rotation_matrix = Matrix2::from_angle(cgmath::Deg(angle / (1024.0 / 360.0)));

        screen_positions
            .iter_mut()
            .for_each(|position| *position = (rotation_matrix * *position) + offset - EFFECT_ORIGIN - half_screen);

        let screen_positions = screen_positions.map(|position| {
            [
                position.x / half_screen.x + screen_space_position.x,
                position.y / half_screen.y + screen_space_position.y,
            ]
        });
        // TODO: as_array()
        let texture_coordinates = texture_coordinates.map(|coordinate| [coordinate.x, coordinate.y]);

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.linear_sampler.clone()),
        ]);

        // TODO: apply angle
        let constants = Constants {
            top_left: screen_positions[0],
            bottom_left: screen_positions[2],
            top_right: screen_positions[1],
            bottom_right: screen_positions[3],
            t_top_left: texture_coordinates[2],
            t_bottom_left: texture_coordinates[3],
            t_top_right: texture_coordinates[1],
            t_bottom_right: texture_coordinates[0],
            color: [color.red_f32(), color.green_f32(), color.blue_f32(), color.alpha_f32()],
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
}
