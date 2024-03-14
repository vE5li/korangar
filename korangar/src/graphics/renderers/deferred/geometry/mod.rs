vertex_shader!("src/graphics/renderers/deferred/geometry/vertex_shader.glsl");
fragment_shader!("src/graphics/renderers/deferred/geometry/fragment_shader.glsl");

use std::sync::Arc;

use cgmath::Matrix4;
use korangar_procedural::profile;
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::sampler::Sampler;
use vulkano::image::SampleCount;
use vulkano::pipeline::graphics::rasterization::{CullMode, PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint, StateMode};
use vulkano::render_pass::Subpass;
use vulkano::shader::EntryPoint;

//use self::fragment_shader::SpecializationConstants;
use self::vertex_shader::{Constants, Matrices};
use crate::graphics::renderers::deferred::DeferredSubrenderer;
use crate::graphics::renderers::pipeline::PipelineBuilder;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{allocate_descriptor_set, *};

pub struct GeometryRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    vertex_shader: EntryPoint,
    fragment_shader: EntryPoint,
    matrices_buffer: MatrixAllocator<Matrices>,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
    pipeline: Arc<GraphicsPipeline>,
}

impl GeometryRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::entry_point(&device);
        let fragment_shader = fragment_shader::entry_point(&device);
        let matrices_buffer = MatrixAllocator::new(&memory_allocator);
        let nearest_sampler = create_new_sampler(&device, SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, SamplerType::LinearAnisotropic(4.0));
        let pipeline = Self::create_pipeline(
            device,
            subpass,
            viewport,
            &vertex_shader,
            &fragment_shader,
            #[cfg(feature = "debug")]
            false,
        );

        Self {
            memory_allocator,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            nearest_sampler,
            linear_sampler,
            pipeline,
        }
    }

    #[profile]
    pub fn recreate_pipeline(
        &mut self,
        device: Arc<Device>,
        subpass: Subpass,
        viewport: Viewport,
        #[cfg(feature = "debug")] wireframe: bool,
    ) {
        self.pipeline = Self::create_pipeline(
            device,
            subpass,
            viewport,
            &self.vertex_shader,
            &self.fragment_shader,
            #[cfg(feature = "debug")]
            wireframe,
        );
    }

    fn create_pipeline(
        device: Arc<Device>,
        subpass: Subpass,
        viewport: Viewport,
        vertex_shader: &EntryPoint,
        fragment_shader: &EntryPoint,
        #[cfg(feature = "debug")] wireframe: bool,
    ) -> Arc<GraphicsPipeline> {
        #[cfg(feature = "debug")]
        let (polygon_mode, additional_color) = match wireframe {
            true => (PolygonMode::Line, 1.0f32),
            false => (PolygonMode::Fill, 0.0f32),
        };

        #[cfg(not(feature = "debug"))]
        let (polygon_mode, additional_color) = (PolygonMode::Fill, 0.0f32);

        let rasterization_state = RasterizationState {
            cull_mode: StateMode::Fixed(CullMode::Back),
            polygon_mode,
            ..Default::default()
        };

        let vertex_shader_constants = [];
        let fragment_shader_contsants = [(0, additional_color.into())];
        let specialization_constants = [vertex_shader_constants.as_slice(), fragment_shader_contsants.as_slice()];

        PipelineBuilder::<_, { DeferredRenderer::deferred_subpass() }>::new([vertex_shader, fragment_shader])
            .vertex_input_state::<ModelVertex>(vertex_shader)
            .fixed_viewport(viewport)
            .rasterization_state(rasterization_state)
            .multisample(SampleCount::Sample4)
            .simple_depth_test()
            .build_with_specialization(device, subpass, specialization_constants)
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, time: f32) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let buffer = self.matrices_buffer.allocate(Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
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
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
        time: f32,
    ) {
        if render_target.bind_subrenderer(DeferredSubrenderer::Geometry) {
            self.bind_pipeline(render_target, camera, time);
        }

        if textures.is_empty() {
            return;
        }

        const TEXTURE_COUNT: usize = 30;

        let texture_count = textures.len();
        let mut textures: Vec<Arc<ImageView>> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| texture.clone() as _)
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            textures.push(textures[0].clone());
        }

        let (layout, set, set_id) = allocate_descriptor_set(&self.pipeline, &self.memory_allocator, 1, [
            WriteDescriptorSet::sampler(0, self.nearest_sampler.clone()),
            WriteDescriptorSet::sampler(1, self.linear_sampler.clone()),
            WriteDescriptorSet::image_view_array(2, 0, textures),
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
