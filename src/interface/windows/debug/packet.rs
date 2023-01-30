use procedural::*;

use crate::interface::*;

pub struct PacketWindow {
    packets: TrackedState<Vec<PacketEntry>>,
    cleared: TrackedState<()>,
    show_pings: TrackedState<bool>,
    update: TrackedState<bool>,
}

impl PacketWindow {
    pub const WINDOW_CLASS: &'static str = "network";

    pub fn new(packets: TrackedState<Vec<PacketEntry>>) -> Self {
        let cleared = TrackedState::new(());
        let show_pings = TrackedState::new(false);
        let update = TrackedState::new(true);

        Self {
            packets,
            cleared,
            show_pings,
            update,
        }
    }
}

impl PrototypeWindow for PacketWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![
            PacketView::new(
                self.packets.clone(),
                self.cleared.new_remote(),
                self.show_pings.new_remote(),
                self.update.new_remote(),
            )
            .wrap(),
        ];

        let clear_selector = {
            let packets = self.packets.clone();
            move || !packets.borrow().is_empty()
        };

        let clear_action = {
            let mut packets = self.packets.clone();
            let mut cleared = self.cleared.clone();

            move || {
                packets.clear();
                cleared.update();
                None
            }
        };

        let elements = vec![
            Button::default()
                .with_text("clear")
                .with_disabled_selector(clear_selector)
                .with_event(Box::new(clear_action))
                .with_width(dimension!(33.33%))
                .wrap(),
            StateButton::default()
                .with_text("show pings")
                .with_selector(self.show_pings.selector())
                .with_event(self.show_pings.toggle_action())
                .with_width(dimension!(33.33%))
                .wrap(),
            StateButton::default()
                .with_text("update")
                .with_selector(self.update.selector())
                .with_event(self.update.toggle_action())
                .with_width(dimension!(!))
                .wrap(),
            ScrollView::new(elements, constraint!(100%, ?)).wrap(),
        ];

        WindowBuilder::default()
            .with_title("Network".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
