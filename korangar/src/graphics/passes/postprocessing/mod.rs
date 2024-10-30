mod blitter;
#[cfg(feature = "debug")]
mod buffer;
mod effect;
mod rectangle;

pub(crate) use blitter::PostProcessingBlitterDrawer;
#[cfg(feature = "debug")]
pub(crate) use buffer::{PostProcessingBufferDrawData, PostProcessingBufferDrawer};
pub(crate) use effect::PostProcessingEffectDrawer;
pub(crate) use rectangle::{PostProcessingRectangleDrawInstruction, PostProcessingRectangleDrawer, PostProcessingRectangleLayer};
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "post processing render pass";

pub(crate) struct PostProcessingRenderPassContext {
    surface_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }>
    for PostProcessingRenderPassContext
{
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let surface_texture_format = global_context.surface_texture_format;
        Self { surface_texture_format }
    }

    fn create_pass<'encoder>(
        &mut self,
        frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::global_bind_group_layout(device)]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.surface_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
