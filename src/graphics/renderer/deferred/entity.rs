mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/entity_vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/entity_fragment_shader.glsl"
    }
}

use std::sync::Arc;
use std::iter;

use cgmath::{ Vector3, Vector2 };

use vulkano::device::Device;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::render_pass::Subpass;
use vulkano::sampler::Sampler;
use vulkano::buffer::BufferUsage;

use graphics::*;

use self::vertex_shader::Shader as VertexShader;
use self::fragment_shader::Shader as FragmentShader;
use self::vertex_shader::ty::Constants;
use self::vertex_shader::ty::Matrices;

pub struct EntityRenderer {
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: VertexShader,
    fragment_shader: FragmentShader,
    vertex_buffer: ModelVertexBuffer,
    matrices_buffer: CpuBufferPool<Matrices>,
    nearest_sampler: Arc<Sampler>,
}

impl EntityRenderer {

    pub fn new(device: Arc<Device>, subpass: Subpass, viewport: Viewport) -> Self {

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(device.clone(), subpass, viewport, &vertex_shader, &fragment_shader);

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

        let nearest_sampler = create_sampler!(device, Nearest, MirroredRepeat);

        return Self { pipeline, vertex_shader, fragment_shader, vertex_buffer, matrices_buffer, nearest_sampler };
    }

    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader);
    }

    fn create_pipeline(device: Arc<Device>, subpass: Subpass, viewport: Viewport, vertex_shader: &VertexShader, fragment_shader: &FragmentShader) -> Arc<GraphicsPipeline> {

        let pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<ModelVertex>()
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

    //pub fn start(&self, camera: &dyn Camera, builder: &mut CommandBuilder) {
    //}

    pub fn render(&self, camera: &dyn Camera, builder: &mut CommandBuilder, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>) {

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.descriptor_set_layouts().get(0).unwrap().clone();

        let mut set_builder = PersistentDescriptorSet::start(descriptor_layout);

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view: view_matrix.into(),
            projection: projection_matrix.into(),
        };
        let matrices_subbuffer = Arc::new(self.matrices_buffer.next(matrices).unwrap());

        set_builder
            .add_buffer(matrices_subbuffer).unwrap()
            .add_sampled_image(texture, self.nearest_sampler.clone()).unwrap();

        let set = Arc::new(set_builder.build().unwrap());
        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);

        let constants = Constants {
            world: world_matrix.into(),
            texture_position: [texture_position.x, texture_position.y],
            texture_size: [texture_size.x, texture_size.y],
        };

        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 0, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(6, 1, 0, 0).unwrap();
    }
}
