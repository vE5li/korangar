vertex_shader!("src/graphics/renderers/shadow/indicator/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/shadow/indicator/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Vector3;
use korangar_debug::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::padded::Padded;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::Constants;
use super::ShadowSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{allocate_descriptor_set, *};

pub struct IndicatorRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    nearest_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl IndicatorRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let pipeline = Self::create_pipeline(device, subpass, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
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
            .simple_depth_test()
            .build(device, subpass)
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_target: &mut <ShadowRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render ground indicator")]
    pub fn render_ground_indicator(
        &self,
        render_target: &mut <ShadowRenderer as Renderer>::Target,
        camera: &dyn Camera,
        texture: Arc<ImageView>,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) {
        if render_target.bind_subrenderer(ShadowSubrenderer::Indicator) {
            self.bind_pipeline(render_target);
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.nearest_sampler.clone()),
        ]);

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let constants = Constants {
            view_projection: (projection_matrix * view_matrix).into(),
            upper_left: Padded(upper_left.into()),
            upper_right: Padded(upper_right.into()),
            lower_left: Padded(lower_left.into()),
            lower_right: lower_right.into(),
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
