vertex_shader!("src/graphics/renderers/deferred/box/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/box/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Vector3;
use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::input_assembly::PrimitiveTopology;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint, StateMode};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;
use crate::world::{BoundingBox, Model};

pub struct BoxRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    vertex_buffer: Subbuffer<[WaterVertex]>,
    index_buffer: Subbuffer<[u16]>,
    matrices_buffer: MatrixAllocator<Matrices>,
    pipeline: Arc<GraphicsPipeline>,
}

impl BoxRenderer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        buffer_allocator: &mut BufferAllocator,
        subpass: Subpass,
        viewport: Viewport,
    ) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);

        let vertex_buffer = buffer_allocator.allocate_vertex_buffer([
            WaterVertex::new(Vector3::new(-1.0, -1.0, -1.0)), // bottom left front
            WaterVertex::new(Vector3::new(-1.0, 1.0, -1.0)),  // top left front
            WaterVertex::new(Vector3::new(1.0, -1.0, -1.0)),  // bottom right front
            WaterVertex::new(Vector3::new(1.0, 1.0, -1.0)),   // top right front
            WaterVertex::new(Vector3::new(-1.0, -1.0, 1.0)),  // bottom left back
            WaterVertex::new(Vector3::new(-1.0, 1.0, 1.0)),   // top left back
            WaterVertex::new(Vector3::new(1.0, -1.0, 1.0)),   // bottom right back
            WaterVertex::new(Vector3::new(1.0, 1.0, 1.0)),    // top right back
        ]);

        let index_buffer = buffer_allocator.allocate_index_buffer([
            0, 1, 2, 3, 4, 5, 6, 7, // sides
            1, 3, 3, 7, 7, 5, 5, 1, // top
            0, 2, 2, 6, 6, 4, 4, 0, // bottom
        ]);

        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            vertex_buffer,
            index_buffer,
            matrices_buffer,
            pipeline,
        }
    }

    #[profile]
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
        let rasterization_state = RasterizationState {
            line_width: StateMode::Fixed(3.0),
            ..Default::default()
        };

        PipelineBuilder::<_, { DeferredRenderer::lighting_subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<WaterVertex>(vertex_shader)
            .topology(PrimitiveTopology::LineList)
            .fixed_viewport(viewport)
            .rasterization_state(rasterization_state)
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
        });

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [WriteDescriptorSet::buffer(
            0, buffer,
        )]);

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, set_id, set)
            .unwrap()
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .unwrap()
            .bind_index_buffer(self.index_buffer.clone())
            .unwrap();
    }

    #[profile("render bounding box")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        transform: &Transform,
        bounding_box: &BoundingBox,
        color: Color,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::BoundingBox) {
            self.bind_pipeline(render_target, camera);
        }

        let layout = self.pipeline.layout().clone();
        let world_matrix = Model::bounding_box_matrix(bounding_box, transform);

        let constants = Constants {
            world: world_matrix.into(),
            color: color.into(),
        };

        render_target
            .state
            .get_builder()
            .push_constants(layout, 0, constants)
            .unwrap()
            .draw_indexed(24, 1, 0, 0, 0)
            .unwrap();
    }
}
