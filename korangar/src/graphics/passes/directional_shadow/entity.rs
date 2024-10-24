use std::num::NonZeroU64;
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState,
    Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, StencilState, TextureSampleType, TextureView,
    TextureViewDimension, VertexState,
};

use crate::graphics::cameras::NEAR_PLANE;
use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DirectionalShadowRenderPassContext, Drawer, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, EntityInstruction, GlobalContext, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/entity.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/entity_bindless.wgsl");
const DRAWER_NAME: &str = "directional shadow entity";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    world: [[f32; 4]; 4],
    frame_part_transform: [[f32; 4]; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    extra_depth_offset: f32,
    depth_offset: f32,
    angle: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
    padding: [u32; 2],
}

pub(crate) struct DirectionalShadowEntityDrawer {
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

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }> for DirectionalShadowEntityDrawer {
    type Context = DirectionalShadowRenderPassContext;
    type DrawData<'data> = &'data [EntityInstruction];

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

        let mut constants = std::collections::HashMap::new();
        constants.insert("near_plane".to_string(), NEAR_PLANE as f64);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                targets: &[],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
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
            draw_count: 0,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, 0..self.draw_count as u32);
        } else {
            let mut current_texture_id = self.solid_pixel_texture.get_id();
            pass.set_bind_group(3, self.solid_pixel_texture.get_bind_group(), &[]);

            for (index, instruction) in draw_data[0..self.draw_count].iter().enumerate() {
                if instruction.texture.get_id() != current_texture_id {
                    current_texture_id = instruction.texture.get_id();
                    pass.set_bind_group(3, instruction.texture.get_bind_group(), &[]);
                }
                let index = index as u32;

                pass.draw(0..6, index..index + 1);
            }
        }
    }
}

impl Prepare for DirectionalShadowEntityDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.directional_shadow_entities.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();
            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for instruction in instructions.directional_shadow_entities.iter() {
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
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
                    angle: instruction.angle,
                    curvature: instruction.curvature,
                    mirror: instruction.mirror as u32,
                    texture_index,
                    padding: Default::default(),
                });
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
        } else {
            for instruction in instructions.directional_shadow_entities.iter() {
                self.instance_data.push(InstanceData {
                    world: instruction.world.into(),
                    frame_part_transform: instruction.frame_part_transform.into(),
                    texture_position: instruction.texture_position.into(),
                    texture_size: instruction.texture_size.into(),
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
                    angle: instruction.angle,
                    curvature: instruction.curvature,
                    mirror: instruction.mirror as u32,
                    texture_index: 0,
                    padding: Default::default(),
                });
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl DirectionalShadowEntityDrawer {
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
