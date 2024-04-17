use std::cell::UnsafeCell;

use korangar_debug::profiling::RingBuffer;
use korangar_interface::elements::{ButtonBuilder, ElementWrap, ScrollView, StateButtonBuilder, WeakElementCell};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainRemote, PlainTrackedState, Remote, TrackedState, TrackedStateBinary};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::{PacketEntry, PacketView};
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

pub struct PacketWindow<const N: usize> {
    packets: PlainRemote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>), N>>,
    show_pings: PlainTrackedState<bool>,
    update: PlainTrackedState<bool>,
}

impl<const N: usize> PacketWindow<N> {
    pub const WINDOW_CLASS: &'static str = "network";

    pub fn new(
        packets: PlainRemote<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>), N>>,
        update: PlainTrackedState<bool>,
    ) -> Self {
        let show_pings = PlainTrackedState::default();

        Self {
            packets,
            show_pings,
            update,
        }
    }
}

impl<const N: usize> PrototypeWindow<InterfaceSettings> for PacketWindow<N> {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![PacketView::new(self.packets.clone(), self.show_pings.new_remote()).wrap()];

        let clear_selector = {
            let packets = self.packets.clone();
            move || !packets.get().is_empty()
        };

        let clear_action = { move || vec![ClickAction::Custom(UserEvent::ClearPacketHistory)] };

        let elements = vec![
            ButtonBuilder::new()
                .with_text("Clear")
                .with_disabled_selector(clear_selector)
                .with_event(Box::new(clear_action))
                .with_width_bound(dimension_bound!(33.33%))
                .build()
                .wrap(),
            StateButtonBuilder::new()
                .with_text("Show pings")
                .with_remote(self.show_pings.new_remote())
                .with_event(self.show_pings.toggle_action())
                .with_width_bound(dimension_bound!(33.33%))
                .build()
                .wrap(),
            StateButtonBuilder::new()
                .with_text("Update")
                .with_remote(self.update.new_remote())
                .with_event(self.update.toggle_action())
                .with_width_bound(dimension_bound!(!))
                .build()
                .wrap(),
            ScrollView::new(elements, size_bound!(100%, ? < super)).wrap(),
        ];

        WindowBuilder::new()
            .with_title("Network".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
