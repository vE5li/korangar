use std::num::NonZeroU64;
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderStages, StencilState, TextureSampleType, TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, GlobalContext, Prepare, RectangleInstruction, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/rectangle.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/rectangle_bindless.wgsl");
const DRAWER_NAME: &str = "forward rectangle";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

pub(crate) struct ForwardRectangleDrawInstruction<'a> {
    pub(crate) layer: ForwardRectangleLayer,
    pub(crate) instructions: &'a [RectangleInstruction],
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum ForwardRectangleLayer {
    Bottom = 0,
    Middle = 1,
    Top = 2,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct InstanceData {
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    texture_index: i32,
    linear_filtering: u32,
    padding: [u32; 2],
}

#[derive(Default, Copy, Clone)]
struct Batch {
    offset: usize,
    count: usize,
}

pub(crate) struct ForwardRectangleDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    layer_batches: [Batch; 3],
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for ForwardRectangleDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = ForwardRectangleDrawInstruction<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = if capabilities.supports_bindless() {
            device.create_shader_module(SHADER_BINDLESS)
        } else {
            device.create_shader_module(SHADER)
        };

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = if capabilities.supports_bindless() {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: capabilities.get_max_texture_binding_array_count(),
                    },
                ],
            })
        } else {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(DRAWER_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                    },
                    count: None,
                }],
            })
        };

        let bind_group = if capabilities.supports_bindless() {
            Self::create_bind_group_bindless(device, &bind_group_layout, &instance_data_buffer, &[global_context
                .solid_pixel_texture
                .get_texture_view()])
        } else {
            Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer)
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[&BindGroupLayout] = if capabilities.supports_bindless() {
            &[pass_bind_group_layouts[0], pass_bind_group_layouts[1], &bind_group_layout]
        } else {
            &[
                pass_bind_group_layouts[0],
                pass_bind_group_layouts[1],
                &bind_group_layout,
                Texture::bind_group_layout(device),
            ]
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self {
            bindless_support: capabilities.supports_bindless(),
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            layer_batches: [Batch::default(); 3],
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        let batch = self.layer_batches[draw_data.layer as usize];

        if batch.count == 0 {
            return;
        }
        let offset = batch.offset as u32;
        let count = batch.count as u32;
        let end = offset + count;

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, offset..end);
        } else {
            let mut current_texture_id = self.solid_pixel_texture.get_id();
            pass.set_bind_group(3, self.solid_pixel_texture.get_bind_group(), &[]);

            for (index, instruction) in draw_data.instructions.iter().enumerate() {
                if let RectangleInstruction::Sprite { texture, .. } = instruction
                    && texture.get_id() != current_texture_id
                {
                    current_texture_id = texture.get_id();
                    pass.set_bind_group(3, texture.get_bind_group(), &[]);
                }
                let index = offset + index as u32;

                pass.draw(0..6, index..index + 1);
            }
        }
    }
}

impl Prepare for ForwardRectangleDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.bottom_layer_rectangles.len()
            + instructions.middle_layer_rectangles.len()
            + instructions.top_layer_rectangles.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();

            let mut texture_views = Vec::with_capacity_in(draw_count, &self.bump);

            let mut offset = 0;

            for (batch_index, batch) in [
                instructions.bottom_layer_rectangles,
                instructions.middle_layer_rectangles,
                instructions.top_layer_rectangles,
            ]
            .into_iter()
            .enumerate()
            {
                let count = batch.len();
                self.layer_batches[batch_index] = Batch { offset, count };
                offset += count;

                for instruction in batch.iter() {
                    match instruction {
                        RectangleInstruction::Solid {
                            screen_position,
                            screen_size,
                            color,
                        } => {
                            self.instance_data.push(InstanceData {
                                color: color.components_linear(),
                                screen_position: (*screen_position).into(),
                                screen_size: (*screen_size).into(),
                                texture_position: [0.0; 2],
                                texture_size: [0.0; 2],
                                texture_index: -1,
                                linear_filtering: 0,
                                padding: Default::default(),
                            });
                        }
                        RectangleInstruction::Sprite {
                            screen_position,
                            screen_size,
                            color,
                            texture_position,
                            texture_size,
                            linear_filtering,
                            texture,
                        } => {
                            let mut texture_index = texture_views.len() as i32;
                            let id = texture.get_id();
                            let potential_index = self.lookup.get(&id);

                            if let Some(potential_index) = potential_index {
                                texture_index = *potential_index;
                            } else {
                                self.lookup.insert(id, texture_index);
                                texture_views.push(texture.get_texture_view());
                            }

                            self.instance_data.push(InstanceData {
                                color: color.components_linear(),
                                screen_position: (*screen_position).into(),
                                screen_size: (*screen_size).into(),
                                texture_position: (*texture_position).into(),
                                texture_size: (*texture_size).into(),
                                texture_index,
                                linear_filtering: *linear_filtering as u32,
                                padding: Default::default(),
                            });
                        }
                    }
                }
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views);
        } else {
            let mut offset = 0;

            for (batch_index, batch) in [
                instructions.bottom_layer_rectangles,
                instructions.middle_layer_rectangles,
                instructions.top_layer_rectangles,
            ]
            .into_iter()
            .enumerate()
            {
                let count = batch.len();
                self.layer_batches[batch_index] = Batch { offset, count };
                offset += count;

                for instruction in batch.iter() {
                    match instruction {
                        RectangleInstruction::Solid {
                            screen_position,
                            screen_size,
                            color,
                        } => {
                            self.instance_data.push(InstanceData {
                                color: color.components_linear(),
                                screen_position: (*screen_position).into(),
                                screen_size: (*screen_size).into(),
                                texture_position: [0.0; 2],
                                texture_size: [0.0; 2],
                                texture_index: -1,
                                linear_filtering: 0,
                                padding: Default::default(),
                            });
                        }
                        RectangleInstruction::Sprite {
                            screen_position,
                            screen_size,
                            color,
                            texture_position,
                            texture_size,
                            linear_filtering,
                            texture: _,
                        } => {
                            self.instance_data.push(InstanceData {
                                color: color.components_linear(),
                                screen_position: (*screen_position).into(),
                                screen_size: (*screen_size).into(),
                                texture_position: (*texture_position).into(),
                                texture_size: (*texture_size).into(),
                                texture_index: 0,
                                linear_filtering: *linear_filtering as u32,
                                padding: Default::default(),
                            });
                        }
                    }
                }
            }

            if self.instance_data_buffer.reserve(device, self.instance_data.len()) {
                self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer);
            }
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl ForwardRectangleDrawer {
    fn create_bind_group_bindless(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        texture_views: &[&TextureView],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(texture_views),
                },
            ],
        })
    }

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
