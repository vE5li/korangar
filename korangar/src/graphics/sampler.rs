use wgpu::{AddressMode, CompareFunction, Device, FilterMode, Sampler, SamplerBorderColor, SamplerDescriptor};

use crate::graphics::{Capabilities, TextureSamplerType};

pub(crate) fn create_new_sampler(
    device: &Device,
    capabilities: &Capabilities,
    label: &str,
    sampler_type: impl Into<SamplerType>,
) -> Sampler {
    match sampler_type.into() {
        SamplerType::TextureNearest => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }),
        SamplerType::TextureLinear => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }),
        SamplerType::TextureAnisotropic(anisotropy_clamp) => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp,
            ..Default::default()
        }),
        SamplerType::DepthCompare => {
            let mut descriptor = SamplerDescriptor {
                label: Some(label),
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                compare: Some(CompareFunction::Greater),
                ..Default::default()
            };

            if capabilities.supports_clamp_to_border() {
                descriptor.address_mode_u = AddressMode::ClampToBorder;
                descriptor.address_mode_v = AddressMode::ClampToBorder;
                descriptor.border_color = Some(SamplerBorderColor::Zero);
            }

            device.create_sampler(&descriptor)
        }
    }
}

pub(crate) enum SamplerType {
    TextureNearest,
    TextureLinear,
    TextureAnisotropic(u16),
    DepthCompare,
}

impl From<TextureSamplerType> for SamplerType {
    fn from(value: TextureSamplerType) -> Self {
        match value {
            TextureSamplerType::Nearest => SamplerType::TextureNearest,
            TextureSamplerType::Linear => SamplerType::TextureLinear,
            TextureSamplerType::Anisotropic(anisotropy_clamp) => SamplerType::TextureAnisotropic(anisotropy_clamp),
        }
    }
}
