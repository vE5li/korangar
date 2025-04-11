use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix, Matrix4, SquareMatrix, Transform};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, IndexFormat,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, StencilState, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};

use crate::graphics::passes::forward::ForwardRenderPassContext;
use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DrawIndexedIndirectArgs, Drawer, ModelBatchDrawData, RenderPassContext,
};
use crate::graphics::{
    BindlessSupport, Buffer, Capabilities, GlobalContext, ModelBatch, ModelVertex, Msaa, Prepare, RenderInstruction, Texture, TextureSet,
};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/model.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/model_bindless.wgsl");
#[cfg(feature = "debug")]
const SHADER_WIREFRAME: ShaderModuleDescriptor = include_wgsl!("shader/model_wireframe.wgsl");
const DRAWER_NAME: &str = "forward model";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
    inv_world: [[f32; 4]; 4],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum ModelPassMode {
    /// Draws all opaque parts of models that have no transparent textures.
    Opaque = 0,
    /// Draws all opaque parts of models that have transparent textures.
    SemiOpaque = 1,
    /// Draws all transparent parts of models that have transparent textures.
    Transparent = 2,
}

pub(crate) struct ForwardModelDrawData<'a> {
    pub(crate) batch_data: &'a ModelBatchDrawData<'a>,
    pub(crate) pass_mode: ModelPassMode,
}

pub(crate) struct ForwardModelDrawer {
    multi_draw_indirect_support: bool,
    bindless_support: BindlessSupport,
    instance_data_buffer: Buffer<InstanceData>,
    instance_index_vertex_buffer: Buffer<u32>,
    command_buffer: Buffer<DrawIndexedIndirectArgs>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    opaque_pipeline: RenderPipeline,
    semi_transparent_pipeline: RenderPipeline,
    transparent_pipeline: RenderPipeline,
    #[cfg(feature = "debug")]
    wireframe_pipeline: RenderPipeline,
    instance_data: Vec<InstanceData>,
    instance_indices: Vec<u32>,
    draw_commands: Vec<DrawIndexedIndirectArgs>,
    opaque_batches: Vec<ModelBatch>,
    transparent_batches: Vec<ModelBatch>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardModelDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = ForwardModelDrawData<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = match capabilities.bindless_support() {
            BindlessSupport::Full | BindlessSupport::Limited => device.create_shader_module(SHADER_BINDLESS),
            BindlessSupport::None => device.create_shader_module(SHADER),
        };

