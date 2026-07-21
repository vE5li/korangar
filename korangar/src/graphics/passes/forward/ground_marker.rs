use std::num::NonZeroU64;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StencilState, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Buffer, Capabilities, GlobalContext, GroundMarkerInstruction, Prepare, RenderInstruction, Texture};

const DRAWER_NAME: &str = "forward ground marker";
const INITIAL_INSTRUCTION_SIZE: usize = 8;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct GpuGroundMarker {
    corners: [[f32; 4]; 4],
    color: [f32; 4],
    // uv_scale.xy, uv_offset.xy
    uv_transform: [f32; 4],
}

/// Emissive, depth-tested ground quads drawn at the end of the forward pass:
/// entities and models in front of a marker occlude it, while the part in
/// front of them draws over their feet. Unlit, since the markers represent
/// glowing magic.
pub(crate) struct ForwardGroundMarkerDrawer {
    instance_data_buffer: Buffer<GpuGroundMarker>,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    instance_data: Vec<GpuGroundMarker>,
    textures: Vec<Arc<Texture>>,
    bind_groups: Vec<BindGroup>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardGroundMarkerDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = &'data [GroundMarkerInstruction];

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("forward", "ground_marker");

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<GpuGroundMarker>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<GpuGroundMarker>() as _),
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
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Some(Self::Context::bind_group_layout(device)[0]),
                Some(Self::Context::bind_group_layout(device)[1]),
                Some(&bind_group_layout),
            ],
            immediate_size: 0,
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
                targets: &[
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[0],
                        // Additive: the markers are glowing magic.
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[1],
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    }),
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[2],
                        blend: None,
                        write_mask: ColorWrites::empty(),
                    }),
                ],
            }),
            primitive: PrimitiveState {
                // Double sided: spin can flip the winding.
                cull_mode: None,
                front_face: FrontFace::Ccw,
                ..Default::default()
            },
            multisample: MultisampleState {
                count: global_context.msaa.sample_count(),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                // Tested but never written: markers must be occluded by what
                // stands in front of them without ever occluding anything
                // themselves.
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Greater),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
            multiview_mask: None,
        });

        Self {
            instance_data_buffer,
            bind_group_layout,
            pipeline,
            instance_data: Vec::default(),
            textures: Vec::default(),
            bind_groups: Vec::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_data: Self::DrawData<'_>) {
        if self.bind_groups.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);

        for (index, bind_group) in self.bind_groups.iter().enumerate() {
            pass.set_bind_group(2, bind_group, &[]);
            pass.draw(0..6, index as u32..index as u32 + 1);
        }
    }
}

impl Prepare for ForwardGroundMarkerDrawer {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.instance_data.clear();
        self.textures.clear();

        for marker in instructions.ground_markers {
            let corner = |point: cgmath::Point3<f32>| [point.x, point.y, point.z, 1.0];
            self.instance_data.push(GpuGroundMarker {
                corners: [
                    corner(marker.upper_left),
                    corner(marker.upper_right),
                    corner(marker.lower_left),
                    corner(marker.lower_right),
                ],
                color: marker.color.components_linear(),
                uv_transform: [marker.uv_scale.x, marker.uv_scale.y, marker.uv_offset.x, marker.uv_offset.y],
            });
            self.textures.push(marker.texture.clone());
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.bind_groups.clear();

        if self.instance_data.is_empty() {
            return;
        }

        self.instance_data_buffer.reserve(device, self.instance_data.len());
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);

        // One bind group per marker for its texture; the storage buffer is
        // shared. Marker counts are tiny, so per-frame creation is fine.
        for texture in &self.textures {
            self.bind_groups.push(device.create_bind_group(&BindGroupDescriptor {
                label: Some(DRAWER_NAME),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.instance_data_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(texture.get_texture_view()),
                    },
                ],
            }));
        }
    }
}
