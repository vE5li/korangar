use std::cell::UnsafeCell;

use derive_new::new;
use procedural::*;

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
        let friend_name = Rc::new(RefCell::new(String::new()));

        let add_action = {
            let friend_name = friend_name.clone();
            Box::new(move || {
                let friend_name: &mut String = &mut friend_name.borrow_mut();
                let mut taken_string = String::new();
                std::mem::swap(friend_name, &mut taken_string);

                (!taken_string.is_empty())
                    .then_some(vec![ClickAction::Event(UserEvent::AddFriend(taken_string))])
                    .unwrap_or_default()
            })
        };

        let elements = vec![
            InputField::<24>::new(friend_name, "Name", add_action.clone(), dimension_bound!(80%)).wrap(),
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
