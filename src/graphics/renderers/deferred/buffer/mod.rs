// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/buffer/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/buffer/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::image::StorageImage;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Sampler, SamplerCreateInfo, Filter};
use vulkano::shader::ShaderModule;

use self::fragment_shader::ty::Constants;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct BufferRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    nearest_sampler: Arc<Sampler>,
}

impl BufferRenderer {
    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let nearest_sampler = Sampler::new(device, SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            ..Default::default()
        }).unwrap();

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
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
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        picker_image: ImageBuffer,
        light_image: ImageBuffer,
        font_atlas: Arc<ImageView<StorageImage>>,
        vertex_buffer: ScreenVertexBuffer,
        render_settings: &RenderSettings,
    ) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.water_image.clone()),
            WriteDescriptorSet::image_view(3, render_target.depth_image.clone()),
            WriteDescriptorSet::image_view_sampler(4, picker_image, self.nearest_sampler.clone()),
            WriteDescriptorSet::image_view_sampler(5, light_image, self.nearest_sampler.clone()),
            WriteDescriptorSet::image_view_sampler(6, font_atlas, self.nearest_sampler.clone()),
        ])
        .unwrap();

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
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0)
            .unwrap();
    }
}
