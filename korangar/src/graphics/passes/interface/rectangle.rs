use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device,
    Features, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureView, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, InterfaceRenderPassContext, RenderPassContext,
};
use crate::graphics::{features_supported, Buffer, GlobalContext, InterfaceRectangleInstruction, Prepare, RenderInstruction, Texture};
use crate::MAX_BINDING_TEXTURE_ARRAY_COUNT;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/rectangle.wgsl");
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
    aspect_ratio: f32,
    rectangle_type: u32,
    texture_index: i32,
    smooth: u32,
}

pub(crate) struct InterfaceRectangleDrawer {
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
    type DrawData<'data> = Option<()>;

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
        });

        let mut texture_views = vec![global_context.solid_pixel_texture.get_texture_view()];

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_views.len()) {
                texture_views.push(texture_views[0]);
            }
        }

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &instance_data_buffer,
            &texture_views,
            global_context.solid_pixel_texture.get_texture_view(),
        );

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[Self::Context::bind_group_layout(device)[0], &bind_group_layout],
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
            primitive: Default::default(),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: None,
            cache: None,
        });

        Self {
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

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_dara: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.draw(0..6, 0..self.draw_count as u32);
    }
}

impl Prepare for InterfaceRectangleDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.interface.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();
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
                    aspect_ratio,
                } => {
                    self.instance_data.push(InstanceData {
                        color: color.components_linear(),
                        corner_radius: (*corner_radius).into(),
                        screen_clip: (*screen_clip).into(),
                        screen_position: (*screen_position).into(),
                        screen_size: (*screen_size).into(),
                        texture_position: [0.0, 0.0],
                        texture_size: [1.0, 1.0],
                        aspect_ratio: *aspect_ratio,
                        rectangle_type: 0,
                        texture_index: 0,
                        smooth: 0,
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
                        corner_radius: [0.0, 0.0, 0.0, 0.0],
                        screen_clip: (*screen_clip).into(),
                        screen_position: (*screen_position).into(),
                        screen_size: (*screen_size).into(),
                        texture_position: [0.0, 0.0],
                        texture_size: [1.0, 1.0],
                        aspect_ratio: 0.0,
                        rectangle_type: 1,
                        texture_index,
                        smooth: *smooth as u32,
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
                        aspect_ratio: 0.0,
                        rectangle_type: 2,
                        texture_index: 0,
                        smooth: 1,
                    });
                }
            }

            if texture_views.is_empty() {
                texture_views.push(self.solid_pixel_texture.get_texture_view());
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
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.instance_data_buffer,
            &texture_views,
            instructions.font_atlas_texture.get_texture_view(),
        )
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl InterfaceRectangleDrawer {
    fn create_bind_group(
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
}
