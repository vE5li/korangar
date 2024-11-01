use hashbrown::HashMap;
use wgpu::{
    include_wgsl, BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, TextureSampleType, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext, Msaa};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/blitter.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("shader/blitter_msaa.wgsl");
const DRAWER_NAME: &str = "post processing blitter";

pub(crate) struct PostProcessingBlitterDrawData<'a> {
    pub(crate) source_texture: &'a AttachmentTexture,
    pub(crate) luma_in_alpha: bool,
    pub(crate) alpha_blending: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PipelineKey {
    msaa: Msaa,
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
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let msaa_module = device.create_shader_module(SHADER_MSAA);

        let mut pipeline_cache = HashMap::new();

        for (msaa, luma_in_alpha, alpha_blending) in [
            (global_context.msaa, false, false),
            (global_context.msaa, true, false),
            (Msaa::X4, false, true),
        ] {
            let pipeline = Self::create_pipeline(
                device,
                render_pass_context,
                &shader_module,
                &msaa_module,
                msaa,
                luma_in_alpha,
                alpha_blending,
            );
            pipeline_cache.insert(
                PipelineKey {
                    msaa,
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
            msaa: draw_data.source_texture.get_texture().sample_count().into(),
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
        render_pass_context: &PostProcessingRenderPassContext,
        shader_module: &ShaderModule,
        msaa_module: &ShaderModule,
        msaa: Msaa,
        luma_in_alpha: bool,
        alpha_blending: bool,
    ) -> RenderPipeline {
        let label = format!("{DRAWER_NAME} {msaa}");

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureSampleType::Float {
                filterable: !msaa.multisampling_activated(),
            },
            msaa.multisampling_activated(),
        );

        let pass_bind_group_layouts = <Self as Drawer<
            { BindGroupCount::One },
            { ColorAttachmentCount::One },
            { DepthAttachmentCount::None },
        >>::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&label),
            bind_group_layouts: &[pass_bind_group_layouts[0], &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut constants = std::collections::HashMap::new();
        constants.insert("SAMPLE_COUNT".to_string(), f64::from(msaa.sample_count()));
        constants.insert("LUMA_IN_ALPHA".to_string(), f64::from(luma_in_alpha));

        let shader_module = match msaa.multisampling_activated() {
            true => msaa_module,
            false => shader_module,
        };

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: if alpha_blending { Some(BlendState::ALPHA_BLENDING) } else { None },
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
