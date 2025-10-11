use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState,
    DepthStencilState, Device, FragmentState, IndexFormat, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DirectionalShadowRenderPassContext, DrawIndexedIndirectArgs, Drawer,
    ModelBatchDrawData, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{BindlessSupport, Buffer, Capabilities, GlobalContext, ModelVertex, Prepare, RenderInstruction, Texture, TextureSet};

const DRAWER_NAME: &str = "directional shadow model";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
}

pub(crate) struct DirectionalShadowModelDrawer {
    multi_draw_indirect_support: bool,
    bindless_support: BindlessSupport,
    instance_data_buffer: Buffer<InstanceData>,
    command_buffer: Buffer<DrawIndexedIndirectArgs>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    instance_data: Vec<InstanceData>,
    draw_commands: Vec<DrawIndexedIndirectArgs>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for DirectionalShadowModelDrawer {
    type Context = DirectionalShadowRenderPassContext;
    type DrawData<'data> = ModelBatchDrawData<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = match capabilities.bindless_support() {
            BindlessSupport::Full | BindlessSupport::Limited => {
                shader_compiler.create_shader_module("directional_shadow", "model_bindless")
            }
            BindlessSupport::None => shader_compiler.create_shader_module("directional_shadow", "model"),
        };

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let command_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} indirect buffer"),
            BufferUsages::COPY_DST | BufferUsages::INDIRECT,
            (size_of::<DrawIndexedIndirectArgs>() * INITIAL_INSTRUCTION_SIZE) as _,
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

        let texture_bind_group = match capabilities.bindless_support() {
            BindlessSupport::Full | BindlessSupport::Limited => {
                TextureSet::bind_group_layout(device, capabilities.get_max_texture_binding_array_count())
            }
            BindlessSupport::None => Texture::bind_group_layout(device),
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Self::Context::bind_group_layout(device)[0],
                Self::Context::bind_group_layout(device)[1],
                &bind_group_layout,
                texture_bind_group,
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
                buffers: &[ModelVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: None,
                    write_mask: ColorWrites::empty(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self {
            multi_draw_indirect_support: capabilities.supports_multidraw_indirect(),
            bindless_support: capabilities.bindless_support(),
            instance_data_buffer,
            command_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            instance_data: Vec::default(),
            draw_commands: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.batches.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);

        match self.bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                for batch in draw_data.batches.iter() {
                    if batch.count == 0 {
                        continue;
                    }

                    pass.set_bind_group(3, batch.texture_set.get_bind_group().unwrap(), &[]);
                    pass.set_index_buffer(batch.index_buffer.slice(..), IndexFormat::Uint32);
                    pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));

                    if self.multi_draw_indirect_support {
                        pass.multi_draw_indexed_indirect(
                            self.command_buffer.get_buffer(),
                            (batch.offset * size_of::<DrawIndexedIndirectArgs>()) as BufferAddress,
                            batch.count as u32,
                        );
                    } else {
                        let start = batch.offset;
                        let end = start + batch.count;

                        for (index, instruction) in draw_data.instructions[start..end].iter().enumerate() {
                            let index_start = instruction.index_offset;
                            let index_end = index_start + instruction.index_count;
                            let instance_offset = (start + index) as u32;

                            pass.draw_indexed(
                                index_start..index_end,
                                instruction.base_vertex,
                                instance_offset..instance_offset + 1,
                            );
                        }
                    }
                }
            }
            BindlessSupport::None => {
                for batch in draw_data.batches.iter() {
                    if batch.count == 0 {
                        continue;
                    }

                    pass.set_index_buffer(batch.index_buffer.slice(..), IndexFormat::Uint32);
                    pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));

                    let start = batch.offset;
                    let end = start + batch.count;

                    for (index, instruction) in draw_data.instructions[start..end].iter().enumerate() {
                        let index_start = instruction.index_offset;
                        let index_end = index_start + instruction.index_count;
                        let instance_offset = (start + index) as u32;
                        let texture_bind_group = batch.texture_set.get_texture_bind_group(instruction.texture_index);

                        pass.set_bind_group(3, texture_bind_group, &[]);
                        pass.draw_indexed(
                            index_start..index_end,
                            instruction.base_vertex,
                            instance_offset..instance_offset + 1,
                        );
                    }
                }
            }
        }
    }
}

impl Prepare for DirectionalShadowModelDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        if instructions
            .directional_shadow_model_batches
            .iter()
            .all(|instructions| instructions.is_empty())
        {
            return;
        }

        self.instance_data.clear();
        self.draw_commands.clear();

        for instruction in instructions.directional_shadow_models.iter() {
            let instance_index = self.instance_data.len();

            self.instance_data.push(InstanceData {
                world: instruction.model_matrix.into(),
            });

            self.draw_commands.push(DrawIndexedIndirectArgs {
                index_count: instruction.index_count,
                instance_count: 1,
                first_index: instruction.index_offset,
                base_vertex: instruction.base_vertex,
                first_instance: instance_index as u32,
            });
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
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
