// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/indicator/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/indicator/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::Vector3;
use procedural::profile;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Sampler, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::fragment_shader::ty::Constants;
use super::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

pub struct IndicatorRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    nearest_sampler: Arc<Sampler>,
}

impl IndicatorRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);
        let nearest_sampler = Sampler::new(device, SamplerCreateInfo::simple_repeat_linear_no_mipmap()).unwrap();

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            nearest_sampler,
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
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .multisample_state(MultisampleState {
                rasterization_samples: vulkano::image::SampleCount::Sample4,
                ..Default::default()
            })
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target) {
        render_target.state.get_builder().bind_pipeline_graphics(self.pipeline.clone());
    }

    #[profile]
    pub fn render_ground_indicator(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        texture: Texture,
        color: Color,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Indicator) {
            self.bind_pipeline(render_target);
        }

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::image_view_sampler(0, texture, self.nearest_sampler.clone()),
        ])
        .unwrap();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let constants = Constants {
            view_projection: (projection_matrix * view_matrix).into(),
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
            upper_left: upper_left.into(),
            upper_right: upper_right.into(),
            lower_left: lower_left.into(),
            lower_right: lower_right.into(),
            _dummy0: [0; 4],
            _dummy1: [0; 4],
            _dummy2: [0; 4],
            _dummy3: [0; 4],
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0)
            .unwrap();
    }
}
