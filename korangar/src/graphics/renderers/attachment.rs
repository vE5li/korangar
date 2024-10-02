use derive_new::new;
use wgpu::{Device, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use crate::graphics::Texture;
use crate::interface::layout::ScreenSize;

pub(super) enum AttachmentImageType {
    InputColor,
    CopyColor,
    InputDepth,
    Depth,
}

#[derive(new)]
pub(super) struct AttachmentTextureFactory<'a> {
    target_name: &'a str,
    device: &'a Device,
    dimensions: ScreenSize,
    sample_count: u32,
}

impl<'a> AttachmentTextureFactory<'a> {
    pub(super) fn new_texture(&self, texture_name: &str, format: TextureFormat, attachment_image_type: AttachmentImageType) -> Texture {
        let usage = match attachment_image_type {
            AttachmentImageType::InputColor => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            AttachmentImageType::CopyColor => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            AttachmentImageType::InputDepth => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            AttachmentImageType::Depth => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        };

        Texture::new(self.device, &TextureDescriptor {
            label: Some(&format!("{} {}", self.target_name, texture_name)),
            size: Extent3d {
                width: self.dimensions.width as u32,
                height: self.dimensions.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    }
}
