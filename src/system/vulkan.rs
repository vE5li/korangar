use vulkano::instance::InstanceExtensions;
use vulkano::device::DeviceExtensions;

#[cfg(feature = "debug")]
use debug::*;

pub fn get_instance_extensions() -> InstanceExtensions {
    InstanceExtensions {
        ext_debug_report: true,
        ..vulkano_win::required_extensions()
    }
}

pub fn get_layers() -> Vec<&'static str> {

    let available_layers: Vec<_> = vulkano::instance::layers_list().unwrap().collect();
    let desired_layers = vec![];// vec!["VK_LAYER_KHRONOS_validation"];

    #[cfg(feature = "debug")]
    let timer = Timer::new_dynamic(format!("available layers"));

    #[cfg(feature = "debug")]
    for layer in &available_layers {
        print_debug!("{}{}{}", magenta(), layer.name(), none());
    }

    #[cfg(feature = "debug")]
    timer.stop();

    let used_layers = desired_layers
        .into_iter()
        .filter(|&l| available_layers.iter().any(|li| li.name() == l))
        .collect();

    #[cfg(feature = "debug")]
    let timer = Timer::new_dynamic(format!("used layers"));

    #[cfg(feature = "debug")]
    for layer in &used_layers {
        print_debug!("{}{}{}", magenta(), layer, none());
    }

    #[cfg(feature = "debug")]
    timer.stop();

    return used_layers;
}

pub fn get_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
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
    }}
}
