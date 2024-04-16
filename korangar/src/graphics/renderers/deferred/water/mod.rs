vertex_shader!("src/graphics/renderers/deferred/water/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/water/fragment_shader.glsl");

use std::sync::Arc;

use korangar_debug::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::depth_stencil::{CompareOp, DepthState, DepthStencilState};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint, StateMode};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::*;

pub struct WaterRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    pipeline: Arc<GraphicsPipeline>,
}

impl WaterRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
        }
    }

    #[korangar_debug::profile]
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
        let depth_stencil_state = DepthStencilState {
            depth: Some(DepthState {
                enable_dynamic: false,
                compare_op: StateMode::Fixed(CompareOp::Less),
                write_enable: StateMode::Fixed(false),
            }),
            ..Default::default()
        };

        PipelineBuilder::<_, { DeferredRenderer::deferred_subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<WaterVertex>(vertex_shader)
            .fixed_viewport(viewport)
            .multisample(SampleCount::Sample4)
            .depth_stencil_state(depth_stencil_state)
            .build(device, subpass)
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render water")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[WaterVertex]>,
        day_timer: f32,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Water) {
            self.bind_pipeline(render_target);
        }

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        });

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [WriteDescriptorSet::buffer(
            0, buffer,
        )]);

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<WaterVertex>();
        let constants = Constants { wave_offset: day_timer };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap()
            .push_constants(layout, 0, constants)
            .unwrap()
            .bind_vertex_buffers(0, vertex_buffer)
            .unwrap()
            .draw(vertex_count as u32, 1, 0, 0)
            .unwrap();
    }
}
