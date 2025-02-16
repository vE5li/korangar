use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderStages, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::water::WaterRenderPassContext;
use crate::graphics::passes::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext};
use crate::graphics::{AttachmentTexture, Buffer, Capabilities, GlobalContext, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/wave.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("shader/wave_msaa.wgsl");
const DRAWER_NAME: &str = "water wave";

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
struct WaterWaveUniforms {
    water_bounds: [f32; 4],
    texture_repeat: f32,
    water_level: f32,
    wave_amplitude: f32,
    wave_speed: f32,
    wave_length: f32,
    water_opacity: f32,
    padding: [u32; 2],
}

pub(crate) struct WaterWaveDrawer {
    uniforms: WaterWaveUniforms,
    uniforms_buffer: Buffer<WaterWaveUniforms>,
    bind_group: BindGroup,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::Two }, { DepthAttachmentCount::None }> for WaterWaveDrawer {
    type Context = WaterRenderPassContext;
    type DrawData<'data> = &'data AttachmentTexture;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = if global_context.msaa.multisampling_activated() {
            device.create_shader_module(SHADER_MSAA)
        } else {
            device.create_shader_module(SHADER)
        };

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
                    visibility: ShaderStages::FRAGMENT,
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
            bind_group_layouts: &[
                pass_bind_group_layouts[0],
                pass_bind_group_layouts[1],
                &bind_group_layout,
                &AttachmentTexture::bind_group_layout(device, TextureSampleType::Depth, global_context.msaa.multisampling_activated()),
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
                        format: render_pass_context.color_attachment_formats()[1],
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
            depth_stencil: None,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.set_bind_group(3, draw_data.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Prepare for WaterWaveDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        if let Some(instruction) = instructions.water.as_ref() {
            self.uniforms = WaterWaveUniforms {
                water_bounds: instruction.water_bounds.into(),
                texture_repeat: instruction.texture_repeat,
                water_level: instruction.water_level,
                wave_amplitude: instruction.wave_amplitude,
                wave_speed: instruction.wave_speed,
                wave_length: instruction.wave_length,
                water_opacity: instruction.water_opacity,
                padding: Default::default(),
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
