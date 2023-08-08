// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/effect/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/effect/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::{Matrix2, Vector2};
use procedural::profile;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Sampler, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::vertex_shader::ty::Constants;
use super::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct EffectRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    linear_sampler: Arc<Sampler>,
}

impl EffectRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let linear_sampler = Sampler::new(device, SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap();

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            linear_sampler,
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
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(1).blend(EFFECT_ATTACHMENT_BLEND))
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target.state.get_builder().bind_pipeline_graphics(self.pipeline.clone());
    }

    #[profile("render effect")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Texture,
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

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.linear_sampler.clone()),
        ])
        .unwrap();

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
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