        #[cfg(feature = "debug")]
        let shader_module_wireframe = device.create_shader_module(SHADER_WIREFRAME);

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
                shader_location: 6,
            }],
        };

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

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let texture_bind_group = match capabilities.bindless_support() {
            BindlessSupport::Full | BindlessSupport::Limited => {
                TextureSet::bind_group_layout(device, capabilities.get_max_textures_per_shader_stage())
            }
            BindlessSupport::None => Texture::bind_group_layout(device),
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                pass_bind_group_layouts[0],
                pass_bind_group_layouts[1],
                &bind_group_layout,
                texture_bind_group,
            ],
            push_constant_ranges: &[],
        });

        let color_attachment_formats = render_pass_context.color_attachment_formats();

        #[cfg(feature = "debug")]
        let wireframe_pipeline = if capabilities.supports_polygon_mode_line() {
            Self::create_pipeline(
                device,
                render_pass_context,
                global_context.msaa,
                &shader_module_wireframe,
                instance_index_buffer_layout.clone(),
                &pipeline_layout,
                PolygonMode::Line,
                &color_attachment_formats,
                ModelPassMode::Opaque,
            )
        } else {
            Self::create_pipeline(
                device,
                render_pass_context,
                global_context.msaa,
                &shader_module_wireframe,
                instance_index_buffer_layout.clone(),
                &pipeline_layout,
                PolygonMode::Fill,
                &color_attachment_formats,
                ModelPassMode::Opaque,
            )
        };

        let opaque_pipeline = Self::create_pipeline(
            device,
            render_pass_context,
            global_context.msaa,
            &shader_module,
            instance_index_buffer_layout.clone(),
            &pipeline_layout,
            PolygonMode::Fill,
            &color_attachment_formats,
            ModelPassMode::Opaque,
        );

        let semi_transparent_pipeline = Self::create_pipeline(
            device,
            render_pass_context,
            global_context.msaa,
            &shader_module,
            instance_index_buffer_layout.clone(),
            &pipeline_layout,
            PolygonMode::Fill,
            &color_attachment_formats,
            ModelPassMode::SemiOpaque,
        );

        let transparent_pipeline = Self::create_pipeline(
            device,
            render_pass_context,
            global_context.msaa,
            &shader_module,
            instance_index_buffer_layout,
            &pipeline_layout,
            PolygonMode::Fill,
            &color_attachment_formats,
            ModelPassMode::Transparent,
        );

        Self {
            multi_draw_indirect_support: capabilities.supports_multidraw_indirect(),
            bindless_support: capabilities.bindless_support(),
            instance_data_buffer,
            instance_index_vertex_buffer,
            command_buffer,
            bind_group_layout,
            bind_group,
            opaque_pipeline,
            semi_transparent_pipeline,
            transparent_pipeline,
            #[cfg(feature = "debug")]
            wireframe_pipeline,
            instance_data: Vec::default(),
            instance_indices: Vec::default(),
            draw_commands: Vec::default(),
            opaque_batches: Vec::default(),
            transparent_batches: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.batch_data.instructions.is_empty()
            || (draw_data.pass_mode == ModelPassMode::Opaque && self.opaque_batches.is_empty())
            || (draw_data.pass_mode != ModelPassMode::Opaque && self.transparent_batches.is_empty())
        {
            return;
        }

        fn process_batches(
            pass: &mut RenderPass<'_>,
            batches: &[ModelBatch],
            draw_data: &ModelBatchDrawData,
            instance_index_vertex_buffer: &Buffer<u32>,
            command_buffer: &Buffer<DrawIndexedIndirectArgs>,
            multi_draw_indirect_support: bool,
            bindless_support: BindlessSupport,
        ) {
            match bindless_support {
                BindlessSupport::Full | BindlessSupport::Limited => {
                    for batch in batches {
                        pass.set_bind_group(3, batch.texture_set.get_bind_group().unwrap(), &[]);
                        pass.set_index_buffer(batch.index_buffer.slice(..), IndexFormat::Uint32);
                        pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));
                        pass.set_vertex_buffer(1, instance_index_vertex_buffer.slice(..));

                        match multi_draw_indirect_support {
                            true => pass.multi_draw_indexed_indirect(
                                command_buffer.get_buffer(),
                                (batch.offset * size_of::<DrawIndexedIndirectArgs>()) as BufferAddress,
                                batch.count as u32,
                            ),
                            false => {
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
                }
                BindlessSupport::None => {
                    for batch in batches {
                        pass.set_index_buffer(batch.index_buffer.slice(..), IndexFormat::Uint32);
                        pass.set_vertex_buffer(0, batch.vertex_buffer.slice(..));
                        pass.set_vertex_buffer(1, instance_index_vertex_buffer.slice(..));

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

        pass.set_bind_group(2, &self.bind_group, &[]);

        match draw_data.pass_mode {
            ModelPassMode::Opaque => {
                #[cfg(feature = "debug")]
                let opaque_pipeline = if draw_data.batch_data.show_wireframe {
                    &self.wireframe_pipeline
                } else {
                    &self.opaque_pipeline
                };
                #[cfg(not(feature = "debug"))]
                let opaque_pipeline = &self.opaque_pipeline;

                pass.set_pipeline(opaque_pipeline);

                process_batches(
                    pass,
                    &self.opaque_batches,
                    draw_data.batch_data,
                    &self.instance_index_vertex_buffer,
                    &self.command_buffer,
                    self.multi_draw_indirect_support,
                    self.bindless_support,
                );
            }
            ModelPassMode::SemiOpaque => {
                #[cfg(feature = "debug")]
                let semi_transparent_pipeline = if draw_data.batch_data.show_wireframe {
                    &self.wireframe_pipeline
                } else {
                    &self.semi_transparent_pipeline
                };
                #[cfg(not(feature = "debug"))]
                let semi_transparent_pipeline = &self.semi_transparent_pipeline;

                pass.set_pipeline(semi_transparent_pipeline);

                process_batches(
                    pass,
                    &self.transparent_batches,
                    draw_data.batch_data,
                    &self.instance_index_vertex_buffer,
                    &self.command_buffer,
                    self.multi_draw_indirect_support,
                    self.bindless_support,
                );
            }
            ModelPassMode::Transparent => {
                #[cfg(feature = "debug")]
                if draw_data.batch_data.show_wireframe {
                    return;
                }

                pass.set_pipeline(&self.transparent_pipeline);

                process_batches(
                    pass,
                    &self.transparent_batches,
                    draw_data.batch_data,
                    &self.instance_index_vertex_buffer,
                    &self.command_buffer,
                    self.multi_draw_indirect_support,
                    self.bindless_support,
                );
            }
        }
    }
}

impl Prepare for ForwardModelDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.models.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();
        self.instance_indices.clear();
        self.draw_commands.clear();
        self.opaque_batches.clear();
        self.transparent_batches.clear();

        // We assume that batches inside instructions are sorted by transparency (first
        // opaque, then transparent models).
        for batch in instructions.model_batches {
            let start = batch.offset;
            let end = batch.offset + batch.count;

            let relative_transparent_start = instructions.models[start..end].iter().position(|model| model.transparent);

            if let Some(relative_transparent_start) = relative_transparent_start {
                let absolute_transparent_start = start + relative_transparent_start;
                let opaque_count = relative_transparent_start;
                let transparent_count = end - absolute_transparent_start;

                self.opaque_batches.push(ModelBatch {
                    offset: batch.offset,
                    count: opaque_count,
                    texture_set: batch.texture_set.clone(),
                    vertex_buffer: batch.vertex_buffer.clone(),
                    index_buffer: batch.index_buffer.clone(),
                });

                self.transparent_batches.push(ModelBatch {
                    offset: absolute_transparent_start,
                    count: transparent_count,
                    texture_set: batch.texture_set.clone(),
                    vertex_buffer: batch.vertex_buffer.clone(),
                    index_buffer: batch.index_buffer.clone(),
                });
            } else {
                self.opaque_batches.push(ModelBatch {
                    offset: batch.offset,
                    count: batch.count,
                    texture_set: batch.texture_set.clone(),
                    vertex_buffer: batch.vertex_buffer.clone(),
                    index_buffer: batch.index_buffer.clone(),
                });
            }
        }

        for instruction in instructions.models.iter() {
            let instance_index = self.instance_data.len();

            self.instance_data.push(InstanceData {
                world: instruction.model_matrix.into(),
                inv_world: instruction
                    .model_matrix
                    .inverse_transform()
                    .unwrap_or(Matrix4::identity())
                    .transpose()
                    .into(),
            });

            self.instance_indices.push(instance_index as u32);

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
        self.instance_index_vertex_buffer
            .write(device, staging_belt, command_encoder, &self.instance_indices);
        self.command_buffer
            .write(device, staging_belt, command_encoder, &self.draw_commands);

        if recreated {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }
}

impl ForwardModelDrawer {
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

    fn create_pipeline(
        device: &Device,
        render_pass_context: &ForwardRenderPassContext,
        msaa: Msaa,
        shader_module: &ShaderModule,
        instance_index_buffer_layout: VertexBufferLayout,
        pipeline_layout: &PipelineLayout,
        polygon_mode: PolygonMode,
        color_attachment_formats: &[TextureFormat; 3],
        pass_mode: ModelPassMode,
    ) -> RenderPipeline {
        let targets = match pass_mode {
            ModelPassMode::Opaque | ModelPassMode::SemiOpaque => [
                Some(ColorTargetState {
                    format: color_attachment_formats[0],
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[1],
                    blend: Some(BlendState {
                        color: BlendComponent::default(),
                        alpha: BlendComponent::default(),
                    }),
                    write_mask: ColorWrites::empty(),
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[2],
                    blend: Some(BlendState {
                        color: BlendComponent::default(),
                        alpha: BlendComponent::default(),
                    }),
                    write_mask: ColorWrites::empty(),
                }),
            ],
            ModelPassMode::Transparent => [
                Some(ColorTargetState {
                    format: color_attachment_formats[0],
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::empty(),
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[1],
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: color_attachment_formats[2],
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::OneMinusSrc,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::default(),
                    }),
                    write_mask: ColorWrites::RED,
                }),
            ],
        };

        let opaque = pass_mode != ModelPassMode::Transparent;
        let alpha_to_coverage_activated = msaa.multisampling_activated() && opaque;

        let constants = &[
            ("ALPHA_TO_COVERAGE_ACTIVATED", f64::from(u32::from(alpha_to_coverage_activated))),
            ("PASS_MODE", f64::from(pass_mode as u32)),
        ];

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{DRAWER_NAME} {pass_mode:?}")),
            layout: Some(pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[ModelVertex::buffer_layout(), instance_index_buffer_layout],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: if opaque { Some("opaque_main") } else { Some("transparent_main") },
                compilation_options: PipelineCompilationOptions {
                    constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &targets,
            }),
            multiview: None,
            primitive: PrimitiveState {
                cull_mode: if opaque { Some(Face::Back) } else { None },
                front_face: FrontFace::Ccw,
                polygon_mode,
                ..Default::default()
            },
            multisample: if msaa.multisampling_activated() {
                MultisampleState {
                    count: msaa.sample_count(),
                    alpha_to_coverage_enabled: alpha_to_coverage_activated,
                    ..Default::default()
                }
            } else {
                MultisampleState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: opaque,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        })
    }
}
