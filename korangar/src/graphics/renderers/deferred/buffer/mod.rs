vertex_shader!("src/graphics/renderers/deferred/buffer/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/buffer/fragment_shader.glsl");

use std::sync::Arc;

use korangar_procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::Constants;
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;

pub struct BufferRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    nearest_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl BufferRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
            pipeline,
        }
    }

    #[korangar_procedural::profile]
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
            .build(device, subpass)
    }

    #[korangar_procedural::profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render buffers")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        picker_image: Arc<ImageView>,
        light_image: Arc<ImageView>,
        font_atlas: Arc<ImageView>,
        render_settings: &RenderSettings,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Buffers) {
            self.bind_pipeline(render_target);
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.water_image.clone()),
            WriteDescriptorSet::image_view(3, render_target.depth_image.clone()),
            WriteDescriptorSet::image_view_sampler(4, picker_image, self.nearest_sampler.clone()),
            WriteDescriptorSet::image_view_sampler(5, light_image, self.nearest_sampler.clone()),
            WriteDescriptorSet::image_view_sampler(6, font_atlas, self.nearest_sampler.clone()),
        ]);

        let constants = Constants {
            show_diffuse_buffer: render_settings.show_diffuse_buffer as u32,
            show_normal_buffer: render_settings.show_normal_buffer as u32,
            show_water_buffer: render_settings.show_water_buffer as u32,
            show_depth_buffer: render_settings.show_depth_buffer as u32,
            show_picker_buffer: render_settings.show_picker_buffer as u32,
            show_shadow_buffer: render_settings.show_shadow_buffer as u32,
            show_font_atlas: render_settings.show_font_atlas as u32,
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
