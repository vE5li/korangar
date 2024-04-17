use std::sync::Arc;

use cgmath::{Matrix4, Vector3};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{DeviceExtensions, QueueFlags};
#[cfg(feature = "debug")]
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessengerCallbackData};
use vulkano::instance::Instance;
use vulkano::swapchain::Surface;
use vulkano::VulkanLibrary;

pub fn get_layers(library: &VulkanLibrary) -> Vec<String> {
    let available_layers: Vec<_> = library.layer_properties().unwrap().collect();
    let desired_layers: Vec<String> = vec![/*"VK_LAYER_KHRONOS_validation".to_owned()*/];

    #[cfg(feature = "debug")]
    let timer = Timer::new("available layers");

    #[cfg(feature = "debug")]
    for layer in &available_layers {
        print_debug!("{}", layer.name().magenta());
    }

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    for layer in &desired_layers {
        print_debug!("{}", layer.magenta());
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
                .position(|(i, q)| q.queue_flags.intersects(QueueFlags::GRAPHICS) && p.surface_support(i as u32, surface).unwrap_or(false))
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

#[cfg(feature = "debug")]
pub fn vulkan_message_callback(
    message_severity: DebugUtilsMessageSeverity,
    message_type: DebugUtilsMessageType,
    callback_data: DebugUtilsMessengerCallbackData<'_>,
) {
    let severity = if message_severity.intersects(DebugUtilsMessageSeverity::ERROR) {
        "error".red()
    } else if message_severity.intersects(DebugUtilsMessageSeverity::WARNING) {
        "warning".yellow()
    } else if message_severity.intersects(DebugUtilsMessageSeverity::INFO) {
        "information".cyan()
    } else if message_severity.intersects(DebugUtilsMessageSeverity::VERBOSE) {
        "verbose".magenta()
    } else {
        panic!("no-impl");
    };

    let message_type = if message_type.intersects(DebugUtilsMessageType::GENERAL) {
        "general"
    } else if message_type.intersects(DebugUtilsMessageType::VALIDATION) {
        "validation"
    } else if message_type.intersects(DebugUtilsMessageType::PERFORMANCE) {
        "performance"
    } else {
        panic!("no-impl");
    };

    print_debug!(
        "{} [{}] [{}]: {}",
        callback_data.message_id_name.unwrap_or("unknown").magenta(),
        message_type.yellow(),
        severity,
        callback_data.message
    );
}
