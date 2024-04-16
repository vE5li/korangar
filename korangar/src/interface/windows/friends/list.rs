use std::cell::UnsafeCell;

use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap, InputFieldBuilder, WeakElementCell};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedStateTake};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};
use ragnarok_networking::Friend;

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::FriendView;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(new)]
pub struct FriendsWindow {
    friend_list: PlainRemote<Vec<(Friend, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>)>>,
}

impl FriendsWindow {
    pub const WINDOW_CLASS: &'static str = "friends";
}

impl PrototypeWindow<InterfaceSettings> for FriendsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let friend_name = PlainTrackedState::<String>::default();

        let add_action = {
            let mut friend_name = friend_name.clone();

            Box::new(move || {
                let taken_string = friend_name.take();

                (!taken_string.is_empty())
                    .then_some(vec![ClickAction::Custom(UserEvent::AddFriend(taken_string))])
                    .unwrap_or_default()
            })
        };

        let elements = vec![
            InputFieldBuilder::new()
                .with_state(friend_name)
                .with_ghost_text("Name")
                .with_enter_action(add_action.clone())
                .with_length(24)
                .with_width_bound(dimension_bound!(80%))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Add")
                .with_event(add_action)
                .with_width_bound(dimension_bound!(!))
                .build()
                .wrap(),
            FriendView::new(self.friend_list.clone()).wrap(),
        ];

        WindowBuilder::new()
            .with_title("Friends".to_string())
            .with_class(Self::WINDOW_CLASS.to_owned())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
