use std::num::NonZeroU64;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::{Buffer, Camera, DirectionalShadowRenderer, DirectionalShadowSubRenderer, ModelVertex, Renderer, TextureGroup};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::renderers::DrawIndirectArgs;
use crate::graphics::GeometryInstruction;

const INITIAL_INSTRUCTION_SIZE: usize = 512;
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
struct InstanceData {
    world: [[f32; 4]; 4],
}

pub struct GeometryRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    matrices_buffer: Buffer<Matrices>,
    instance_buffer: Buffer<InstanceData>,
    instance_index_vertex_buffer: Buffer<u32>,
    command_buffer: Buffer<DrawIndirectArgs>,
    instance_data_bind_group_layout: BindGroupLayout,
    uniform_bind_group: BindGroup,
    pipeline: RenderPipeline,
    instance_data: Vec<InstanceData>,
    draw_commands: Vec<DrawIndirectArgs>,
    instance_indices: Vec<u32>,
}

impl GeometryRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let matrices_buffer = Buffer::with_capacity(
            &device,
            "geometry uniform",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<Matrices>() as _,
        );
        let instance_buffer = Buffer::with_capacity(
            &device,
            "instance data",
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );
        let command_buffer = Buffer::with_capacity(
            &device,
            "indirect draw",
            BufferUsages::COPY_DST | BufferUsages::INDIRECT,
            (size_of::<DrawIndirectArgs>() * INITIAL_INSTRUCTION_SIZE) as _,
        );
        let nearest_sampler = create_new_sampler(&device, "geometry nearest", SamplerType::Nearest);
        let uniform_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry uniform"),
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
        let instance_data_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry instance data"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                },
                count: None,
            }],
        });
        // TODO: NHA This instance index vertex buffer is only needed until this issue is fixed for DX12: https://github.com/gfx-rs/wgpu/issues/2471
        let instance_index_vertex_buffer = Buffer::with_capacity(
            &device,
            "instance index vertex",
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            (size_of::<u32>() * INITIAL_INSTRUCTION_SIZE) as _,
        );
        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry uniform"),
            layout: &uniform_bind_group_layout,
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

        let pipeline = Self::create_pipeline(
            &device,
            &shader_module,
            &uniform_bind_group_layout,
            &instance_data_bind_group_layout,
            output_depth_format,
        );

        Self {
            device,
            queue,
            matrices_buffer,
            instance_buffer,
            instance_index_vertex_buffer,
            command_buffer,
            instance_data_bind_group_layout,
            uniform_bind_group,
            pipeline,
            instance_data: Vec::new(),
            draw_commands: Vec::new(),
            instance_indices: Vec::new(),
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        uniform_bind_group_layout: &BindGroupLayout,
        instance_data_bind_group_layout: &BindGroupLayout,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[
                uniform_bind_group_layout,
                instance_data_bind_group_layout,
                TextureGroup::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        });

        let instance_index_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<u32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 0,
                shader_location: 5,
            }],
        };

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("geometry"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout(), instance_index_buffer_layout],
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
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("geometry renderer"))]
    pub fn render(
        &mut self,
        render_target: &mut <DirectionalShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        instructions: &[GeometryInstruction],
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        time: f32,
    ) {
        if instructions.is_empty() {
            return;
        }

        if render_target.bound_sub_renderer(DirectionalShadowSubRenderer::Geometry) {
            self.bind_pipeline(render_pass, camera, time)
        }

        self.instance_data.clear();
        self.draw_commands.clear();
        self.instance_indices.clear();

        for (instance_index, instruction) in instructions.iter().enumerate() {
            self.instance_data.push(InstanceData {
                world: instruction.world_matrix.into(),
            });

            self.draw_commands.push(DrawIndirectArgs {
                vertex_count: instruction.vertex_count,
                instance_count: 1,
                first_vertex: instruction.vertex_offset,
                first_instance: instance_index as u32,
            });

            self.instance_indices.push(instance_index as u32);
        }

        self.instance_buffer.write(&self.device, &self.queue, &self.instance_data);
        self.command_buffer.write(&self.device, &self.queue, &self.draw_commands);
        self.instance_index_vertex_buffer
            .write(&self.device, &self.queue, &self.instance_indices);

        let instance_data_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry instance data"),
            layout: &self.instance_data_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.instance_buffer.as_entire_binding(),
            }],
        });

        render_pass.set_bind_group(1, &instance_data_bind_group, &[]);
        render_pass.set_bind_group(2, textures.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_index_vertex_buffer.slice(..));
        render_pass.multi_draw_indirect(self.command_buffer.get_buffer(), 0, self.draw_commands.len() as u32);
    }
}
