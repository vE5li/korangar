use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;
use crate::loaders::Service;

#[derive(new)]
pub struct SelectServiceWindow {
    services: Vec<Service>,
}

impl SelectServiceWindow {
    pub const WINDOW_CLASS: &'static str = "service_select";
}

impl PrototypeWindow for SelectServiceWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = self
            .services
            .iter()
            .map(|service| {
                Button::default()
                    .with_text(service.display_name.clone().unwrap())
                    .with_event(UserEvent::SelectService(service.clone()))
                    .wrap()
            })
            .collect();

        WindowBuilder::default()
            .with_title("Select Service".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ? < 80%))
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
