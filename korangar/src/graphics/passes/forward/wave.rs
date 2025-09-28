use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StencilState, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, ForwardRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Buffer, Capabilities, GlobalContext, Prepare, RenderInstruction, Texture, WaterInstruction, WaterVertex};

const DRAWER_NAME: &str = "water wave";

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
struct WaterWaveUniforms {
    texture_repeat_rcp: f32,
    waveform_phase_shift: f32,
    waveform_amplitude: f32,
    waveform_frequency: f32,
    water_opacity: f32,
}

pub(crate) struct WaterWaveDrawer {
    uniforms: WaterWaveUniforms,
    uniforms_buffer: Buffer<WaterWaveUniforms>,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for WaterWaveDrawer {
    type Context = ForwardRenderPassContext;
    type DrawData<'data> = &'data WaterInstruction<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("forward", "wave");

        let uniforms_buffer = Buffer::with_data(
            device,
            queue,
            format!("{DRAWER_NAME} uniforms"),
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            &[WaterWaveUniforms::default()],
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<WaterWaveUniforms>() as _),
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

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &uniforms_buffer,
            &global_context.solid_pixel_texture,
        );

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], pass_bind_group_layouts[1], &bind_group_layout],
            push_constant_ranges: &[],
        });

        let color_attachment_formats = render_pass_context.color_attachment_formats();
        let depth_attachment_formats = render_pass_context.depth_attachment_output_format();

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[WaterVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[
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
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState {
                count: global_context.msaa.sample_count(),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: depth_attachment_formats[0],
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self {
            uniforms: WaterWaveUniforms::default(),
            uniforms_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.water_index_buffer.count() == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.set_index_buffer(draw_data.water_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, draw_data.water_vertex_buffer.slice(..));
        pass.draw_indexed(0..draw_data.water_index_buffer.count(), 0, 0..1);
    }
}

impl Prepare for WaterWaveDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        if let Some(instruction) = instructions.water.as_ref()
            && instruction.water_index_buffer.count() != 0
        {
            self.uniforms = WaterWaveUniforms {
                texture_repeat_rcp: 1.0 / instruction.texture_repeat,
                waveform_phase_shift: instruction.waveform_phase_shift,
                waveform_amplitude: instruction.waveform_amplitude,
                waveform_frequency: instruction.waveform_frequency.0,
                water_opacity: instruction.water_opacity,
            };
            self.bind_group = Self::create_bind_group(
                device,
                &self.bind_group_layout,
                &self.uniforms_buffer,
                instruction.water_texture,
            );
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.uniforms_buffer.write(device, staging_belt, command_encoder, &[self.uniforms]);
    }
}

impl WaterWaveDrawer {
    fn create_bind_group(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        uniform_buffer: &Buffer<WaterWaveUniforms>,
        water_texture: &Texture,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(water_texture.get_texture_view()),
                },
            ],
        })
    }
}
