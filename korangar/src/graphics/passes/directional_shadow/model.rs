use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferAddress, BufferBindingType, BufferUsages, CommandEncoder, CompareFunction, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DirectionalShadowRenderPassContext, DrawIndirectArgs, Drawer,
    ModelBatchDrawData, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, GlobalContext, ModelVertex, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/model.wgsl");
const DRAWER_NAME: &str = "directional shadow model";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
}

pub(crate) struct DirectionalShadowModelDrawer {
    multi_draw_indirect_support: bool,
    instance_data_buffer: Buffer<InstanceData>,
    instance_index_vertex_buffer: Buffer<u32>,
    command_buffer: Buffer<DrawIndirectArgs>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    instance_data: Vec<InstanceData>,
    instance_indices: Vec<u32>,
    draw_commands: Vec<DrawIndirectArgs>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }> for DirectionalShadowModelDrawer {
    type Context = DirectionalShadowRenderPassContext;
    type DrawData<'data> = ModelBatchDrawData<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        // TODO: NHA This instance index vertex buffer is only needed until this issue is fixed for DX12: https://github.com/gfx-rs/wgpu/issues/2471
        let instance_index_vertex_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} index vertex data"),
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            (size_of::<u32>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let instance_index_buffer_layout = VertexBufferLayout {
            array_stride: size_of::<u32>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 0,
                shader_location: 5,
            }],
        };

        let command_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} indirect buffer"),
            BufferUsages::COPY_DST | BufferUsages::INDIRECT,
            (size_of::<DrawIndirectArgs>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
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

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Self::Context::bind_group_layout(device)[0],
                Self::Context::bind_group_layout(device)[1],
                &bind_group_layout,
                Texture::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout(), instance_index_buffer_layout],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        });

        Self {
            multi_draw_indirect_support: capabilities.supports_multidraw_indirect(),
            instance_data_buffer,
            instance_index_vertex_buffer,
            command_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            instance_data: Vec::default(),
            instance_indices: Vec::default(),
            draw_commands: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.batches.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);

        for batch in draw_data.batches.iter() {
            if batch.count == 0 {
                continue;
            }

            pass.set_bind_group(3, batch.texture.get_bind_group(), &[]);
            pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_index_vertex_buffer.slice(..));

            if self.multi_draw_indirect_support {
                pass.multi_draw_indirect(
                    self.command_buffer.get_buffer(),
                    (batch.offset * size_of::<DrawIndirectArgs>()) as BufferAddress,
                    batch.count as u32,
                );
            } else {
                let start = batch.offset;
                let end = start + batch.count;

                for (index, instruction) in draw_data.instructions[start..end].iter().enumerate() {
                    let vertex_start = instruction.vertex_offset as u32;
                    let vertex_end = vertex_start + instruction.vertex_count as u32;
                    let index = (start + index) as u32;

                    pass.draw(vertex_start..vertex_end, index..index + 1);
                }
            }
        }
    }
}

impl Prepare for DirectionalShadowModelDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.directional_shadow_models.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();
        self.instance_indices.clear();
        self.draw_commands.clear();

        for instruction in instructions.directional_shadow_models.iter() {
            let instance_index = self.instance_data.len();

            self.instance_data.push(InstanceData {
                world: instruction.model_matrix.into(),
            });

            self.instance_indices.push(instance_index as u32);

            self.draw_commands.push(DrawIndirectArgs {
                vertex_count: instruction.vertex_count as u32,
                instance_count: 1,
                first_vertex: instruction.vertex_offset as u32,
                first_instance: instance_index as u32,
            });
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
        self.instance_index_vertex_buffer
            .write(device, staging_belt, command_encoder, &self.instance_indices);
        self.command_buffer
            .write(device, staging_belt, command_encoder, &self.draw_commands);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }
}

impl DirectionalShadowModelDrawer {
    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, instance_data_buffer: &Buffer<InstanceData>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: instance_data_buffer.as_entire_binding(),
            }],
        })
    }
}
