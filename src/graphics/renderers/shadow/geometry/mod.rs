vertex_shader!("src/graphics/renderers/shadow/geometry/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/shadow/geometry/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Matrix4;
use procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::{Constants, Matrices};
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::renderers::shadow::ShadowSubrenderer;
use crate::graphics::*;

pub struct GeometryRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    matrices_buffer: MatrixAllocator<Matrices>,
    nearest_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl GeometryRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let pipeline = Self::create_pipeline(device, subpass, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            matrices_buffer,
            nearest_sampler,
            pipeline,
        }
    }

    fn create_pipeline(
        device: Arc<Device>,
        subpass: Subpass,
        vertex_shader: &EntryPoint,
        fragment_shader: &EntryPoint,
    ) -> Arc<GraphicsPipeline> {
        PipelineBuilder::<_, { ShadowRenderer::subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<ModelVertex>(vertex_shader)
            .simple_depth_test()
            .build(device, subpass)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <ShadowRenderer as Renderer>::Target, camera: &dyn Camera, time: f32) {
        #[cfg(feature = "debug")]
        let measurement = start_measurement("update matrices buffer");

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
        });

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("create persistent descriptor set");

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [WriteDescriptorSet::buffer(
            0, buffer,
        )]);

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("create viewport");

        let dimensions = render_target.image.image().extent().map(|component| component as f32);

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [dimensions[0], dimensions[1]],
            depth_range: 0.0..=1.0,
        };

        let builder = render_target.state.get_builder();

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("bind pipeline");

        builder.bind_pipeline_graphics(self.pipeline.clone()).unwrap();

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("set viewport");

        builder.set_viewport(0, std::iter::once(viewport).collect()).unwrap();

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("bind descriptor set");

        builder
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, set_id, set)
            .unwrap();

        #[cfg(feature = "debug")]
        measurement.stop();
    }

    #[profile("geometry renderer")]
    pub fn render(
        &self,
        render_target: &mut <ShadowRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
        time: f32,
    ) {
        if render_target.bind_subrenderer(ShadowSubrenderer::Geometry) {
            self.bind_pipeline(render_target, camera, time)
        }

        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 30;

        #[cfg(feature = "debug")]
        let measurement = start_measurement("create samplers");

        let texture_count = textures.len();
        let mut samplers: Vec<(Arc<ImageView>, Arc<Sampler>)> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| (texture.clone() as _, self.nearest_sampler.clone()))
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            samplers.push((textures[0].clone() as _, self.nearest_sampler.clone()));
        }

        #[cfg(feature = "debug")]
        measurement.stop();

        #[cfg(feature = "debug")]
        let measurement = start_measurement("create persistent descriptor set");

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 1, [
            WriteDescriptorSet::image_view_sampler_array(0, 0, samplers),
        ]);

        #[cfg(feature = "debug")]
        measurement.stop();

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        #[cfg(feature = "debug")]
        let measurement = start_measurement("append commands");

        let builder = render_target.state.get_builder();

        #[cfg(feature = "debug")]
        let inner_measurement = start_measurement("append commands");

        builder
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap();

        #[cfg(feature = "debug")]
        inner_measurement.stop();

        #[cfg(feature = "debug")]
        let inner_measurement = start_measurement("push constants");

        builder.push_constants(layout, 0, constants).unwrap();

        #[cfg(feature = "debug")]
        inner_measurement.stop();

        #[cfg(feature = "debug")]
        let inner_measurement = start_measurement("bind vertex buffer");

        builder.bind_vertex_buffers(0, vertex_buffer).unwrap();

        #[cfg(feature = "debug")]
        inner_measurement.stop();

        #[cfg(feature = "debug")]
        let inner_measurement = start_measurement("draw call");

        builder.draw(vertex_count as u32, 1, 0, 0).unwrap();

        #[cfg(feature = "debug")]
        inner_measurement.stop();

        #[cfg(feature = "debug")]
        measurement.stop();
    }
}
