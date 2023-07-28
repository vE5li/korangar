// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/graphics/renderers/deferred/geometry/vertex_shader.glsl"
    }
}

// TODO: remove once no longer needed
#[allow(clippy::needless_question_mark)]
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/graphics/renderers/deferred/geometry/fragment_shader.glsl"
    }
}

use std::iter;
use std::sync::Arc;

use cgmath::Matrix4;
use procedural::profile;
use vulkano::buffer::{BufferAccess, BufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned};
use vulkano::image::ImageViewAbstract;
use vulkano::memory::allocator::MemoryUsage;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{CullMode, PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint, StateMode};
use vulkano::render_pass::Subpass;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;

use self::fragment_shader::SpecializationConstants;
use self::vertex_shader::ty::{Constants, Matrices};
use crate::graphics::renderers::deferred::DeferredSubrenderer;
use crate::graphics::*;

unsafe impl bytemuck::Zeroable for Constants {}
unsafe impl bytemuck::Pod for Constants {}

unsafe impl bytemuck::Zeroable for Matrices {}
unsafe impl bytemuck::Pod for Matrices {}

pub struct GeometryRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    matrices_buffer: CpuBufferPool<Matrices, MemoryAllocator>,
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
}

impl GeometryRenderer {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, subpass: Subpass, viewport: Viewport) -> Self {
        let device = memory_allocator.device().clone();
        let vertex_shader = vertex_shader::load(device.clone()).unwrap();
        let fragment_shader = fragment_shader::load(device.clone()).unwrap();
        let pipeline = Self::create_pipeline(
            device.clone(),
            subpass,
            viewport,
            &vertex_shader,
            &fragment_shader,
            #[cfg(feature = "debug")]
            false,
        );

        let matrices_buffer = CpuBufferPool::new(
            memory_allocator.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            MemoryUsage::Upload,
        );

        let nearest_sampler = Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            ..Default::default()
        })
        .unwrap();

        let linear_sampler = Sampler::new(device, SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            anisotropy: Some(4.0),
            mip_lod_bias: 1.0,
            ..Default::default()
        })
        .unwrap();

        Self {
            memory_allocator,
            pipeline,
            vertex_shader,
            fragment_shader,
            matrices_buffer,
            nearest_sampler,
            linear_sampler,
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
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
        #[cfg(feature = "debug")] wireframe: bool,
    ) -> Arc<GraphicsPipeline> {
        #[cfg(feature = "debug")]
        let polygon_mode = match wireframe {
            true => PolygonMode::Line,
            false => PolygonMode::Fill,
        };

        #[cfg(feature = "debug")]
        let specialization_constants = match wireframe {
            true => SpecializationConstants { additional_color: 1.0 },
            false => SpecializationConstants { additional_color: 0.0 },
        };

        #[cfg(not(feature = "debug"))]
        let (polygon_mode, specialization_constants) = (PolygonMode::Fill, SpecializationConstants { additional_color: 0.0 });

        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ModelVertex>())
            .vertex_shader(vertex_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant(iter::once(viewport)))
            .fragment_shader(fragment_shader.entry_point("main").unwrap(), specialization_constants)
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState {
                cull_mode: StateMode::Fixed(CullMode::Back),
                polygon_mode,
                ..Default::default()
            })
            .multisample_state(MultisampleState {
                rasterization_samples: vulkano::image::SampleCount::Sample4,
                ..Default::default()
            })
            .render_pass(subpass)
            .build(device)
            .unwrap()
    }

    #[profile]
    fn bind_pipeline(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, camera: &dyn Camera, time: f32) {
        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let matrices = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
        };

        let matrices_subbuffer = Arc::new(self.matrices_buffer.from_data(matrices).unwrap());
        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [WriteDescriptorSet::buffer(
            0,
            matrices_subbuffer,
        )])
        .unwrap();

        render_target
            .state
            .get_builder()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout, 0, set);
    }

    #[profile("render geometry")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: ModelVertexBuffer,
        textures: &[Texture],
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

        let layout = self.pipeline.layout().clone();
        let descriptor_layout = layout.set_layouts().get(1).unwrap().clone();

        let texture_count = textures.len();
        let mut textures: Vec<Arc<dyn ImageViewAbstract>> = textures
            .iter()
            .take(TEXTURE_COUNT.min(texture_count))
            .map(|texture| texture.clone() as _)
            .collect();

        for _ in 0..TEXTURE_COUNT.saturating_sub(texture_count) {
            textures.push(textures[0].clone());
        }

        let set = PersistentDescriptorSet::new(&*self.memory_allocator, descriptor_layout, [
            WriteDescriptorSet::sampler(0, self.nearest_sampler.clone()),
            WriteDescriptorSet::sampler(1, self.linear_sampler.clone()),
            WriteDescriptorSet::image_view_array(2, 0, textures),
        ])
        .unwrap();

        let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<ModelVertex>();
        let constants = Constants {
            world: world_matrix.into(),
        };

        render_target
            .state
            .get_builder()
            .bind_descriptor_sets(PipelineBindPoint::Graphics, layout.clone(), 1, set)
            .push_constants(layout, 0, constants)
            .bind_vertex_buffers(0, vertex_buffer)
            .draw(vertex_count as u32, 1, 0, 0)
            .unwrap();
    }
}
