use std::sync::Arc;

use vulkano::device::Device;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};

pub(super) enum SamplerType {
    Linear,
    LinearAnisotropic(f32),
    Nearest,
}

pub(super) fn create_new_sampler(device: &Arc<Device>, sampler_type: SamplerType) -> Arc<Sampler> {
    match sampler_type {
        SamplerType::Linear => Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            ..Default::default()
        })
        .unwrap(),
        SamplerType::LinearAnisotropic(anisotropy) => Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            anisotropy: Some(anisotropy),
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            ..Default::default()
        })
        .unwrap(),
        SamplerType::Nearest => Sampler::new(device.clone(), SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            ..Default::default()
        })
        .unwrap(),
    }
}
