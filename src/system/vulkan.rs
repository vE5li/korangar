use std::sync::Arc;

use cgmath::{Matrix4, Vector3};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::DeviceExtensions;
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::swapchain::Surface;
use vulkano::VulkanLibrary;

#[cfg(feature = "debug")]
use crate::debug::*;

pub fn get_instance_extensions(library: &VulkanLibrary) -> InstanceExtensions {
    InstanceExtensions {
        ext_debug_utils: true,
        //khr_get_physical_device_properties2: true,
        ..vulkano_win::required_extensions(library)
    }
}

pub fn get_layers(library: &VulkanLibrary) -> Vec<String> {
    let available_layers: Vec<_> = library.layer_properties().unwrap().collect();
    let desired_layers = vec![/*"VK_LAYER_KHRONOS_validation".to_owned()*/];

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
        .filter(|layer| available_layers.iter().any(|li| li.name() == layer))
        .collect()
}

pub fn get_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    }
}

pub fn choose_physical_device(
    instance: &Arc<Instance>,
    surface: &Surface,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| q.queue_flags.graphics && p.surface_support(i as u32, surface).unwrap_or(false))
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            vulkano::device::physical::PhysicalDeviceType::DiscreteGpu => 0,
            vulkano::device::physical::PhysicalDeviceType::IntegratedGpu => 1,
            vulkano::device::physical::PhysicalDeviceType::VirtualGpu => 2,
            vulkano::device::physical::PhysicalDeviceType::Cpu => 3,
            vulkano::device::physical::PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .unwrap()
}

pub fn multiply_matrix4_and_vector3(matrix: &Matrix4<f32>, vector: Vector3<f32>) -> Vector3<f32> {
    let adjusted_vector = matrix * vector.extend(1.0);
    (adjusted_vector / adjusted_vector.w).truncate()
}
