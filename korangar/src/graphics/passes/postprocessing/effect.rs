use std::num::NonZeroU64;
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, EffectInstruction, GlobalContext, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/effect.wgsl");
const SHADER_BINDLESS: ShaderModuleDescriptor = include_wgsl!("shader/effect_bindless.wgsl");
const DRAWER_NAME: &str = "post processing effect";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

/// These are the TOP5 combinations we currently find in the korean client
/// files and will preload at start.
const PRELOAD_PIPELINES: &[(BlendFactor, BlendFactor)] = &[
    (BlendFactor::SrcAlpha, BlendFactor::DstAlpha),
    (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
    (BlendFactor::One, BlendFactor::Zero),
    (BlendFactor::Zero, BlendFactor::OneMinusSrcAlpha),
    (BlendFactor::OneMinusSrcAlpha, BlendFactor::OneMinusSrcAlpha),
];

struct EffectBatch {
    offset: usize,
    count: usize,
    blend_state: (BlendFactor, BlendFactor),
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct InstanceData {
    top_left: [f32; 2],
    bottom_left: [f32; 2],
    top_right: [f32; 2],
    bottom_right: [f32; 2],
    texture_top_left: [f32; 2],
    texture_bottom_left: [f32; 2],
    texture_top_right: [f32; 2],
    texture_bottom_right: [f32; 2],
    // Needs to be stored in two arrays,
    // or else we get alignment problems.
    color0: [f32; 2],
    color1: [f32; 2],
    texture_index: i32,
    padding: u32,
}

pub(crate) struct PostProcessingEffectDrawer {
    bindless_support: bool,
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    shader_module: ShaderModule,
    pipeline_layout: PipelineLayout,
    color_attachment_format: TextureFormat,
    pipelines: HashMap<(BlendFactor, BlendFactor), RenderPipeline>,
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
    batches: Vec<EffectBatch>,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingEffectDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data [EffectInstruction];

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let color_attachment_format = render_pass_context.color_attachment_formats()[0];

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
            &[pass_bind_group_layouts[0], &bind_group_layout]
        } else {
            &[pass_bind_group_layouts[0], &bind_group_layout, Texture::bind_group_layout(device)]
        };

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let mut pipelines = HashMap::with_capacity(PRELOAD_PIPELINES.len());
        for (source_blend_factor, destination_blend_factor) in PRELOAD_PIPELINES.iter().copied() {
            let pipeline = Self::create_pipeline(
                device,
                &shader_module,
                &pipeline_layout,
                color_attachment_format,
                source_blend_factor,
                destination_blend_factor,
            );
            pipelines.insert((source_blend_factor, destination_blend_factor), pipeline);
        }

        Self {
            bindless_support: capabilities.supports_bindless(),
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            shader_module,
            pipeline_layout,
            color_attachment_format,
            pipelines,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
            batches: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if self.batches.is_empty() {
            return;
        }

        pass.set_bind_group(1, &self.bind_group, &[]);

        for batch in self.batches.drain(..) {
            if let Some(pipeline) = self.pipelines.get(&batch.blend_state) {
                pass.set_pipeline(pipeline);

                let start = batch.offset as u32;
                let end = start + batch.count as u32;

                if self.bindless_support {
                    pass.draw(0..6, start..end);
                } else {
                    let mut current_texture_id = self.solid_pixel_texture.get_id();
                    pass.set_bind_group(2, self.solid_pixel_texture.get_bind_group(), &[]);

                    for (index, instruction) in draw_data[start as usize..end as usize].iter().enumerate() {
                        if instruction.texture.get_id() != current_texture_id {
                            current_texture_id = instruction.texture.get_id();
                            pass.set_bind_group(2, instruction.texture.get_bind_group(), &[]);
                        }
                        let index = start + index as u32;

                        pass.draw(0..6, index..index + 1);
                    }
                }
            }
        }
    }
}

impl Prepare for PostProcessingEffectDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.effects.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();
        self.batches.clear();

        let first_effect = &instructions.effects[0];
        let mut blend_state = (first_effect.source_blend_factor, first_effect.destination_blend_factor);
        let mut offset = 0;

        if self.bindless_support {
            self.bump.reset();
            self.lookup.clear();

            let mut texture_views = Vec::with_capacity_in(draw_count, &self.bump);

            for (index, instruction) in instructions.effects.iter().enumerate() {
                let effect_blend_state = (instruction.source_blend_factor, instruction.destination_blend_factor);

                if effect_blend_state != blend_state {
                    Self::push_effect_batch(
                        device,
                        &mut self.pipelines,
                        &mut self.batches,
                        &self.shader_module,
                        self.color_attachment_format,
                        &self.pipeline_layout,
                        blend_state,
                        self.instance_data.len() - offset,
                        offset,
                    );

                    blend_state = effect_blend_state;
                    offset = index;
                }

                let mut texture_index = texture_views.len() as i32;
                let id = instruction.texture.get_id();
                let potential_index = self.lookup.get(&id);

                if let Some(potential_index) = potential_index {
                    texture_index = *potential_index;
                } else {
                    self.lookup.insert(id, texture_index);
                    texture_views.push(instruction.texture.get_texture_view());
                }

                let color = instruction.color.components_linear();
                self.instance_data.push(InstanceData {
                    top_left: instruction.top_left.into(),
                    bottom_left: instruction.bottom_left.into(),
                    top_right: instruction.top_right.into(),
                    bottom_right: instruction.bottom_right.into(),
                    texture_top_left: instruction.texture_top_left.into(),
                    texture_bottom_left: instruction.texture_bottom_left.into(),
                    texture_top_right: instruction.texture_top_right.into(),
                    texture_bottom_right: instruction.texture_bottom_right.into(),
                    color0: [color[0], color[1]],
                    color1: [color[2], color[3]],
                    texture_index,
                    padding: 0,
                });
            }

            Self::push_effect_batch(
                device,
                &mut self.pipelines,
                &mut self.batches,
                &self.shader_module,
                self.color_attachment_format,
                &self.pipeline_layout,
                blend_state,
                self.instance_data.len() - offset,
                offset,
            );

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
            }

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group_bindless(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
        } else {
            for (index, instruction) in instructions.effects.iter().enumerate() {
                let effect_blend_state = (instruction.source_blend_factor, instruction.destination_blend_factor);

                if effect_blend_state != blend_state {
                    Self::push_effect_batch(
                        device,
                        &mut self.pipelines,
                        &mut self.batches,
                        &self.shader_module,
                        self.color_attachment_format,
                        &self.pipeline_layout,
                        blend_state,
                        self.instance_data.len() - offset,
                        offset,
                    );

                    blend_state = effect_blend_state;
                    offset = index;
                }

                let color = instruction.color.components_linear();
                self.instance_data.push(InstanceData {
                    top_left: instruction.top_left.into(),
                    bottom_left: instruction.bottom_left.into(),
                    top_right: instruction.top_right.into(),
                    bottom_right: instruction.bottom_right.into(),
                    texture_top_left: instruction.texture_top_left.into(),
                    texture_bottom_left: instruction.texture_bottom_left.into(),
                    texture_top_right: instruction.texture_top_right.into(),
                    texture_bottom_right: instruction.texture_bottom_right.into(),
                    color0: [color[0], color[1]],
                    color1: [color[2], color[3]],
                    texture_index: 0,
                    padding: 0,
                });
            }

            Self::push_effect_batch(
                device,
                &mut self.pipelines,
                &mut self.batches,
                &self.shader_module,
                self.color_attachment_format,
                &self.pipeline_layout,
                blend_state,
                self.instance_data.len() - offset,
                offset,
            );

            self.instance_data_buffer.reserve(device, self.instance_data.len());
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer)
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl PostProcessingEffectDrawer {
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
        shader_module: &ShaderModule,
        pipeline_layout: &PipelineLayout,
        color_attachment_format: TextureFormat,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) -> RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: color_attachment_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: source_blend_factor,
                            dst_factor: destination_blend_factor,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: source_blend_factor,
                            dst_factor: destination_blend_factor,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        })
    }

    fn push_effect_batch(
        device: &Device,
        pipelines: &mut HashMap<(BlendFactor, BlendFactor), RenderPipeline>,
        batches: &mut Vec<EffectBatch>,
        shader_module: &ShaderModule,
        color_attachment_format: TextureFormat,
        pipeline_layout: &PipelineLayout,
        blend_state: (BlendFactor, BlendFactor),
        count: usize,
        offset: usize,
    ) {
        if !pipelines.contains_key(&blend_state) {
            let pipeline = Self::create_pipeline(
                device,
                shader_module,
                pipeline_layout,
                color_attachment_format,
                blend_state.0,
                blend_state.1,
            );
            pipelines.insert(blend_state, pipeline);
        }

        batches.push(EffectBatch {
            offset,
            count,
            blend_state,
        });
    }
}
