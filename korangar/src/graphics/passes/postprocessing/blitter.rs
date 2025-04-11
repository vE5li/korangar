use hashbrown::HashMap;
use wgpu::{
    BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, TextureFormat, TextureSampleType, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::{AttachmentTexture, Capabilities, FXAA_COLOR_LUMA_TEXTURE_FORMAT, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/blitter.wgsl");
const DRAWER_NAME: &str = "post processing blitter";

pub(crate) struct PostProcessingBlitterDrawData<'a> {
    pub(crate) target_texture_format: TextureFormat,
    pub(crate) source_texture: &'a AttachmentTexture,
    pub(crate) luma_in_alpha: bool,
    pub(crate) alpha_blending: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PipelineKey {
    format: TextureFormat,
    luma_in_alpha: bool,
    alpha_blending: bool,
}

pub(crate) struct PostProcessingBlitterDrawer {
    pipeline_cache: HashMap<PipelineKey, RenderPipeline>,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingBlitterDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = PostProcessingBlitterDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let mut pipeline_cache = HashMap::new();

        let color_texture_format = render_pass_context.color_attachment_formats()[0];

        let modes = vec![
            (color_texture_format, false, false),
            (color_texture_format, false, true),
            (FXAA_COLOR_LUMA_TEXTURE_FORMAT, true, false),
        ];

        for (format, luma_in_alpha, alpha_blending) in modes {
            let pipeline = Self::create_pipeline(device, format, &shader_module, luma_in_alpha, alpha_blending);
            pipeline_cache.insert(
                PipelineKey {
                    format,
                    luma_in_alpha,
                    alpha_blending,
                },
                pipeline,
            );
        }

        Self { pipeline_cache }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        let key = PipelineKey {
            format: draw_data.target_texture_format,
            luma_in_alpha: draw_data.luma_in_alpha,
            alpha_blending: draw_data.alpha_blending,
        };
        let pipeline = self.pipeline_cache.get(&key).unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, draw_data.source_texture.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}

impl PostProcessingBlitterDrawer {
    fn create_pipeline(
        device: &Device,
        color_texture_format: TextureFormat,
        shader_module: &ShaderModule,
        luma_in_alpha: bool,
        alpha_blending: bool,
    ) -> RenderPipeline {
        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(device, TextureSampleType::Float { filterable: true }, false);

        let pass_bind_group_layouts = <Self as Drawer<
            { BindGroupCount::One },
            { ColorAttachmentCount::One },
            { DepthAttachmentCount::None },
        >>::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let constants = &[("LUMA_IN_ALPHA", f64::from(luma_in_alpha))];

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                targets: &[Some(ColorTargetState {
                    format: color_texture_format,
                    blend: if alpha_blending {
                        Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                    } else {
                        None
                    },
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }
}
