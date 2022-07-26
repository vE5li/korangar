mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/shadow/geometry/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/shadow/geometry/fragment_shader.glsl"
    }
}

use std::sync::Arc;

use vulkano::device::Device;

use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::shader::ShaderModule;
use vulkano::render_pass::Subpass;
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use vulkano::buffer::{ BufferUsage, BufferAccess };

use crate::types::maths::*;
use crate::graphics::*;

use self::vertex_shader::ty::Constants;
use self::vertex_shader::ty::Matrices;

pub struct GeometryRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices>,
    linear_sampler: Arc<Sampler>,
}

impl GeometryRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, &vertex_shader, &fragment_shader);
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        let linear_sampler = Sampler::start(device)
            .filter(Filter::Linear)
            .address_mode(SamplerAddressMode::ClampToEdge)
            .min_lod(1.0)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, matrices_buffer, linear_sampler }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass) {
        self.pipeline = Self::create_pipeline(device, subpass, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {

        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn bind_pipeline(&self, render_target: &mut <ShadowRenderer as Renderer>::Target, camera: &dyn Camera) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());
        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::buffer(0, matrices_subbuffer),
        ]).unwrap();

        render_target.state.get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set);
    }

    pub fn render(&self, render_target: &mut <ShadowRenderer as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>) {

        if textures.is_empty() {
            return;
        }

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(1).unwrap().clone();

        // SUPER DIRTY, PLEASE FIX

        let texture0 = textures[0].clone();

        let texture1 = match textures.len() > 1 {
            true => textures[1].clone(),
            false => texture0.clone(),
        };

        let texture2 = match textures.len() > 2 {
            true => textures[2].clone(),
            false => texture0.clone(),
        };

        let texture3 = match textures.len() > 3 {
            true => textures[3].clone(),
            false => texture0.clone(),
        };

        let texture4 = match textures.len() > 4 {
            true => textures[4].clone(),
            false => texture0.clone(),
        };

        let texture5 = match textures.len() > 5 {
            true => textures[5].clone(),
            false => texture0.clone(),
        };

        let texture6 = match textures.len() > 6 {
            true => textures[6].clone(),
            false => texture0.clone(),
        };

        let texture7 = match textures.len() > 7 {
            true => textures[7].clone(),
            false => texture0.clone(),
        };

        let texture8 = match textures.len() > 8 {
            true => textures[8].clone(),
            false => texture0.clone(),
        };

        let texture9 = match textures.len() > 9 {
            true => textures[9].clone(),
            false => texture0.clone(),
        };

        let texture10 = match textures.len() > 10 {
            true => textures[10].clone(),
            false => texture0.clone(),
        };

        let texture11 = match textures.len() > 11 {
            true => textures[11].clone(),
            false => texture0.clone(),
        };

        let texture12 = match textures.len() > 12 {
            true => textures[12].clone(),
            false => texture0.clone(),
        };

        let texture13 = match textures.len() > 13 {
            true => textures[13].clone(),
            false => texture0.clone(),
        };

        let texture14 = match textures.len() > 14 {
            true => textures[14].clone(),
            false => texture0.clone(),
        };

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view_sampler_array(0, 0, [
                (texture0 as _, self.linear_sampler.clone()),
                (texture1 as _, self.linear_sampler.clone()),
                (texture2 as _, self.linear_sampler.clone()),
                (texture3 as _, self.linear_sampler.clone()),
                (texture4 as _, self.linear_sampler.clone()),
                (texture5 as _, self.linear_sampler.clone()),
                (texture6 as _, self.linear_sampler.clone()),
                (texture7 as _, self.linear_sampler.clone()),
                (texture8 as _, self.linear_sampler.clone()),
                (texture9 as _, self.linear_sampler.clone()),
                (texture10 as _, self.linear_sampler.clone()),
                (texture11 as _, self.linear_sampler.clone()),
                (texture12 as _, self.linear_sampler.clone()),
                (texture13 as _, self.linear_sampler.clone()),
                (texture14 as _, self.linear_sampler.clone()),
            ])
        ]).unwrap(); 

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        let size = 4096.0;
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [size; 2],
            depth_range: 0.0..1.0,
        };

        render_target.state.get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 1, set)
            .set_viewport(0, [viewport])
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(vertex_count as u32, 1, 0, 0).unwrap();
    }
}
