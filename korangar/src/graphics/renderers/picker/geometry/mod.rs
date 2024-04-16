vertex_shader!("src/graphics/renderers/picker/geometry/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/picker/geometry/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Matrix4;
use korangar_debug::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

use self::vertex_shader::{Constants, Matrices};
use crate::graphics::renderers::picker::PickerSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{allocate_descriptor_set, *};

pub struct GeometryRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl GeometryRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let linear_sampler = create_new_sampler(&device, SamplerType::Linear);
        let pipeline = Self::create_pipeline(device, subpass, viewport, &vertex_shader, &fragment_shader, false);

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            linear_sampler,
            pipeline,
        }
    }

    #[korangar_debug::profile]
    pub fn recreate_pipeline(&mut self, device: Arc<Device>, subpass: Subpass, viewport: Viewport, wireframe: bool) {
        self.pipeline = Self::create_pipeline(device, subpass, viewport, &self.vertex_shader, &self.fragment_shader, wireframe);
    }

    fn create_pipeline(
        device: Arc<Device>,
        subpass: Subpass,
        viewport: Viewport,
        vertex_shader: &EntryPoint,
        fragment_shader: &EntryPoint,
        _wireframe: bool,
    ) -> Arc<GraphicsPipeline> {
        PipelineBuilder::<_, { PickerRenderer::subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<ModelVertex>(vertex_shader)
            .fixed_viewport(viewport)
            .simple_depth_test()
            .build(device, subpass)
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_target: &mut <PickerRenderer as Renderer>::Target, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
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

    #[profile("render geometry")]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
    ) {
        if render_target.bind_subrenderer(PickerSubrenderer::Geometry) {
            self.bind_pipeline(render_target, camera);
        }

        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 15;

        let texture_count = textures.len();
        let mut samplers: Vec<(Arc<ImageView>, Arc<Sampler>)> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| (texture.clone() as _, self.linear_sampler.clone()))
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            samplers.push((textures[0].clone() as _, self.linear_sampler.clone()));
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 1, [
            WriteDescriptorSet::image_view_sampler_array(0, 0, samplers),
        ]);

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), set_id, set)
            .unwrap()
            .push_constants(layout, 0, constants)
            .unwrap()
            .bind_vertex_buffers(0, vertex_buffer)
            .unwrap()
            .draw(vertex_count as u32, 1, 0, 0)
            .unwrap();
    }
}
