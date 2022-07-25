mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/shadow/entity/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/shadow/entity/fragment_shader.glsl"
    }
}

use std::sync::Arc;

use cgmath::{ Vector3, Vector2 };

use vulkano::device::Device;

use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint, Pipeline };
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::{ Viewport, ViewportState };
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::shader::ShaderModule;
use vulkano::render_pass::Subpass;
use vulkano::sampler::{ Sampler, Filter, SamplerAddressMode };
use vulkano::buffer::BufferUsage;

use crate::graphics::*;

use self::vertex_shader::ty::Constants;
use self::vertex_shader::ty::Matrices;

pub struct EntityRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    vertex_buffer: ModelVertexBuffer,
    matrices_buffer: CpuBufferPool<Matrices>,
    nearest_sampler: Arc<Sampler>,
}

impl EntityRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass) -> Self {

        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, &vertex_shader, &fragment_shader);

        let vertices = vec![
            ModelVertex::new(Vector3::new(-1.0, -2.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 0.0), 0),
            ModelVertex::new(Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 1.0), 0),
            ModelVertex::new(Vector3::new(1.0, -2.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 0.0), 0),
            ModelVertex::new(Vector3::new(1.0, -2.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 0.0), 0),
            ModelVertex::new(Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(1.0, 1.0), 0),
            ModelVertex::new(Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector2::new(0.0, 1.0), 0),
        ];

        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());
 
        let nearest_sampler = Sampler::start(device)
            .filter(Filter::Nearest)
            .address_mode(SamplerAddressMode::MirroredRepeat)
            .build()
            .unwrap();

        Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, matrices_buffer, nearest_sampler }
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass) {
        self.pipeline = Self::create_pipeline(device, subpass, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    pub fn render(&self, render_target: &mut <ShadowRenderer as Renderer>::Target, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>)
    {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        };
        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());

        let set = PersistentDescriptorSet::new(descriptor_layout, [
            WriteDescriptorSet::buffer(0, matrices_subbuffer),
            WriteDescriptorSet::image_view_sampler(1, texture, self.nearest_sampler.clone()),
        ]).unwrap(); 

        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);

        let constants = Constants {
            world: world_matrix.into(),
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
        };

        //let size = (render_target.image.image().mem_size() / 4).sqrt(); // FIND A BETTER WAY TO GET THE 
                                                                        // SIZE OF A PIXEL
        let size = 4096.0;

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [size; 2],
            depth_range: 0.0..1.0,
        };

        render_target.state.get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .set_viewport(0, [viewport])
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }
}
