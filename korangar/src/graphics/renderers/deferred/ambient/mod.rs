vertex_shader!("src/graphics/renderers/deferred/ambient/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/ambient/fragment_shader.glsl");

use std::sync::Arc;

use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::Constants;
use super::{DeferredRenderer, DeferredSubrenderer};
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::{allocate_descriptor_set, *};

pub struct AmbientLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
}

impl AmbientLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
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
            .vertex_input_state::<ModelVertex>(vertex_shader)
            .fixed_viewport(viewport)
            .color_blend(LIGHT_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render ambient light"))]
    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, color: Color) {
        if render_target.bind_subrenderer(DeferredSubrenderer::AmbientLight) {
            self.bind_pipeline(render_target);
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
        ]);

        let constants = Constants { color: color.into() };

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
