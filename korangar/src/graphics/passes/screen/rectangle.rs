use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device,
    Features, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureView, TextureViewDimension,
    VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext, ScreenRenderPassContext,
};
use crate::graphics::{features_supported, Buffer, GlobalContext, Prepare, RectangleInstruction, RenderInstruction, Texture};
use crate::MAX_BINDING_TEXTURE_ARRAY_COUNT;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/rectangle.wgsl");
const DRAWER_NAME: &str = "screen rectangle";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Layer {
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

pub(crate) struct ScreenRectangleDrawer {
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

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenRectangleDrawer {
    type Context = ScreenRenderPassContext;
    type DrawData<'data> = Layer;

    fn new(device: &Device, _queue: &Queue, global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                    count: NonZeroU32::new(MAX_BINDING_TEXTURE_ARRAY_COUNT as _),
                },
            ],
        });

        let mut texture_views = vec![global_context.solid_pixel_texture.get_texture_view()];

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_views.len()) {
                texture_views.push(texture_views[0]);
            }
        }

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer, &texture_views);

        let bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[bind_group_layouts[0], bind_group_layouts[1], &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
            cache: None,
        });

        Self {
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
        let batch = self.layer_batches[draw_data as usize];

        if batch.count == 0 {
            return;
        }
        let offset = batch.offset as u32;
        let count = batch.count as u32;
        let end = offset + count;

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..6, offset..end);
    }
}

impl Prepare for ScreenRectangleDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        let draw_count = instructions.bottom_layer_rectangles.len()
            + instructions.middle_layer_rectangles.len()
            + instructions.top_layer_rectangles.len();

        if draw_count == 0 {
            return;
        }

        self.instance_data.clear();
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
                            texture_index: 0,
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
                        let mut texture_index = (texture_views.len() + 1) as i32;
                        let id = texture.get_texture().global_id().inner();
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

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_views.len()) {
                texture_views.push(texture_views[0]);
            }
        }

        self.instance_data_buffer.reserve(device, self.instance_data.len());
        self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views);
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl ScreenRectangleDrawer {
    fn create_bind_group(
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
}
