use wgpu::{Device, FilterMode, Sampler, SamplerDescriptor};

use crate::graphics::TextureSamplerType;

pub(crate) fn create_new_sampler(device: &Device, label: &str, sampler_type: TextureSamplerType) -> Sampler {
    match sampler_type {
        TextureSamplerType::Nearest => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }),
        TextureSamplerType::Linear => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }),
        TextureSamplerType::Anisotropic(anisotropy_clamp) => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp,
            ..Default::default()
        }),
    }
}
