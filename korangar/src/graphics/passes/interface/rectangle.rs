use std::num::NonZeroU64;
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, InterfaceRenderPassContext, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, GlobalContext, InterfaceRectangleInstruction, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/rectangle.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/rectangle_bindless.wgsl");
const DRAWER_NAME: &str = "interface rectangle";
const INITIAL_INSTRUCTION_SIZE: usize = 1024;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    color: [f32; 4],
    corner_radius: [f32; 4],
    screen_clip: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    rectangle_type: u32,
    texture_index: i32,
    smooth: u32,
    padding: u32,
}

pub(crate) struct InterfaceRectangleDrawer {
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

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for InterfaceRectangleDrawer {
    type Context = InterfaceRenderPassContext;
    type DrawData<'data> = &'data [InterfaceRectangleInstruction];

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
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        } else {
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
                        count: None,
                    },
                ],
            })
        };

        let bind_group = if capabilities.supports_bindless() {
            Self::create_bind_group_bindless(
                device,
                &bind_group_layout,
                &instance_data_buffer,
                &[global_context.solid_pixel_texture.get_texture_view()],
                global_context.solid_pixel_texture.get_texture_view(),
            )
        } else {
            Self::create_bind_group(
                device,
                &bind_group_layout,
                &instance_data_buffer,
                global_context.solid_pixel_texture.get_texture_view(),
            )
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[&BindGroupLayout] = if capabilities.supports_bindless() {
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
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: Default::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
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
        pass.set_bind_group(1, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, 0..self.draw_count as u32);
        } else {
            let mut current_texture_id = self.solid_pixel_texture.get_id();
            pass.set_bind_group(2, self.solid_pixel_texture.get_bind_group(), &[]);

            for (index, instruction) in draw_data.iter().enumerate() {
                if let InterfaceRectangleInstruction::Sprite { texture, .. } = instruction
                    && texture.get_id() != current_texture_id
                {
                    current_texture_id = texture.get_id();
                    pass.set_bind_group(2, texture.get_bind_group(), &[]);
                }
                let index = index as u32;

                pass.draw(0..6, index..index + 1);
            }
        }
    }
}

impl Prepare for InterfaceRectangleDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.interface.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();

            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for instruction in instructions.interface.iter() {
                match instruction {
                    InterfaceRectangleInstruction::Solid {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        corner_radius,
                    } => {
                        self.instance_data.push(InstanceData {
                            color: color.components_linear(),
                            corner_radius: (*corner_radius).into(),
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: [0.0, 0.0],
                            texture_size: [1.0, 1.0],
                            rectangle_type: 0,
                            texture_index: 0,
                            smooth: 0,
                            padding: 0,
                        });
                    }
                    InterfaceRectangleInstruction::Sprite {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        texture,
                        smooth,
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
                            corner_radius: [0.0, 0.0, 0.0, 0.0],
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: [0.0, 0.0],
                            texture_size: [1.0, 1.0],
                            rectangle_type: 1,
                            texture_index,
                            smooth: *smooth as u32,
                            padding: 0,
                        });
                    }
                    InterfaceRectangleInstruction::Text {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        texture_position,
                        texture_size,
                    } => {
                        self.instance_data.push(InstanceData {
                            color: color.components_linear(),
                            corner_radius: [0.0, 0.0, 0.0, 0.0],
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: (*texture_position).into(),
                            texture_size: (*texture_size).into(),
                            rectangle_type: 2,
                            texture_index: 0,
                            smooth: 1,
                            padding: 0,
                        });
                    }
                }
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(
                device,
                &self.bind_group_layout,
                &self.instance_data_buffer,
                &texture_views,
                instructions.font_atlas_texture.get_texture_view(),
            )
        } else {
            for instruction in instructions.interface.iter() {
                match instruction {
                    InterfaceRectangleInstruction::Solid {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        corner_radius,
                    } => {
                        self.instance_data.push(InstanceData {
                            color: color.components_linear(),
                            corner_radius: (*corner_radius).into(),
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: [0.0, 0.0],
                            texture_size: [1.0, 1.0],
                            rectangle_type: 0,
                            texture_index: 0,
                            smooth: 0,
                            padding: 0,
                        });
                    }
                    InterfaceRectangleInstruction::Sprite {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        texture: _,
                        smooth,
                    } => {
                        self.instance_data.push(InstanceData {
                            color: color.components_linear(),
                            corner_radius: [0.0, 0.0, 0.0, 0.0],
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: [0.0, 0.0],
                            texture_size: [1.0, 1.0],
                            rectangle_type: 1,
                            texture_index: 0,
                            smooth: *smooth as u32,
                            padding: 0,
                        });
                    }
                    InterfaceRectangleInstruction::Text {
                        screen_position,
                        screen_size,
                        screen_clip,
                        color,
                        texture_position,
                        texture_size,
                    } => {
                        self.instance_data.push(InstanceData {
                            color: color.components_linear(),
                            corner_radius: [0.0, 0.0, 0.0, 0.0],
                            screen_clip: (*screen_clip).into(),
                            screen_position: (*screen_position).into(),
                            screen_size: (*screen_size).into(),
                            texture_position: (*texture_position).into(),
                            texture_size: (*texture_size).into(),
                            rectangle_type: 2,
                            texture_index: 0,
                            smooth: 1,
                            padding: 0,
                        });
                    }
                }
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(
                device,
                &self.bind_group_layout,
                &self.instance_data_buffer,
                instructions.font_atlas_texture.get_texture_view(),
            )
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl InterfaceRectangleDrawer {
    fn create_bind_group_bindless(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        texture_views: &[&TextureView],
        font_atlas_texture_view: &TextureView,
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
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(font_atlas_texture_view),
                },
            ],
        })
    }

    fn create_bind_group(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        font_atlas_texture_view: &TextureView,
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
                    resource: BindingResource::TextureView(font_atlas_texture_view),
                },
            ],
        })
    }
}
