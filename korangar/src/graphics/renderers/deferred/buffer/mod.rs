use std::num::NonZeroU32;
use std::sync::Arc;

use bytemuck::checked::cast_slice;
use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::renderers::texture::CubeTexture;
use super::{DeferredRenderer, DeferredSubRenderer, RenderSettings, Renderer, Texture};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};

const SHADER: ShaderModuleDescriptor = include_wgsl!("buffer.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    show_diffuse_buffer: u32,
    show_normal_buffer: u32,
    show_water_buffer: u32,
    show_depth_buffer: u32,
    show_picker_texture: u32,
    show_shadow_texture: u32,
    show_font_atlas: u32,
    show_point_shadow: u32,
}

pub struct BufferRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    nearest_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl BufferRenderer {
    pub fn new(device: Arc<Device>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let nearest_sampler = create_new_sampler(&device, "buffer nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("buffer"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(6),
                },
            ],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, surface_format);

        Self {
            device,
            shader_module,
            nearest_sampler,
            bind_group_layout,
            pipeline,
        }
    }

    #[korangar_debug::profile]
    pub fn recreate_pipeline(&mut self, surface_format: TextureFormat) {
        self.pipeline = Self::create_pipeline(&self.device, &self.shader_module, &self.bind_group_layout, surface_format);
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        surface_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("buffer"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("buffer"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[korangar_debug::profile]
    fn bind_pipeline(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        picker_texture: &Texture,
        shadow_map: &Texture,
        font_atlas: &Texture,
        point_shadow: &CubeTexture,
    ) {
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("buffer"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(render_target.diffuse_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(render_target.normal_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(render_target.water_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(render_target.depth_buffer.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(picker_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(shadow_map.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(font_atlas.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::Sampler(&self.nearest_sampler),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(point_shadow.get_texture_array_view()),
                },
            ],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[korangar_debug::profile("render buffers")]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        picker_texture: &Texture,
        shadow_map: &Texture,
        font_atlas: &Texture,
        point_shadow: &CubeTexture,
        render_settings: &RenderSettings,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Buffers) {
            self.bind_pipeline(render_target, render_pass, picker_texture, shadow_map, font_atlas, point_shadow);
        }

        let push_constants = Constants {
            show_diffuse_buffer: render_settings.show_diffuse_buffer as u32,
            show_normal_buffer: render_settings.show_normal_buffer as u32,
            show_water_buffer: render_settings.show_water_buffer as u32,
            show_depth_buffer: render_settings.show_depth_buffer as u32,
            show_picker_texture: render_settings.show_picker_buffer as u32,
            show_shadow_texture: render_settings.show_shadow_buffer as u32,
            show_font_atlas: render_settings.show_font_atlas as u32,
            show_point_shadow: render_settings.show_point_shadow as u32,
        };

        render_pass.set_push_constants(ShaderStages::FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..3, 0..1);
    }
}
