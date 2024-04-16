vertex_shader!("src/graphics/renderers/deferred/directional/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/directional/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::{Matrix4, Vector3};
use korangar_procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::padded::Padded;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::fragment_shader::{Constants, Matrices};
use super::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;

pub struct DirectionalLightRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl DirectionalLightRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            linear_sampler,
            pipeline,
        }
    }

    #[korangar_procedural::profile]
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
        PipelineBuilder::<_, { DeferredRenderer::lighting_subpass() }>::new([vertex_shader, fragment_shader])
            .fixed_viewport(viewport)
            .color_blend(LIGHT_ATTACHMENT_BLEND)
            .build(device, subpass)
    }

    #[korangar_procedural::profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap();
    }

    #[profile("render directional light")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        shadow_image: Arc<ImageView>,
        light_matrix: Matrix4<f32>,
        direction: Vector3<f32>,
        color: Color,
        intensity: f32,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::DirectionalLight) {
            self.bind_pipeline(render_target);
        }

        let buffer = self.matrices_buffer.allocate(Matrices {
            screen_to_world: camera.get_screen_to_world_matrix().into(),
            light: light_matrix.into(),
        });

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 0, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()),
            WriteDescriptorSet::image_view_sampler(3, shadow_image, self.linear_sampler.clone()),
            WriteDescriptorSet::buffer(4, buffer),
        ]);

        let constants = Constants {
            direction: Padded(direction.into()),
            color: [color.red * intensity, color.green * intensity, color.blue * intensity],
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
