mod character;
mod default;
mod dialog;
mod equipment;
mod expandable;
mod friends;
mod hotbar;
mod inventory;
#[cfg(feature = "debug")]
mod packet;
mod scroll;
mod skill_tree;

use std::cell::Cell;
use std::ops::Add;
use std::rc::Weak;

use derive_new::new;

pub use self::character::CharacterPreview;
pub use self::default::Container;
pub use self::dialog::{DialogContainer, DialogElement};
pub use self::equipment::EquipmentContainer;
pub use self::expandable::Expandable;
pub use self::friends::FriendView;
pub use self::hotbar::HotbarContainer;
pub use self::inventory::InventoryContainer;
#[cfg(feature = "debug")]
pub use self::packet::{PacketEntry, PacketView};
pub use self::scroll::ScrollView;
pub use self::skill_tree::SkillTreeContainer;
use crate::input::MouseInputMode;
use crate::interface::*;

#[derive(new)]
pub struct ContainerState {
    elements: Vec<ElementCell>,
    #[new(default)]
    state: ElementState,
    #[new(default)]
    focus_cache: Cell<Option<usize>>,
}

impl ContainerState {
    pub fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
        self.elements.iter().for_each(|element| {
            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, Some(weak_self.clone()));
        });
        self.state.link_back(weak_self, weak_parent);
    }

    pub fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        size_bound: &SizeBound,
        border: ScreenSize,
    ) -> f32 {
        let (mut inner_placement_resolver, mut size, position) = placement_resolver.derive(size_bound, ScreenPosition::default(), border);
        let parent_limits = inner_placement_resolver.get_parent_limits();

        // TODO: add ability to pass this in (by calling .with_gaps(..) on the
        // container) inner_placement_resolver.set_gaps(Size::new(5.0, 3.0));

        self.elements.iter_mut().for_each(|element| {
            element
                .borrow_mut()
                .resolve(&mut inner_placement_resolver, interface_settings, theme)
        });

        let final_height = inner_placement_resolver.final_height();

        if size_bound.height.is_flexible() {
            let final_height = size_bound.validated_height(
                final_height,
                placement_resolver.get_available().height,
                placement_resolver.get_available().height,
                &parent_limits,
                interface_settings.scaling.get(),
            );
            size.height = Some(final_height);
            placement_resolver.register_height(final_height);
        }

        self.state.cached_size = size.finalize();
        self.state.cached_position = position;

        final_height
    }

    fn get_next_element(&self, start_index: usize, focus_mode: FocusMode, wrapped_around: &mut bool) -> Option<ElementCell> {
        if self.elements.is_empty() {
            return None;
        }

        let element_count = self.elements.len();
        let mut index = start_index;

        loop {
            let element = self.elements[index].borrow();

            // TODO: add focus up, down etc
            if element.is_focusable() {
                *wrapped_around |= match focus_mode {
                    FocusMode::FocusNext => index < start_index,
                    FocusMode::FocusPrevious => index > start_index,
                };

                self.focus_cache.set(Some(index));
                return Some(self.elements[index].clone());
            }

            match focus_mode {
                FocusMode::FocusNext => index = index.add(1) % element_count,
                FocusMode::FocusPrevious => index = index.wrapping_sub(1).min(element_count - 1),
            }

            if index == start_index {
                return None;
            }
        }
    }

    fn is_focusable<const SELF_FOCUS: bool>(&self) -> bool {
        SELF_FOCUS || self.elements.iter().any(|element| element.borrow().is_focusable())
    }

    fn focus_next<const SELF_FOCUS: bool>(
        &self,
        self_cell: ElementCell,
        caller_cell: Option<ElementCell>,
        focus: Focus,
    ) -> Option<ElementCell> {
        if focus.downwards {
            if SELF_FOCUS {
                if focus.mode == FocusMode::FocusPrevious {
                    let element = self
                        .get_next_element(self.elements.len().saturating_sub(1), focus.mode, &mut false)
                        .and_then(|element| element.borrow().focus_next(element.clone(), Some(self_cell.clone()), focus));

                    if element.is_some() {
                        return element;
                    }
                }

                self.focus_cache.take();
                return Some(self_cell);
            }

            let start_index = match focus.mode {
                FocusMode::FocusNext => 0,
                FocusMode::FocusPrevious => self.elements.len().saturating_sub(1),
            };

            return self
                .get_next_element(start_index, focus.mode, &mut false)
                .and_then(|element| element.borrow().focus_next(element.clone(), Some(self_cell), focus));
        }

        if let Some(caller_cell) = caller_cell {
            // find focused element
            let position = self
                .elements
                .iter()
                .position(|element| element.borrow().is_element_self(Some(&*caller_cell.borrow())));

            if let Some(position) = position {
                let offset_position = match focus.mode {
                    FocusMode::FocusNext => position.add(1) % self.elements.len(),
                    FocusMode::FocusPrevious => position.wrapping_sub(1).min(self.elements.len() - 1),
                };

                let mut wrapped_around = match focus.mode {
                    FocusMode::FocusNext => offset_position <= position,
                    FocusMode::FocusPrevious => offset_position >= position,
                };

                let element = self.get_next_element(offset_position, focus.mode, &mut wrapped_around);
                let cached_index = self.focus_cache.get();

                if wrapped_around {
                    if focus.mode == FocusMode::FocusPrevious && SELF_FOCUS {
                        self.focus_cache.take();
                        return Some(self_cell);
                    }

                    if let Some(parent_element) = &self.state.parent_element {
                        let parent_element = parent_element.upgrade().unwrap();
                        let next_element = parent_element
                            .borrow()
                            .focus_next(parent_element.clone(), Some(self_cell.clone()), focus);

                        if next_element.is_some() {
                            // important to clear here since this element might be used as a fallback if the
                            // next sibling is removed.
                            self.focus_cache.take();
                            return next_element;
                        }
                    }
                }

                // restore the cache in case it was changed
                self.focus_cache.set(cached_index);

                // should this really always return and not just if is_some?
                return element.and_then(|element| element.borrow().focus_next(element.clone(), Some(self_cell), focus.to_downwards()));
            }

            panic!("when did this happen? implement correct behavior");
        }

        let start_index = match focus.mode {
            FocusMode::FocusNext => 0,
            FocusMode::FocusPrevious => self.elements.len() - 1,
        };

        let mut wrapped_around = false;
        let focusable_element = self
            .get_next_element(start_index, focus.mode, &mut wrapped_around)
            .and_then(|element| {
                element
                    .borrow()
                    .focus_next(element.clone(), Some(self_cell.clone()), focus.to_downwards())
            });
        let cached_index = self.focus_cache.get();

        if let Some(parent_element) = &self.state.parent_element
            && (wrapped_around || focusable_element.is_none() || focus.mode == FocusMode::FocusPrevious)
        {
            let parent_element = parent_element.upgrade().unwrap();
            let next_element = parent_element.borrow().focus_next(parent_element.clone(), Some(self_cell), focus);

            if next_element.is_some() {
                // important to clear here since this element might be used as a fallback if the
                // next sibling is removed.
                self.focus_cache.take();
                return next_element;
            }
        }

        // restore the cache in case it was changed
        self.focus_cache.set(cached_index);

        focusable_element
    }

    fn restore_focus(&self, self_cell: ElementCell) -> Option<ElementCell> {
        if let Some(index) = self.focus_cache.get()
            && !self.elements.is_empty()
        {
            let focused_element = self.elements[0..index.add(1).min(self.elements.len())]
                .iter()
                .rev()
                .find_map(|element| element.borrow().restore_focus(element.clone()));

            if focused_element.is_some() {
                return focused_element;
            }
        }

        // TODO: only if focusable
        Some(self_cell)
    }

    pub fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::union).or(current).or(other)
            })
    }

    pub fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode, hoverable: bool) -> HoverInformation {
        let absolute_position = ScreenPosition::from_size(mouse_position - self.state.cached_position);

        if absolute_position.left >= 0.0
            && absolute_position.top >= 0.0
            && absolute_position.left <= self.state.cached_size.width
            && absolute_position.top <= self.state.cached_size.height
        {
            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position, mouse_mode) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Missed => {}
                    hover_information => return hover_information,
                }
            }

            if hoverable {
                return HoverInformation::Hovered;
            }
        }

        HoverInformation::Missed
    }

    pub fn render(
        &self,
        renderer: &mut ElementRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        self.elements.iter().for_each(|element| {
            renderer.render_element(
                &*element.borrow(),
                state_provider,
                interface_settings,
                theme,
                hovered_element,
                focused_element,
                mouse_mode,
                second_theme,
            )
        });
    }
}
