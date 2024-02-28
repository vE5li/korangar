use std::cell::UnsafeCell;

use procedural::*;

use crate::interface::*;

pub struct PacketWindow<const N: usize> {
    packets: Remote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell>>), N>>,
    show_pings: TrackedState<bool>,
    update: TrackedState<bool>,
}

impl<const N: usize> PacketWindow<N> {
    pub const WINDOW_CLASS: &'static str = "network";

    pub fn new(packets: Remote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell>>), N>>, update: TrackedState<bool>) -> Self {
        let show_pings = TrackedState::new(false);

        Self {
            packets,
            show_pings,
            update,
        }
    }
}

impl<const N: usize> PrototypeWindow for PacketWindow<N> {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![PacketView::new(self.packets.clone(), self.show_pings.new_remote()).wrap()];

        let clear_selector = {
            let packets = self.packets.clone();
            move || !packets.borrow().is_empty()
        };

        let clear_action = { move || vec![ClickAction::Event(UserEvent::ClearPacketHistory)] };

        let elements = vec![
            Button::default()
                .with_text("Clear")
                .with_disabled_selector(clear_selector)
                .with_event(Box::new(clear_action))
                .with_width(dimension!(33.33%))
                .wrap(),
            StateButton::default()
                .with_text("Show pings")
                .with_selector(self.show_pings.selector())
                .with_event(self.show_pings.toggle_action())
                .with_width(dimension!(33.33%))
                .wrap(),
            StateButton::default()
                .with_text("Update")
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
