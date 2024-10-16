mod default;
mod expandable;
mod scroll;

use std::cell::{Cell, RefCell};
use std::ops::Add;
use std::rc::{Rc, Weak};

pub use self::default::Container;
pub use self::expandable::Expandable;
pub use self::scroll::ScrollView;
use super::{Element, ElementCell, ElementRenderer, ElementState, Focus, FocusMode};
use crate::application::{Application, PartialSizeTrait, PartialSizeTraitExt, PositionTrait, PositionTraitExt, SizeTrait};
use crate::event::{ChangeEvent, HoverInformation};
use crate::layout::{PlacementResolver, SizeBound};

pub struct ContainerState<App>
where
    App: Application,
{
    pub elements: Vec<ElementCell<App>>,
    pub state: ElementState<App>,
    pub focus_cache: Cell<Option<usize>>,
}

impl<App> ContainerState<App>
where
    App: Application,
{
    pub fn new(elements: Vec<ElementCell<App>>) -> Self {
        Self {
            elements,
            state: Default::default(),
            focus_cache: Default::default(),
        }
    }
}

impl<App> ContainerState<App>
where
    App: Application,
{
    pub fn link_back(&mut self, weak_self: Weak<RefCell<dyn Element<App>>>, weak_parent: Option<Weak<RefCell<dyn Element<App>>>>) {
        self.elements.iter().for_each(|element| {
            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, Some(weak_self.clone()));
        });
        self.state.link_back(weak_self, weak_parent);
    }

    pub fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<App>,
        application: &App,
        theme: &App::Theme,
        size_bound: &SizeBound,
        border: App::Size,
    ) -> f32 {
        let (mut inner_placement_resolver, mut size, position) = placement_resolver.derive(size_bound, App::Position::zero(), border);
        let parent_limits = inner_placement_resolver.get_parent_limits();

        // TODO: add ability to pass this in (by calling .with_gaps(..) on the
        // container) inner_placement_resolver.set_gaps(Size::new(5.0, 3.0));

        self.elements
            .iter_mut()
            .for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, application, theme));

        let final_height = inner_placement_resolver.final_height();

        if size_bound.height.is_flexible() {
            let final_height = size_bound.validated_height(
                final_height,
                placement_resolver.get_available().height(),
                placement_resolver.get_available().height(),
                &parent_limits,
                application.get_scaling(),
            );

            size = App::PartialSize::new(size.width(), Some(final_height));
            placement_resolver.register_height(final_height);
        }

        self.state.cached_size = size.finalize();
        self.state.cached_position = position;

        final_height
    }

    pub fn get_next_element(&self, start_index: usize, focus_mode: FocusMode, wrapped_around: &mut bool) -> Option<ElementCell<App>> {
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

    pub fn is_focusable<const SELF_FOCUS: bool>(&self) -> bool {
        SELF_FOCUS || self.elements.iter().any(|element| element.borrow().is_focusable())
    }

    pub fn focus_next<const SELF_FOCUS: bool>(
        &self,
        self_cell: ElementCell<App>,
        caller_cell: Option<ElementCell<App>>,
        focus: Focus,
    ) -> Option<ElementCell<App>> {
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

    pub fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
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

    pub fn hovered_element(
        &self,
        mouse_position: App::Position,
        mouse_mode: &App::MouseInputMode,
        hoverable: bool,
    ) -> HoverInformation<App> {
        let absolute_position = mouse_position.relative_to(self.state.cached_position);

        if absolute_position.left() >= 0.0
            && absolute_position.top() >= 0.0
            && absolute_position.left() <= self.state.cached_size.width()
            && absolute_position.top() <= self.state.cached_size.height()
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

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        renderer: &mut ElementRenderer<App>,
        application: &App,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        self.elements.iter().for_each(|element| {
            renderer.render_element(
                &*element.borrow(),
                application,
                theme,
                hovered_element,
                focused_element,
                mouse_mode,
                second_theme,
            )
        });
    }
}
