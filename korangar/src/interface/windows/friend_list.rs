use korangar_interface::event::ClickAction;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use ragnarok_packets::Friend;
use rust_state::Path;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct FriendListWindow<A> {
    friend_list_path: A,
}

impl<A> FriendListWindow<A> {
    pub fn new(friend_list_path: A) -> Self {
        Self { friend_list_path }
    }
}

impl<A> CustomWindow<ClientState> for FriendListWindow<A>
where
    A: Path<ClientState, Vec<Friend>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::FriendList)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Friend list",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            minimum_height: 300.0,
            elements: (
            )
        }
    }
}

// impl StateWindow<InterfaceSettings> for FriendsWindow {
//     fn window_class(&self) -> Option<&str> {
//         Some(Self::WINDOW_CLASS)
//     }
//
//     fn to_window(
//         &self,
//     ) -> Window<InterfaceSettings> {
//         let friend_name = PlainTrackedState::<String>::default();
//
//         let add_action = {
//             let mut friend_name = friend_name.clone();
//
//             Box::new(move || {
//                 let taken_string = friend_name.take();
//
//                 (!taken_string.is_empty())
//
// .then_some(vec![ClickAction::Custom(UserEvent::AddFriend(taken_string))])
//                     .unwrap_or_default()
//             })
//         };
//
//         let elements = vec![
//             InputFieldBuilder::new()
//                 .with_state(friend_name)
//                 .with_ghost_text("Name")
//                 .with_enter_action(add_action.clone())
//                 .with_length(24)
//                 .with_width_bound(dimension_bound!(80%))
//                 .build()
//                 .wrap(),
//             ButtonBuilder::new()
//                 .with_text("Add")
//                 .with_event(add_action)
//                 .with_width_bound(dimension_bound!(!))
//                 .build()
//                 .wrap(),
//             FriendView::new(self.friend_list.clone()).wrap(),
//         ];
//
//         WindowBuilder::new()
//             .with_title("Friends".to_string())
//             .with_class(Self::WINDOW_CLASS.to_owned())
//             .with_size_bound(size_bound!(200 > 300 < 400, ?))
//             .with_elements(elements)
//             .closable()
//             .build(window_cache, application, available_space)
//     }
// }
