use wgpu::{CompareFunction, Device, FilterMode, Sampler, SamplerDescriptor};

use crate::graphics::TextureSamplerType;

pub(crate) fn create_new_sampler(device: &Device, label: &str, sampler_type: impl Into<SamplerType>) -> Sampler {
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
        SamplerType::DepthCompare => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            compare: Some(CompareFunction::Greater),
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            ..Default::default()
        }),
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
