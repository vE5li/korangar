mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/resolve_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/resolve_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::depth_stencil::CompareOp;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use cgmath::Vector3;

use graphics::*;

pub struct Resolver {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
}

impl Resolver {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        Self { pipeline, vertex_shader, fragment_shader }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
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

    pub fn resolve(&self, builder: &mut CommandBuilder, output_buffer: ImageBuffer, normal_buffer: ImageBuffer, depth_buffer: ImageBuffer, vertex_buffer: ScreenVertexBuffer) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();
        
        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view(0, output_buffer),
            WriteDescriptorSet::image_view(1, normal_buffer),
            WriteDescriptorSet::image_view(2, depth_buffer),
        ]).unwrap();

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0).unwrap();
    }
}
