mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/picker/geometry/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/picker/geometry/fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use vulkano::device::Device;
use vulkano::image::ImageViewAbstract;
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
use cgmath::Matrix4;

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

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader, false);
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        let linear_sampler = Sampler::start(device)
            .filter(Filter::Linear)
            .address_mode(SamplerAddressMode::ClampToEdge)
            .min_lod(1.0)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, matrices_buffer, linear_sampler }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport, wireframe: bool) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader, wireframe);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule, wireframe: bool) -> Arc<GraphicsPipeline> {

        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn bind_pipeline(&self, render_target: &mut <PickerRenderer as Renderer>::Target, camera: &dyn Camera) {

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

    pub fn render(&self, render_target: &mut <PickerRenderer as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>) {
        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 15;

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(1).unwrap().clone();

        let texture_count = textures.len();
        let mut samplers: Vec<(Arc<dyn ImageViewAbstract>, Arc<Sampler>)> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| (texture.clone() as _, self.linear_sampler.clone()))
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            samplers.push((textures[0].clone() as _, self.linear_sampler.clone()));
        }

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view_sampler_array(0, 0, samplers)
        ]).unwrap(); 

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        render_target.state.get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 1, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(vertex_count as u32, 1, 0, 0).unwrap();
    }
}
