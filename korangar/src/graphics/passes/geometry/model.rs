use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix, Matrix4, SquareMatrix, Transform};
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DrawIndirectArgs, Drawer, GeometryRenderPassContext, ModelBatchDrawData,
    RenderPassContext,
};
use crate::graphics::{Buffer, GlobalContext, ModelVertex, Prepare, RenderInstruction, TextureGroup};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/model.wgsl");
const DRAWER_NAME: &str = "geometry model";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
    inv_world: [[f32; 4]; 4],
}

pub(crate) struct GeometryModelDrawer {
    instance_data_buffer: Buffer<InstanceData>,
    instance_index_vertex_buffer: Buffer<u32>,
    command_buffer: Buffer<DrawIndirectArgs>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    #[cfg(feature = "debug")]
    wireframe_pipeline: RenderPipeline,
    instance_data: Vec<InstanceData>,
    instance_indices: Vec<u32>,
    draw_commands: Vec<DrawIndirectArgs>,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for GeometryModelDrawer {
    type Context = GeometryRenderPassContext;
    type DrawData<'data> = ModelBatchDrawData<'data>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
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
                shader_location: 6,
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
                &bind_group_layout,
                TextureGroup::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        });

        #[cfg(feature = "debug")]
        let wireframe_pipeline = Self::create_pipeline(
            device,
            render_pass_context,
            &shader_module,
            instance_index_buffer_layout.clone(),
            &pipeline_layout,
            PolygonMode::Line,
        );

        let pipeline = Self::create_pipeline(
            device,
            render_pass_context,
            &shader_module,
            instance_index_buffer_layout,
            &pipeline_layout,
            PolygonMode::Fill,
        );

        Self {
            instance_data_buffer,
            instance_index_vertex_buffer,
            command_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            #[cfg(feature = "debug")]
            wireframe_pipeline,
            instance_data: Vec::default(),
            instance_indices: Vec::default(),
            draw_commands: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.batches.is_empty() {
            return;
        }

        #[cfg(feature = "debug")]
        if draw_data.show_wireframe {
            pass.set_pipeline(&self.wireframe_pipeline);
        } else {
            pass.set_pipeline(&self.pipeline);
        }

        #[cfg(not(feature = "debug"))]
        pass.set_pipeline(&self.pipeline);

        pass.set_bind_group(1, &self.bind_group, &[]);

        for batch in draw_data.batches.iter() {
            if batch.count == 0 {
                continue;
            }

            let textures = batch.textures.as_deref().unwrap_or(draw_data.map_textures);
            let vertex_buffer = batch.vertex_buffer.as_deref().unwrap_or(draw_data.map_vertex_buffer);

            pass.set_bind_group(2, textures.bind_group(), &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_index_vertex_buffer.slice(..));
            pass.multi_draw_indirect(
                self.command_buffer.get_buffer(),
                (batch.offset * size_of::<DrawIndirectArgs>()) as BufferAddress,
                batch.count as u32,
            );
        }
    }
}

impl Prepare for GeometryModelDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.models.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();
        self.instance_indices.clear();
        self.draw_commands.clear();

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

impl GeometryModelDrawer {
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
        render_pass_context: &GeometryRenderPassContext,
        shader_module: &ShaderModule,
        instance_index_buffer_layout: VertexBufferLayout,
        pipeline_layout: &PipelineLayout,
        polygon_mode: PolygonMode,
    ) -> RenderPipeline {
        let color_attachment_formats = render_pass_context.color_attachment_formats();

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(pipeline_layout),
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
                targets: &[
                    Some(ColorTargetState {
                        format: color_attachment_formats[0],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[1],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[2],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                ],
            }),
            multiview: None,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                polygon_mode,
                ..Default::default()
            },
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }
}
