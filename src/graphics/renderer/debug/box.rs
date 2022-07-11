mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/box_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/box_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use std::ops::Mul;

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use vulkano::buffer::BufferUsage;

use crate::types::maths::*;
use crate::graphics::*;
use crate::types::map::model::BoundingBox;

use self::vertex_shader::ty::Constants;
use self::vertex_shader::ty::Matrices;

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
            ModelVertex::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 0.0), 0), // bottom left front
            ModelVertex::new(Vector3::new(-1.0, 1.0, -1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 1.0), 0), // top left front
            ModelVertex::new(Vector3::new(1.0, -1.0, -1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 0.0), 0), // bottom right front
            ModelVertex::new(Vector3::new(1.0, 1.0, -1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 1.0), 0), // top right front
            ModelVertex::new(Vector3::new(-1.0, -1.0, 1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 0.0), 0), // bottom left back
            ModelVertex::new(Vector3::new(-1.0, 1.0, 1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 1.0), 0), // top left back
            ModelVertex::new(Vector3::new(1.0, -1.0, 1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 0.0), 0), // bottom right back
            ModelVertex::new(Vector3::new(1.0, 1.0, 1.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 1.0), 0), // top right back
        ];

        let indices = vec![
            0, 1, 2, 3, 4, 5, 6, 7, // sides
            1, 3, 3, 7, 7, 5, 5, 1, // top
            0, 2, 2, 6, 6, 4, 4, 0, // bottom
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        let index_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, indices.into_iter()).unwrap();
        let matrices_buffer = CpuBufferPool::new(device, BufferUsage::all());

        Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, index_buffer, matrices_buffer }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new().topology(vulkano::pipeline::graphics::input_assembly::PrimitiveTopology::LineList))
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(&self, builder: &mut CommandBuilder, camera: &dyn Camera, transform: &Transform, bounding_box: &BoundingBox) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        };
        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::buffer(0, matrices_subbuffer),
        ]).unwrap();

        let size = bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(vector3!(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        let world_matrix = Matrix4::from_translation(position)
            * rotation_matrix
            * offset_matrix
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);

        let constants = Constants {
            world: world_matrix.into(),
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .bind_index_buffer(self.index_buffer.clone())
            .draw_indexed(24, 1, 0, 0, 0).unwrap();
    }
}
