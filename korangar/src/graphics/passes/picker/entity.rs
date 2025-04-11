use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState,
    DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, StencilState, TextureSampleType,
    TextureView, TextureViewDimension, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PickerRenderPassContext, RenderPassContext,
};
use crate::graphics::{
    BindlessSupport, Buffer, Capabilities, EntityInstruction, GlobalContext, PickerTarget, Prepare, RenderInstruction, Texture,
};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/entity.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/entity_bindless.wgsl");
const DRAWER_NAME: &str = "picker entity";
const INITIAL_INSTRUCTION_SIZE: usize = 128;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
    frame_part_transform: [[f32; 4]; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    texture_index: i32,
    mirror: u32,
    identifier_high: u32,
    identifier_low: u32,
}

pub(crate) struct PickerEntityDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for PickerEntityDrawer {
    type Context = PickerRenderPassContext;
    type DrawData<'data> = &'data [EntityInstruction];

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = if capabilities.bindless_support() == BindlessSupport::Full {
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

        let bind_group_layout = if capabilities.bindless_support() == BindlessSupport::Full {
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
                        count: NonZeroU32::new(capabilities.get_max_texture_binding_array_count()),
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

        let bind_group = if capabilities.bindless_support() == BindlessSupport::Full {
            Self::create_bind_group_bindless(device, &bind_group_layout, &instance_data_buffer, &[global_context
                .solid_pixel_texture
                .get_texture_view()])
        } else {
            Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer)
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[&BindGroupLayout] = if capabilities.bindless_support() == BindlessSupport::Full {
            &[pass_bind_group_layouts[0], &bind_group_layout]
        } else {
            &[pass_bind_group_layouts[0], &bind_group_layout, Texture::bind_group_layout(device)]
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
                    blend: None,
                    write_mask: ColorWrites::default(),
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
            bindless_support: capabilities.bindless_support() == BindlessSupport::Full,
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            draw_count: 0,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 || draw_data.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, 0..self.draw_count as u32);
        } else {
            let mut current_texture_id = self.solid_pixel_texture.get_id();
            pass.set_bind_group(2, self.solid_pixel_texture.get_bind_group(), &[]);

            draw_data
                .iter()
                .filter(|instruction| instruction.add_to_picker)
                .enumerate()
                .for_each(|(index, instruction)| {
                    if instruction.texture.get_id() != current_texture_id {
                        current_texture_id = instruction.texture.get_id();
                        pass.set_bind_group(2, instruction.texture.get_bind_group(), &[]);
                    }
                    let index = index as u32;

                    pass.draw(0..6, index..index + 1);
                });
        }
    }
}

impl Prepare for PickerEntityDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.instance_data.clear();
        self.draw_count = 0;

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();

            let mut texture_views = Vec::with_capacity_in(instructions.entities.len(), &self.bump);

            instructions
                .entities
                .iter()
                .filter(|instruction| instruction.add_to_picker)
                .for_each(|instruction| {
                    self.draw_count += 1;

                    let picker_target = PickerTarget::Entity(instruction.entity_id);
                    let (identifier_high, identifier_low) = picker_target.into();

                    let mut texture_index = texture_views.len() as i32;
                    let id = instruction.texture.get_id();
                    let potential_index = self.lookup.get(&id);

                    if let Some(potential_index) = potential_index {
                        texture_index = *potential_index;
                    } else {
                        self.lookup.insert(id, texture_index);
                        texture_views.push(instruction.texture.get_texture_view());
                    }

                    self.instance_data.push(InstanceData {
                        world: instruction.world.into(),
                        frame_part_transform: instruction.frame_part_transform.into(),
                        texture_position: instruction.texture_position.into(),
                        texture_size: instruction.texture_size.into(),
                        texture_index,
                        mirror: instruction.mirror as u32,
                        identifier_high,
                        identifier_low,
                    });
                });

            if self.draw_count == 0 {
                return;
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
        } else {
            instructions
                .entities
                .iter()
                .filter(|instruction| instruction.add_to_picker)
                .for_each(|instruction| {
                    self.draw_count += 1;

                    let picker_target = PickerTarget::Entity(instruction.entity_id);
                    let (identifier_high, identifier_low) = picker_target.into();

                    self.instance_data.push(InstanceData {
                        world: instruction.world.into(),
                        frame_part_transform: instruction.frame_part_transform.into(),
                        texture_position: instruction.texture_position.into(),
                        texture_size: instruction.texture_size.into(),
                        texture_index: 0,
                        mirror: instruction.mirror as u32,
                        identifier_high,
                        identifier_low,
                    });
                });

            if self.draw_count == 0 {
                return;
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer);
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl PickerEntityDrawer {
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
