mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/geometry/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/geometry/fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use vulkano::device::Device;

use vulkano::image::ImageViewAbstract;
use vulkano::pipeline::graphics::rasterization::{ RasterizationState, CullMode, PolygonMode };
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline, StateMode };
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

use self::fragment_shader::SpecializationConstants;

pub struct GeometryRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices>,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
}

impl GeometryRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader, false);
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        let nearest_sampler = Sampler::start(device.clone())
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::ClampToEdge)
            .build()
            .unwrap();

        let linear_sampler = Sampler::start(device)
            .filter(Filter::Linear)
            .address_mode(SamplerAddressMode::ClampToEdge)
            .anisotropy(Some(4.0))
            .min_lod(1.0)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, matrices_buffer, nearest_sampler, linear_sampler }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport, wireframe: bool) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader, wireframe);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule, wireframe: bool) -> Arc<GraphicsPipeline> {

        let polygon_mode = match wireframe {
            true => PolygonMode::Line,
            false => PolygonMode::Fill,
        };

        let specialization_constants = match wireframe {
            true => SpecializationConstants { additional_color: 1.0 },
            false => SpecializationConstants { additional_color: 0.0 },
        };

        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), specialization_constants)
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState { cull_mode: StateMode::Fixed(CullMode::Back), polygon_mode, ..Default::default() })
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {

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

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>) {

        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 30;

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(1).unwrap().clone();

        let texture_count = textures.len();
        let mut textures: Vec<Arc<dyn ImageViewAbstract>> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| texture.clone() as _)
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            textures.push(textures[0].clone());
        }

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::sampler(0, self.nearest_sampler.clone()),
            WriteDescriptorSet::sampler(1, self.linear_sampler.clone()),
            WriteDescriptorSet::image_view_array(2, 0, textures)
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
