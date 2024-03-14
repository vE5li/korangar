use std::sync::Arc;

use derive_new::new;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageUsage, SampleCount};
use vulkano::memory::allocator::AllocationCreateInfo;

use crate::graphics::MemoryAllocator;

pub(super) enum AttachmentImageType {
    InputColor,
    CopyColor,
    InputDepth,
    Depth,
}

#[derive(new)]
pub(super) struct AttachmentImageFactory<'a> {
    memory_allocator: &'a MemoryAllocator,
    dimensions: [u32; 2],
    sample_count: SampleCount,
}

impl<'a> AttachmentImageFactory<'a> {
    pub(super) fn new_image(&self, format: Format, attachment_image_type: AttachmentImageType) -> Arc<ImageView> {
        let usage = match attachment_image_type {
            AttachmentImageType::InputColor => ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT | ImageUsage::INPUT_ATTACHMENT,
            AttachmentImageType::CopyColor => ImageUsage::SAMPLED | ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
            AttachmentImageType::InputDepth => ImageUsage::SAMPLED | ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::INPUT_ATTACHMENT,
            AttachmentImageType::Depth => ImageUsage::DEPTH_STENCIL_ATTACHMENT,
        };

        let image = Image::new(
            self.memory_allocator,
            ImageCreateInfo {
                format,
                extent: [self.dimensions[0], self.dimensions[1], 1],
                samples: self.sample_count,
                usage,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        ImageView::new_default(image).unwrap()
    }
}
