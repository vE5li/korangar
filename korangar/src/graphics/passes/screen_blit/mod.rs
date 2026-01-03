mod blitter;

pub(crate) use blitter::ScreenBlitBlitterDrawer;
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "screen blit render pass";

pub(crate) struct ScreenBlitRenderPassContext {
    surface_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }>
    for ScreenBlitRenderPassContext
{
    type PassData<'data> = &'data TextureView;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        Self {
            surface_texture_format: global_context.surface_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        _global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: pass_data,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }

    fn bind_group_layout(_device: &Device) -> [&'static BindGroupLayout; 0] {
        []
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.surface_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
