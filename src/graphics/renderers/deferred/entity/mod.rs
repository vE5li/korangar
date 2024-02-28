vertex_shader!("src/graphics/renderers/deferred/entity/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/entity/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{allocate_descriptor_set, *};

pub struct EntityRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    linear_sampler: Arc<Sampler>,
}

impl EntityRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            linear_sampler,
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
        PipelineBuilder::<_, { DeferredRenderer::deferred_subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .multisample(SampleCount::Sample4)
            .simple_depth_test()
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
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
            .unwrap();
    }

    #[profile("render entity")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        texture: Arc<ImageView>,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Entity) {
            self.bind_pipeline(render_target, camera);
        }

        let image_dimensions = texture.image().extent();
        let size = Vector2::new(
            image_dimensions[0] as f32 * scale.x / 10.0,
            image_dimensions[1] as f32 * scale.y / 10.0,
        );

        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix);

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 1, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.linear_sampler.clone()),
        ]);

        let constants = Constants {
            world: world_matrix.into(),
            texture_position: texture_position.into(),
            texture_size: texture_size.into(),
            depth_offset,
            curvature,
            mirror: mirror as u32,
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap()
            .push_constants(layout, 0, constants)
            .unwrap()
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
