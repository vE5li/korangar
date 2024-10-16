#[cfg(feature = "debug")]
mod aabb;
mod ambient_light;
#[cfg(feature = "debug")]
mod buffer;
#[cfg(feature = "debug")]
mod circle;
mod directional_light;
mod effect;
mod overlay;
mod point_light;
mod rectangle;
mod water_light;

#[cfg(feature = "debug")]
pub(crate) use aabb::ScreenAabbDrawer;
pub(crate) use ambient_light::ScreenAmbientLightDrawer;
#[cfg(feature = "debug")]
pub(crate) use buffer::ScreenBufferDrawer;
#[cfg(feature = "debug")]
pub(crate) use circle::ScreenCircleDrawer;
pub(crate) use directional_light::ScreenDirectionalLightDrawer;
pub(crate) use effect::ScreenEffectDrawer;
pub(crate) use overlay::ScreenOverlayDrawer;
pub(crate) use point_light::ScreenPointLightDrawer;
pub(crate) use rectangle::{Layer, ScreenRectangleDrawer};
pub(crate) use water_light::ScreenWaterLightDrawer;
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "screen render pass";

pub(crate) struct ScreenRenderPassContext {
    screen_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenRenderPassContext {
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let screen_texture_format = global_context.surface_texture_format;

        Self { screen_texture_format }
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
        pass.set_bind_group(1, &global_context.screen_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 2] {
        [
            GlobalContext::global_bind_group_layout(device),
            GlobalContext::screen_bind_group_layout(device),
        ]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.screen_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
