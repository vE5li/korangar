mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/point/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/point/fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;
use vulkano::device::Device;
use vulkano::buffer::BufferUsage;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;
use cgmath::Vector3;

use crate::graphics::*;

use self::fragment_shader::ty::Constants;
use self::fragment_shader::ty::Matrices;

pub struct PointLightRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices>,
}

impl PointLightRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        Self { pipeline, vertex_shader, fragment_shader, matrices_buffer }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ScreenVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(1).blend(LIGHT_ATTACHMENT_BLEND))
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ScreenVertexBuffer) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let screen_to_world_matrix = camera.get_screen_to_world_matrix();
        let matrices = Matrices {
            screen_to_world: screen_to_world_matrix.into(),
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());
        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::image_view(0, render_target.diffuse_image.clone()),
            WriteDescriptorSet::image_view(1, render_target.normal_image.clone()),
            WriteDescriptorSet::image_view(2, render_target.depth_image.clone()),
            WriteDescriptorSet::buffer(3, matrices_subbuffer),

        ]).unwrap();

        render_target.state.get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .bind_vertex_buffers(0, vertex_buffer);
    }

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, position: Vector3<f32>, color: Color, range: f32) {

        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, 10.0 * (range / 0.05).ln());

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > range {
            return;
        }

        let layout = self.pipeline.layout().clone();

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

        let constants = Constants {
            screen_position: [screen_position.x, screen_position.y],
            screen_size: [screen_size.x, screen_size.y],
            position: [position.x, position.y, position.z],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
            range,
            _dummy0: Default::default(),
        };

        render_target.state.get_builder()
            .push_constants(layout, 0, constants)
            .draw(6, 1, 0, 0).unwrap();
    }
}
