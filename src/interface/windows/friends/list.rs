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
    pub const WINDOW_CLASS: &str = "friends";
}

impl PrototypeWindow for FriendsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let friend_name = Rc::new(RefCell::new(String::new()));

        let add_action = {
            let friend_name = friend_name.clone();
            Box::new(move || {
                let friend_name: &mut String = &mut friend_name.borrow_mut();
                let mut taken_string = String::new();
                std::mem::swap(friend_name, &mut taken_string);

                (!taken_string.is_empty()).then_some(ClickAction::Event(UserEvent::AddFriend(taken_string)))
            })
        };

        let elements = vec![
            InputField::<24>::new(friend_name, "name", add_action.clone(), dimension!(80%)).wrap(),
            Button::default()
                .with_text("add")
                .with_event(add_action)
                .with_width(dimension!(!))
                .wrap(),
            FriendView::new(self.friend_list.clone()).wrap(),
        ];

        WindowBuilder::default()
            .with_title("Friends".to_string())
            .with_class(Self::WINDOW_CLASS.to_owned())
            .with_size(constraint!(200 > 300 < 400, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
