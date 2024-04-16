use std::cell::{RefCell, UnsafeCell};
use std::rc::{Rc, Weak};

use korangar_interface::elements::{
    ButtonBuilder, ContainerState, Element, ElementCell, ElementState, ElementWrap, Expandable, Focus, WeakElementCell,
};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};
use ragnarok_networking::Friend;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::InterfaceTheme;

pub struct FriendView {
    friends: PlainRemote<Vec<(Friend, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>)>>,
    state: ContainerState<InterfaceSettings>,
}

impl FriendView {
    pub fn new(friends: PlainRemote<Vec<(Friend, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>)>>) -> Self {
        let elements = {
            let friends = friends.get();

            friends
                .iter()
                .map(|(friend, linked_element)| {
                    let element = Self::friend_to_element(friend);
                    unsafe { *linked_element.get() = Some(Rc::downgrade(&element)) };
                    element
                })
                .collect()
        };

        Self {
            friends,
            state: ContainerState::new(elements),
        }
    }

    fn friend_to_element(friend: &Friend) -> ElementCell<InterfaceSettings> {
        let elements = vec![
            ButtonBuilder::new()
                .with_text("remove")
                .with_event(UserEvent::RemoveFriend {
                    account_id: friend.account_id,
                    character_id: friend.character_id,
                })
                .build()
                .wrap(),
        ];

        Expandable::new(friend.name.clone(), elements, false).wrap()
    }
}

impl Element<InterfaceSettings> for FriendView {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state.state
    }

    fn link_back(
        &mut self,
        weak_self: Weak<RefCell<dyn Element<InterfaceSettings>>>,
        weak_parent: Option<Weak<RefCell<dyn Element<InterfaceSettings>>>>,
    ) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<InterfaceSettings>,
        caller_cell: Option<ElementCell<InterfaceSettings>>,
        focus: Focus,
    ) -> Option<ElementCell<InterfaceSettings>> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<InterfaceSettings>) -> Option<ElementCell<InterfaceSettings>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        self.state.resolve(
            placement_resolver,
            application,
            theme,
            &size_bound!(100%, ?),
            ScreenSize::default(),
        );
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let mut resolve = false;

        if self.friends.consume_changed() {
            // Remove elements of old friends from the start of the list and add new friends
            // to the list.
            self.friends.get().iter().enumerate().for_each(|(index, (friend, linked_element))| {
                if let Some(linked_element) = unsafe { &(*linked_element.get()) } {
                    while !std::ptr::addr_eq(linked_element.as_ptr(), Rc::downgrade(&self.state.elements[index]).as_ptr()) {
                        self.state.elements.remove(index);
                    }
                } else {
                    let element = Self::friend_to_element(friend);
                    unsafe { *linked_element.get() = Some(Rc::downgrade(&element)) };
                    let weak_self = self.state.state.self_element.clone();

                    element.borrow_mut().link_back(Rc::downgrade(&element), weak_self);

                    self.state.elements.insert(index, element);
                    resolve = true;
                }
            });

            // Remove elements of old friends from the end of the list.
            let friend_count = self.friends.get().len();
            if friend_count < self.state.elements.len() {
                self.state.elements.truncate(friend_count);
                resolve = true;
            }
        }

        match resolve {
            true => Some(ChangeEvent::RESOLVE_WINDOW),
            false => None,
        }
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        focused_element: Option<&dyn Element<InterfaceSettings>>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(
            &mut renderer,
            application,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        );
    }
}
