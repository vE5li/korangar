vertex_shader!("src/graphics/renderers/picker/tile/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/picker/tile/fragment_shader.glsl");

use std::sync::Arc;

use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::Matrices;
use super::PickerSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;

pub struct TileRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    pipeline: Arc<GraphicsPipeline>,
}

impl TileRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
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
        PipelineBuilder::<_, { PickerRenderer::subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<TileVertex>(vertex_shader)
            .fixed_viewport(viewport)
            .simple_depth_test()
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <PickerRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render tiles")]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[TileVertex]>,
    ) {
        if render_target.bind_subrenderer(PickerSubrenderer::Tile) {
            self.bind_pipeline(render_target);
        }

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
        });

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [WriteDescriptorSet::buffer(
            0, buffer,
        )]);

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<TileVertex>();

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, set_id, set)
            .unwrap()
            .bind_vertex_buffers(0, vertex_buffer)
            .unwrap()
            .draw(vertex_count as u32, 1, 0, 0)
            .unwrap();
    }
}
