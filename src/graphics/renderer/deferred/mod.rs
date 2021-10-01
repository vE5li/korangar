mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/deferred_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/deferred_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::render_pass::Subpass;
use vulkano::sampler::Sampler;
use vulkano::buffer::{ BufferUsage, BufferAccess };

use graphics::*;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::vertex_shader::ty::Matrices;

pub struct DeferredRenderer {
    pipeline: Arc<GraphicsPipeline>,
    matrices_buffer: CpuBufferPool::<Matrices>,
    linear_sampler: Arc<Sampler>,
}

impl DeferredRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

        let matrices_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        let linear_sampler = create_sampler!(device, Linear, Repeat);

        return Self { pipeline, matrices_buffer, linear_sampler };
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &VertexShader, fragment_shader: &FragmentShader) -> Arc<GraphicsPipeline> {

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(viewport))
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .render_pass(subpass)
            .build(device)
            .unwrap();

        return Arc::new(pipeline);
    }

    pub fn render_geometry(&self, camera: &dyn Camera, builder: &mut CommandBuilder, vertex_buffer: VertexBuffer, textures: &Vec<Texture>, transform: &Transform) {

        // SUPER DIRTY, PLEASE FIX

        let texture0 = textures[0].clone();

        let texture1 = match textures.len() > 1 {
            true => textures[1].clone(),
            false => texture0.clone(),
        };

        let texture2 = match textures.len() > 2 {
            true => textures[2].clone(),
            false => texture0.clone(),
        };

        let texture3 = match textures.len() > 3 {
            true => textures[3].clone(),
            false => texture0.clone(),
        };

        let texture4 = match textures.len() > 4 {
            true => textures[4].clone(),
            false => texture0.clone(),
        };

        let texture5 = match textures.len() > 5 {
            true => textures[5].clone(),
            false => texture0.clone(),
        };

        let texture6 = match textures.len() > 6 {
            true => textures[6].clone(),
            false => texture0.clone(),
        };

        let texture7 = match textures.len() > 7 {
            true => textures[7].clone(),
            false => texture0.clone(),
        };

        let texture8 = match textures.len() > 8 {
            true => textures[8].clone(),
            false => texture0.clone(),
        };

        let texture9 = match textures.len() > 9 {
            true => textures[9].clone(),
            false => texture0.clone(),
        };

        let (rotation_matrix, world_matrix, view_matrix, projection_matrix) = camera.transform_matrices(transform);
        let matrices = Matrices {
            rotation: rotation_matrix.into(),
            world: world_matrix.into(),
            view: view_matrix.into(),
            projection: projection_matrix.into()
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let mut set_builder = PersistentDescriptorSet::start(descriptor_layout);

        set_builder
            .add_buffer(matrices_subbuffer).unwrap()
            .add_sampler(self.linear_sampler.clone()).unwrap()
            .enter_array().unwrap()
                .add_image(texture0).unwrap()
                .add_image(texture1).unwrap()
                .add_image(texture2).unwrap()
                .add_image(texture3).unwrap()
                .add_image(texture4).unwrap()
                .add_image(texture5).unwrap()
                .add_image(texture6).unwrap()
                .add_image(texture7).unwrap()
                .add_image(texture8).unwrap()
                .add_image(texture9).unwrap()
            .leave_array().unwrap();

        let set = Arc::new(set_builder.build().unwrap());
        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<Vertex>();

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(vertex_count as u32, 1, 0, 0).unwrap();
    }
}
