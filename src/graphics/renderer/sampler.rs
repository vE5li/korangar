macro_rules! create_sampler {
    ($device:expr, $filter_mode:ident, $address_mode:ident) => {
        vulkano::sampler::Sampler::new(
            $device,
            vulkano::sampler::Filter::$filter_mode,
            vulkano::sampler::Filter::$filter_mode,
            vulkano::sampler::MipmapMode::$filter_mode,
            vulkano::sampler::SamplerAddressMode::$address_mode,
            vulkano::sampler::SamplerAddressMode::$address_mode,
            vulkano::sampler::SamplerAddressMode::$address_mode,
            0.0,
            1.0,
            0.0,
            0.0,
        ).unwrap()
    }
}
