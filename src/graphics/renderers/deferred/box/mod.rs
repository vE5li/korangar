mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/box/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/box/fragment_shader.glsl"
    }
}

use std::iter;
use std::ops::Mul;
use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use vulkano::buffer::{BufferAccess, BufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::image::ImageViewAbstract;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint, StateMode};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode};
use vulkano::shader::ShaderModule;

use self::fragment_shader::SpecializationConstants;
use self::vertex_shader::ty::{Constants, Matrices};
use crate::graphics::*;
use crate::world::BoundingBox;

pub struct BoxRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ModelVertexBuffer,
    index_buffer: Arc<CpuAccessibleBuffer<[u16]>>,
    matrices_buffer: CpuBufferPool<Matrices>,
}

impl BoxRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let vertices = vec![
            ModelVertex::new(
                Vector3::new(-1.0, -1.0, -1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 0.0),
                0,
            ), // bottom left front
            ModelVertex::new(
                Vector3::new(-1.0, 1.0, -1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 1.0),
                0,
            ), // top left front
            ModelVertex::new(
                Vector3::new(1.0, -1.0, -1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 0.0),
                0,
            ), // bottom right front
            ModelVertex::new(
                Vector3::new(1.0, 1.0, -1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 1.0),
                0,
            ), // top right front
            ModelVertex::new(
                Vector3::new(-1.0, -1.0, 1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 0.0),
                0,
            ), // bottom left back
            ModelVertex::new(
                Vector3::new(-1.0, 1.0, 1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(1.0, 1.0),
                0,
            ), // top left back
            ModelVertex::new(
                Vector3::new(1.0, -1.0, 1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 0.0),
                0,
            ), // bottom right back
            ModelVertex::new(
                Vector3::new(1.0, 1.0, 1.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector2::new(0.0, 1.0),
                0,
            ), // top right back
        ];

        let indices = vec![
            0, 1, 2, 3, 4, 5, 6, 7, // sides
            1, 3, 3, 7, 7, 5, 5, 1, // top
            0, 2, 2, 6, 6, 4, 4, 0, // bottom
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        let index_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, indices.into_iter()).unwrap();
        let matrices_buffer = CpuBufferPool::new(device, BufferUsage::all());

        Self {
            pipeline,
            vertex_shader,
            fragment_shader,
            vertex_buffer,
            index_buffer,
            matrices_buffer,
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
            .input_assembly_state(
                InputAssemblyState::new().topology(vulkano::pipeline::graphics::input_assembly::PrimitiveTopology::LineList),
            )
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
            view_projection: (projection_matrix * view_matrix).into(),
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());
        let set = PersistentDescriptorSet::new(descriptor_layout, [WriteDescriptorSet::buffer(0, matrices_subbuffer)]).unwrap();

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, 0, set)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .bind_index_buffer(self.index_buffer.clone());
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        transform: &Transform,
        bounding_box: &BoundingBox,
        color: Color,
    ) {

        let layout = self.pipeline.layout().clone();
        let size = bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(Vector3::new(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        let world_matrix = Matrix4::from_translation(position)
            * rotation_matrix
            * offset_matrix
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);

        let constants = Constants {
            world: world_matrix.into(),
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
        };

        render_target
            .state
            .get_builder()
            .push_constants(layout, 0, constants)
            .draw_indexed(24, 1, 0, 0, 0)
            .unwrap();
    }
}
