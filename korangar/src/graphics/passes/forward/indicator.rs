use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
    DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, TextureSampleType,
    TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Capabilities, GlobalContext, IndicatorInstruction};

const DRAWER_NAME: &str = "forward indicator";

pub(crate) struct ForwardIndicatorDrawer {
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for ForwardIndicatorDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = Option<&'data IndicatorInstruction>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("forward", "indicator");

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(global_context.walk_indicator_texture.get_texture_view()),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Self::Context::bind_group_layout(device)[0],
                Self::Context::bind_group_layout(device)[1],
                &bind_group_layout,
            ],
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
                targets: &[
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[0],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[1],
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
                        write_mask: ColorWrites::empty(),
                    }),
                    Some(ColorTargetState {
                        format: render_pass_context.color_attachment_formats()[2],
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrcAlpha,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::Zero,
                                dst_factor: BlendFactor::OneMinusSrcAlpha,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::empty(),
                    }),
                ],
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
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self { bind_group, pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, instruction: Self::DrawData<'_>) {
        if instruction.is_some() {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(2, &self.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}
