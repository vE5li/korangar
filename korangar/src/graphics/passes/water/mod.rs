mod wave;

pub(crate) use wave::WaterWaveDrawer;
use wgpu::{
    BindGroupLayout, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "water render pass";

pub(crate) struct WaterRenderPassContext {
    accumulation_texture_format: TextureFormat,
    revealage_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::Two }, { DepthAttachmentCount::None }> for WaterRenderPassContext {
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let accumulation_texture_format = global_context.forward_accumulation_texture.get_format();
        let revealage_texture_format = global_context.forward_revealage_texture.get_format();

        Self {
            accumulation_texture_format,
            revealage_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: global_context.forward_accumulation_texture.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: global_context.forward_revealage_texture.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass.set_bind_group(1, &global_context.forward_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 2] {
        [
            GlobalContext::global_bind_group_layout(device),
            GlobalContext::forward_bind_group_layout(device),
        ]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 2] {
        [self.accumulation_texture_format, self.revealage_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
