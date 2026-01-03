mod blitter;
#[cfg(feature = "debug")]
mod debug_aabb;
#[cfg(feature = "debug")]
mod debug_buffer;
#[cfg(feature = "debug")]
mod debug_circle;
#[cfg(feature = "debug")]
mod debug_rectangle;
mod effect;
mod fxaa;
mod rectangle;
mod wboit_resolve;

pub(crate) use blitter::{PostProcessingBlitterDrawData, PostProcessingBlitterDrawer};
#[cfg(feature = "debug")]
pub(crate) use debug_aabb::DebugAabbDrawer;
#[cfg(feature = "debug")]
pub(crate) use debug_buffer::{DebugBufferDrawData, DebugBufferDrawer};
#[cfg(feature = "debug")]
pub(crate) use debug_circle::DebugCircleDrawer;
#[cfg(feature = "debug")]
pub(crate) use debug_rectangle::DebugRectangleDrawer;
pub(crate) use effect::PostProcessingEffectDrawer;
pub(crate) use fxaa::PostProcessingFxaaDrawer;
pub(crate) use rectangle::{PostProcessingRectangleDrawData, PostProcessingRectangleDrawer, PostProcessingRectangleLayer};
pub(crate) use wboit_resolve::{PostProcessingWboitResolveDrawData, PostProcessingWboitResolveDrawer};
use wgpu::{
    BindGroupLayout, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{AttachmentTexture, GlobalContext};
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "post processing render pass";

pub(crate) struct PostProcessingRenderPassContext {
    color_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }>
    for PostProcessingRenderPassContext
{
    type PassData<'data> = &'data AttachmentTexture;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let color_texture_format = global_context
            .resolved_color_texture
            .as_ref()
            .unwrap_or(&global_context.forward_color_texture)
            .get_format();

        Self { color_texture_format }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: pass_data.get_texture_view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::global_bind_group_layout(device)]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.color_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
