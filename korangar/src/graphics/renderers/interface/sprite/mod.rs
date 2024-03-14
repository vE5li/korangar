vertex_shader!("src/graphics/renderers/interface/sprite/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/interface/sprite/fragment_shader.glsl");

use std::sync::Arc;

use korangar_procedural::profile;
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
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};

pub struct SpriteRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl SpriteRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
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

    #[profile("render sprite")]
    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        texture: Arc<ImageView>,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) {
        if render_target.bind_subrenderer(InterfaceSubrenderer::Sprite) {
            self.bind_pipeline(render_target);
        }

        let half_screen = window_size / 2.0;
        let screen_position = screen_position / half_screen;
        let screen_size = screen_size / half_screen;

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
            screen_clip: screen_clip.into(),
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
}
