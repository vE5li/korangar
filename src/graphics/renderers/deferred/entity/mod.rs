// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/entity/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/entity/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::image::ImageAccess;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode};
use vulkano::shader::ShaderModule;

use self::vertex_shader::ty::{Constants, Matrices};
use crate::graphics::*;

pub struct EntityRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ModelVertexBuffer,
    matrices_buffer: CpuBufferPool<Matrices>,
    nearest_sampler: Arc<Sampler>,
}

impl EntityRenderer {
    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let vertices = vec![
            ModelVertex::new(
                Vector3::new(-1.0, -2.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 0.0),
                0,
                0.0,
            ),
            ModelVertex::new(
                Vector3::new(-1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 1.0),
                0,
                0.0,
            ),
            ModelVertex::new(
                Vector3::new(1.0, -2.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 0.0),
                0,
                0.0,
            ),
            ModelVertex::new(
                Vector3::new(1.0, -2.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 0.0),
                0,
                0.0,
            ),
            ModelVertex::new(
                Vector3::new(-1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 1.0),
                0,
                0.0,
            ),
            ModelVertex::new(
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 1.0),
                0,
                0.0,
            ),
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        let nearest_sampler = Sampler::start(device)
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            .build()
            .unwrap();

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            vertex_buffer,
            matrices_buffer,
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

    pub fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());
        let set = PersistentDescriptorSet::new(descriptor_layout, [WriteDescriptorSet::buffer(0, matrices_subbuffer)]).unwrap();

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, 0, set)
            .bind_vertex_buffers(0, self.vertex_buffer.clone());
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        texture: Texture,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
    ) {
        let image_dimensions = Vector2::<u32>::from(texture.image().dimensions().width_height());
        let size = Vector2::new(
            image_dimensions.x as f32 * scale.x / 10.0,
            image_dimensions.y as f32 * scale.y / 10.0,
        );

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(1).unwrap().clone();

        let set = PersistentDescriptorSet::new(descriptor_layout, [WriteDescriptorSet::image_view_sampler(
            0,
            texture,
            self.nearest_sampler.clone(),
        )])
        .unwrap();

        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);

        let constants = Constants {
            world: world_matrix.into(),
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
            mirror: mirror as u32,
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 1, set)
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
