mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/directional/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/directional/fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;

use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use vulkano::buffer::BufferUsage;
use cgmath::{Vector3, Matrix4};

use crate::graphics::*;

use self::fragment_shader::ty::Constants;
use self::fragment_shader::ty::Matrices;

pub struct DirectionalLightRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices>,
    linear_sampler: Arc<Sampler>,
}

impl DirectionalLightRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());
 
        let linear_sampler = Sampler::start(device)
            .filter(Filter::Linear)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, matrices_buffer, linear_sampler }
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
            .color_blend_state(ColorBlendState::new(1).blend(LIGHT_ATTACHMENT_BLEND))
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, shadow_image: ImageBuffer, light_matrix: Matrix4<f32>, vertex_buffer: ScreenVertexBuffer, direction: Vector3<f32>, color: Color, intensity: f32) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let matrices = Matrices {
            screen_to_world: camera.get_screen_to_world_matrix().into(),
            light: light_matrix.into(),
        };
        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());
        
        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()), 
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()), 
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()), 
            WriteDescriptorSet::image_view_sampler(3, shadow_image, self.linear_sampler.clone()),
            WriteDescriptorSet::buffer(4, matrices_subbuffer),
        ]).unwrap();

        let constants = Constants {
            direction: [direction.x, direction.y, direction.z],
            color: [color.red_f32() * intensity, color.green_f32() * intensity, color.blue_f32() * intensity],
            _dummy0: Default::default(),
        };

        render_target.state.get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0).unwrap();
    }
}
