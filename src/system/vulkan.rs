use cgmath::{Matrix4, Vector3};
use vulkano::device::DeviceExtensions;
use vulkano::instance::InstanceExtensions;

#[cfg(feature = "debug")]
use crate::debug::*;

pub fn get_instance_extensions() -> InstanceExtensions {
    InstanceExtensions {
        ext_debug_utils: true,
        //khr_get_physical_device_properties2: true,
        ..vulkano_win::required_extensions()
    }
}

pub fn get_layers() -> Vec<&'static str> {
    let available_layers: Vec<_> = vulkano::instance::layers_list().unwrap().collect();
    let desired_layers = Vec::new(); // vec!["VK_LAYER_KHRONOS_validation"];

    #[cfg(feature = "debug")]
    let timer = Timer::new("available layers");

    #[cfg(feature = "debug")]
    for layer in &available_layers {
        print_debug!("{}{}{}", MAGENTA, layer.name(), NONE);
    }

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    for layer in &desired_layers {
        print_debug!("{}{}{}", MAGENTA, layer, NONE);
    }

    #[cfg(feature = "debug")]
    let timer = Timer::new("used layers");

    #[cfg(feature = "debug")]
    timer.stop();

    desired_layers
        .into_iter()
        .filter(|&l| available_layers.iter().any(|li| li.name() == l))
        .collect()
}

pub fn get_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
        //amd_mixed_attachment_samples: true,
        ..DeviceExtensions::none()
    }
}

macro_rules! choose_physical_device {
    ($instance:expr, $surface:expr, $device_extensions:expr) => {{
        vulkano::device::physical::PhysicalDevice::enumerate($instance)
            .filter(|&p| p.supported_extensions().is_superset_of($device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| q.supports_graphics() && $surface.is_supported(q).unwrap_or(false))
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                vulkano::device::physical::PhysicalDeviceType::DiscreteGpu => 0,
                vulkano::device::physical::PhysicalDeviceType::IntegratedGpu => 1,
                vulkano::device::physical::PhysicalDeviceType::VirtualGpu => 2,
                vulkano::device::physical::PhysicalDeviceType::Cpu => 3,
                vulkano::device::physical::PhysicalDeviceType::Other => 4,
            })
            .unwrap()
    }};
}

pub fn multiply_matrix4_and_vector3(matrix: &Matrix4<f32>, vector: Vector3<f32>) -> Vector3<f32> {
    let adjusted_vector = matrix * vector.extend(1.0);
    (adjusted_vector / adjusted_vector.w).truncate()
}
