use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Matrix4;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat,
    VertexState,
};

use super::{Buffer, Camera, DirectionalShadowRenderer, DirectionalShadowSubRenderer, ModelVertex, Renderer, TextureGroup};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};

const SHADER: ShaderModuleDescriptor = include_wgsl!("geometry.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Matrices {
    view_projection: [[f32; 4]; 4],
    time: f32,
    padding: [u8; 12],
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    world: [[f32; 4]; 4],
}

pub struct GeometryRenderer {
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl GeometryRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "geometry",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let nearest_sampler = create_new_sampler(&device, "geometry nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: matrices_buffer.byte_capacity(),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: matrices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&nearest_sampler),
                },
            ],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, output_depth_format);

        Self {
            queue,
            matrices_buffer,
            bind_group,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[bind_group_layout, TextureGroup::bind_group_layout(device)],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("geometry"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass, camera: &dyn Camera, time: f32) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let uniform_data = Matrices {
            view_projection: (projection_matrix * view_matrix).into(),
            time,
            padding: Default::default(),
        };
        self.matrices_buffer.write_exact(&self.queue, &[uniform_data]);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("geometry renderer"))]
    pub fn render(
        &self,
        render_target: &mut <DirectionalShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) {
        if render_target.bind_sub_renderer(DirectionalShadowSubRenderer::Geometry) {
            self.bind_pipeline(render_pass, camera, time)
        }

        let push_constants = Constants {
            world: world_matrix.into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(1, &textures.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}