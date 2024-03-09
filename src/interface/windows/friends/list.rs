use std::cell::UnsafeCell;

use derive_new::new;
use procedural::dimension_bound;

use crate::interface::*;
use crate::network::Friend;

#[derive(new)]
pub struct FriendsWindow {
    friend_list: Remote<Vec<(Friend, UnsafeCell<Option<WeakElementCell>>)>>,
}

impl FriendsWindow {
    pub const WINDOW_CLASS: &'static str = "friends";
}

impl PrototypeWindow for FriendsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let friend_name = TrackedState::<String>::default();

        let add_action = {
            let mut friend_name = friend_name.clone();

            Box::new(move || {
                let taken_string = friend_name.take();

                (!taken_string.is_empty())
                    .then_some(vec![ClickAction::Event(UserEvent::AddFriend(taken_string))])
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
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
