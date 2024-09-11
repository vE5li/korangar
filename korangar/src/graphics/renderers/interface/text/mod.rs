use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::InterfaceSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, FontSize};

const SHADER: ShaderModuleDescriptor = include_wgsl!("text.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    screen_clip: [f32; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    color: [f32; 4],
}

pub struct TextRenderer {
    device: Arc<Device>,
    font_loader: Rc<RefCell<FontLoader>>,
    linear_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl TextRenderer {
    pub fn new(device: Arc<Device>, output_texture_format: TextureFormat, font_loader: Rc<RefCell<FontLoader>>) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let linear_sampler = create_new_sampler(&device, "text linear", SamplerType::Linear);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("text"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, output_texture_format);

        Self {
            device,
            font_loader,
            linear_sampler,
            bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_texture_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: output_texture_format,
                    blend: Some(INTERFACE_ATTACHMENT_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render text"))]
    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        text: &str,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_clip: ScreenClip,
        color: Color,
        font_size: FontSize,
    ) -> f32 {
        if render_target.bind_sub_renderer(InterfaceSubRenderer::Text) {
            self.bind_pipeline(render_pass);
        }

        let mut font_loader = self.font_loader.borrow_mut();
        let (character_layout, height) = font_loader.get(text, color, font_size, screen_clip.right - screen_position.left);
        let texture = font_loader.get_font_atlas();

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry uniforms"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });
        render_pass.set_bind_group(0, &bind_group, &[]);

        character_layout.iter().for_each(|(texture_coordinates, position, color)| {
            let screen_position = ScreenPosition {
                left: screen_position.left + position.min.x as f32,
                top: screen_position.top + position.min.y as f32,
            } / window_size;

            let screen_size = ScreenSize {
                width: position.width() as f32,
                height: position.height() as f32,
            } / window_size;

            let texture_position = texture_coordinates.min;
            let texture_size = texture_coordinates.max - texture_coordinates.min; // TODO: use absolute instead

            let push_constants = Constants {
                screen_position: screen_position.into(),
                screen_size: screen_size.into(),
                screen_clip: screen_clip.into(),
                texture_position: [texture_position.x, texture_position.y],
                texture_size: [texture_size.x, texture_size.y],
                color: color.components_linear(),
            };

            render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
            render_pass.draw(0..6, 0..1);
        });

        height
    }
}
