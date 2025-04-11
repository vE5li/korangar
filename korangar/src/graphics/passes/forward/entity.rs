use std::num::NonZeroU64;
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, StencilState, TextureFormat, TextureSampleType,
    TextureView, TextureViewDimension, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::{BindlessSupport, Buffer, Capabilities, EntityInstruction, GlobalContext, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/entity.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/entity_bindless.wgsl");
const DRAWER_NAME: &str = "forward entity";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct InstanceData {
    world: [[f32; 4]; 4],
    frame_part_transform: [[f32; 4]; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    color: [f32; 4],
    frame_size: [f32; 2],
    extra_depth_offset: f32,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
    padding: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityPassMode {
    /// Draws all opaque parts of an entity.
    Opaque = 0,
    /// Draws all transparent parts of an entity.
    Transparent = 1,
}

pub(crate) struct ForwardEntityDrawData<'a> {
    pub(crate) entities: &'a [EntityInstruction],
    pub(crate) pass_mode: EntityPassMode,
}

pub(crate) struct ForwardEntityDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    opaque_pipeline: RenderPipeline,
    transparent_pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardEntityDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = ForwardEntityDrawData<'data>;

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

        let bind_group = if capabilities.bindless_support() == BindlessSupport::Full {
            Self::create_bind_group_bindless(device, &bind_group_layout, &instance_data_buffer, &[global_context
                .solid_pixel_texture
                .get_texture_view()])
        } else {
            Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer)
        };

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group_layouts: &[&BindGroupLayout] = if capabilities.bindless_support() == BindlessSupport::Full {
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

        let color_attachment_formats = render_pass_context.color_attachment_formats();

        let opaque_pipeline = Self::create_pipeline(
            device,
            global_context,
            render_pass_context,
            &shader_module,
            &pipeline_layout,
            &color_attachment_formats,
            EntityPassMode::Opaque,
        );

        let wboit_pipeline = Self::create_pipeline(
            device,
            global_context,
            render_pass_context,
            &shader_module,
            &pipeline_layout,
            &color_attachment_formats,
            EntityPassMode::Transparent,
        );

        Self {
            bindless_support: capabilities.bindless_support() == BindlessSupport::Full,
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            opaque_pipeline,
            transparent_pipeline: wboit_pipeline,
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

        match draw_data.pass_mode {
            EntityPassMode::Opaque => pass.set_pipeline(&self.opaque_pipeline),
            EntityPassMode::Transparent => pass.set_pipeline(&self.transparent_pipeline),
        }

        pass.set_bind_group(2, &self.bind_group, &[]);

        if self.bindless_support {
            pass.draw(0..6, 0..self.draw_count as u32);
        } else {
            let mut current_texture_id = self.solid_pixel_texture.get_id();
            pass.set_bind_group(3, self.solid_pixel_texture.get_bind_group(), &[]);

            for (index, instruction) in draw_data.entities[0..self.draw_count].iter().enumerate() {
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

impl Prepare for ForwardEntityDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.entities.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();
            let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

            for instruction in instructions.entities.iter() {
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
                    color: instruction.color.components_linear(),
                    frame_size: instruction.frame_size.into(),
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
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
            for instruction in instructions.entities.iter() {
                self.instance_data.push(InstanceData {
                    world: instruction.world.into(),
                    frame_part_transform: instruction.frame_part_transform.into(),
                    texture_position: instruction.texture_position.into(),
                    texture_size: instruction.texture_size.into(),
                    color: instruction.color.components_linear(),
                    frame_size: instruction.frame_size.into(),
                    extra_depth_offset: instruction.extra_depth_offset,
                    depth_offset: instruction.depth_offset,
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

impl ForwardEntityDrawer {
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

    fn create_pipeline(
        device: &Device,
        global_context: &GlobalContext,
        render_pass_context: &ForwardRenderPassContext,
        shader_module: &ShaderModule,
        pipeline_layout: &PipelineLayout,
        color_attachment_formats: &[TextureFormat; 3],
        pass_mode: EntityPassMode,
    ) -> RenderPipeline {
        let targets = match pass_mode {
            EntityPassMode::Opaque => [
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
            EntityPassMode::Transparent => [
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

        let constants = &[("PASS_MODE", f64::from(pass_mode as u32))];

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
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: if pass_mode == EntityPassMode::Opaque {
                    Some("opaque_main")
                } else {
                    Some("transparent_main")
                },
                compilation_options: PipelineCompilationOptions {
                    constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &targets,
            }),
            multiview: None,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                ..Default::default()
            },
            multisample: MultisampleState {
                count: global_context.msaa.sample_count(),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: pass_mode == EntityPassMode::Opaque,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        })
    }
}
