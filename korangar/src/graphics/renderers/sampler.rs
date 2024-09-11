use wgpu::{Device, FilterMode, Sampler, SamplerDescriptor};

pub(super) enum SamplerType {
    Linear,
    LinearAnisotropic(u16),
    Nearest,
}

pub(super) fn create_new_sampler(device: &Device, label: &str, sampler_type: SamplerType) -> Sampler {
    match sampler_type {
        SamplerType::Linear => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }),
        SamplerType::LinearAnisotropic(anisotropy_clamp) => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp,
            ..Default::default()
        }),
        SamplerType::Nearest => device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }),
    }
}
