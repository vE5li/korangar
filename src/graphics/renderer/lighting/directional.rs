mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/directional_light_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/directional_light_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use cgmath::Vector3;

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::render_pass::Subpass;

use graphics::*;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::fragment_shader::ty::Constants as Constants;

pub struct DirectionalLightRenderer {
    pipeline: Arc<GraphicsPipeline>,
}

impl DirectionalLightRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader);

        return Self { pipeline };
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &VertexShader, fragment_shader: &FragmentShader) -> Arc<GraphicsPipeline> {

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<ScreenVertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(viewport))
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .blend_collective(LIGHT_ATTACHMENT_BLEND)
            .render_pass(subpass)
            .build(device)
            .unwrap();

        return Arc::new(pipeline);
    }

    pub fn render(&self, builder: &mut CommandBuilder, diffuse_buffer: ImageBuffer, normal_buffer: ImageBuffer, vertex_buffer: ScreenVertexBuffer, direction: Vector3<f32>, color: Color) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let mut set_builder = PersistentDescriptorSet::start(descriptor_layout);

        set_builder
            .add_image(diffuse_buffer).unwrap()
            .add_image(normal_buffer).unwrap();

        let set = Arc::new(set_builder.build().unwrap());

        let constants = Constants {
            direction: [direction.x, direction.y, direction.z],
            color: [color.red_f32(), color.green_f32(), color.blue_f32()],
            _dummy0: [0; 4],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(3, 1, 0, 0).unwrap();
    }
}
