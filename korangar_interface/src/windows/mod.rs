mod anchor;
mod builder;
mod prototype;

use std::rc::Rc;

use rust_state::{Context, ReadState, View};

pub use self::anchor::{Anchor, AnchorPoint};
pub use self::builder::WindowBuilder;
pub use self::prototype::PrototypeWindow;
use crate::application::{Application, ClipTrait, ColorTrait, InterfaceRenderer, PositionTrait, PositionTraitExt, SizeTrait, SizeTraitExt};
use crate::elements::{ElementCell, Focus};
use crate::event::{ChangeEvent, HoverInformation};
use crate::layout::{Dimension, PlacementResolver, SizeBound};
use crate::theme::WindowTheme;
use crate::{ColorEvaluator, _Tracker};

#[derive(Default)]
pub struct WindowReadState {
    initialize_state: ReadState,
    resolve_state: ReadState,
    render_state: ReadState,
}

pub struct Window<App>
where
    App: Application,
{
    window_class: Option<String>,
    anchor: Anchor<App>,
    position: App::Position,
    size_bound: SizeBound,
    size: App::Size,
    elements: Vec<ElementCell<App>>,
    popup_element: Option<(ElementCell<App>, _Tracker<App::Position>, _Tracker<App::Size>)>,
    closable: bool,
    background_color: Option<ColorEvaluator<App>>,
    theme_kind: App::ThemeKind,
}

impl<App> Window<App>
where
    App: Application,
{
    pub fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_deref()
    }

    fn get_background_color(&self, state: &View<App>, theme_selector: App::ThemeSelector) -> App::Color {
        self.background_color
            .as_ref()
            .map(|closure| closure(state, theme_selector))
            .unwrap_or(*state.get_safe(&WindowTheme::background_color(theme_selector)))
    }

    pub fn has_transparency(&self, state: &View<App>, theme_selector: App::ThemeSelector) -> bool {
        self.get_background_color(state, theme_selector).is_transparent()
    }

    pub fn is_closable(&self) -> bool {
        self.closable
    }

    pub fn get_theme_kind(&self) -> &App::ThemeKind {
        &self.theme_kind
    }

    pub fn resolve(
        &mut self,
        font_loader: App::FontLoader,
        state: &Context<App>,
        read_state: &mut WindowReadState,
        theme_selector: App::ThemeSelector,
        available_space: App::Size,
    ) -> App::Size {
        let state = read_state.resolve_state.track_new(state);
        let mut placement_resolver = PlacementResolver::new(
            font_loader.clone(),
            available_space,
            self.size,
            &self.size_bound,
            *state.get_safe(&WindowTheme::border_size(theme_selector)),
            *state.get_safe(&WindowTheme::gaps(theme_selector)),
            *state.get_safe(&App::ScaleSelector::default()),
        );

        self.elements
            .iter()
            .for_each(|element| element.borrow_mut().resolve(&state, theme_selector, &mut placement_resolver));

        if self.size_bound.height.is_flexible() {
            let parent_limits = placement_resolver.get_parent_limits();
            let final_height = placement_resolver.final_height();

            let final_height = self.size_bound.validated_height(
                final_height,
                available_space.height().into(),
                available_space.height().into(),
                &parent_limits,
                *state.get_safe(&App::ScaleSelector::default()),
            );

            self.size = App::Size::new(self.size.width(), final_height);
            self.validate_size(&state, available_space);
        }

        self.validate_position(available_space);

        if let Some((popup, _, size_tracker)) = &self.popup_element {
            let size = size_tracker().unwrap(); // FIX: Don't unwrap obviously

            let size_bound = SizeBound {
                minimum_height: Some(Dimension::Absolute(0.0)),
                maximum_height: Some(Dimension::Absolute(250.0)),
                ..SizeBound::only_height(Dimension::Flexible)
            };

            let mut placement_resolver = PlacementResolver::new(
                font_loader,
                available_space,
                // TODO: 250 is an arbitrary limitation. This should be replaced with a value based
                // on some reasoning.
                App::Size::new(size.width(), 250.0),
                &size_bound,
                App::Size::zero(), //theme.window.border_size.get(), // TODO: Popup
                App::Size::zero(), //theme.window.gaps.get(), // TODO: Popup
                *state.get_safe(&App::ScaleSelector::default()),
            );

            popup.borrow_mut().resolve(&state, theme_selector, &mut placement_resolver);
        };

        self.size
    }

    pub fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::union).or(current).or(other)
            })
    }

    pub fn first_focused_element(&self) -> Option<ElementCell<App>> {
        let element_cell = self.elements[0].clone();
        self.elements[0].borrow().focus_next(element_cell, None, Focus::downwards())
    }

    pub fn restore_focus(&self) -> Option<ElementCell<App>> {
        self.elements[0].borrow().restore_focus(self.elements[0].clone())
    }

    pub fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        let absolute_position = mouse_position.relative_to(self.position);

        if let Some((popup, position_tracker, _)) = &self.popup_element {
            let position = position_tracker().unwrap(); // FIX: Don't unwrap obviously
            let position = mouse_position.relative_to(position);

            match popup.borrow().hovered_element(position, mouse_mode) {
                HoverInformation::Hovered => return HoverInformation::Element(popup.clone()),
                HoverInformation::Missed => {}
                hover_information => return hover_information,
            }
        }

        if absolute_position.left() >= 0.0
            && absolute_position.top() >= 0.0
            && absolute_position.left() <= self.size.width()
            && absolute_position.top() <= self.size.height()
        {
            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position, mouse_mode) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Missed => {}
                    hover_information => return hover_information,
                }
            }

            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    pub fn get_layout(&self) -> (Anchor<App>, App::Size) {
        (self.anchor.clone(), self.size)
    }

    pub fn get_area(&self) -> (App::Position, App::Size) {
        (self.position, self.size)
    }

    pub fn hovers_area(&self, position: App::Position, size: App::Size) -> bool {
        let self_combined = self.position.offset(self.size);
        let area_combined = position.offset(size);

        self_combined.left() > position.left()
            && self.position.left() < area_combined.left()
            && self_combined.top() > position.top()
            && self.position.top() < area_combined.top()
    }

    pub fn offset(&mut self, available_space: App::Size, offset: App::Position) -> Option<(&str, Anchor<App>)> {
        self.position = self.position.combined(offset);
        self.anchor.update(available_space, self.position, self.size);

        self.validate_position(available_space);

        self.window_class
            .as_ref()
            .map(|window_class| (window_class.as_str(), self.anchor.clone()))
    }

    fn validate_position(&mut self, available_space: App::Size) {
        self.position = self.anchor.current_position(available_space, self.size);
        self.position = self.size_bound.validated_position(self.position, self.size, available_space);
    }

    pub fn resize(&mut self, application: &View<App>, available_space: App::Size, growth: App::Size) -> (Option<&str>, App::Size) {
        self.size = self.size.grow(growth);
        self.validate_size(application, available_space);
        (self.window_class.as_deref(), self.size)
    }

    fn validate_size(&mut self, state: &View<App>, available_space: App::Size) {
        self.size = self
            .size_bound
            .validated_window_size(self.size, available_space, *state.get_safe(&App::ScaleSelector::default()));
    }

    pub fn open_popup(&mut self, element: ElementCell<App>, position_tracker: _Tracker<App::Position>, size_tracker: _Tracker<App::Size>) {
        // Very important to link back
        let weak_element = Rc::downgrade(&element);
        element.borrow_mut().link_back(weak_element, None);

        self.popup_element = Some((element, position_tracker, size_tracker));
    }

    pub fn close_popup(&mut self) {
        self.popup_element = None;
    }

    pub fn render(
        &self,
        read_state: &mut WindowReadState,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &Context<App>,
        theme_selector: App::ThemeSelector,
    ) {
        let state = read_state.resolve_state.track_new(state);
        let screen_clip = App::Clip::new(
            self.position.left(),
            self.position.top(),
            self.position.left() + self.size.width(),
            self.position.top() + self.size.height(),
        );

        let corner_radius = state.get_safe(&WindowTheme::corner_radius(theme_selector));

        renderer.render_rectangle(
            render_target,
            self.position,
            self.size,
            screen_clip,
            *corner_radius,
            self.get_background_color(&state, theme_selector),
        );

        self.elements.iter().for_each(|element| {
            element.borrow().render(
                render_target,
                renderer,
                &state,
                theme_selector,
                self.position,
                screen_clip,
                false,
            )
        });

        if let Some((popup, position_tracker, _)) = &self.popup_element {
            let position = position_tracker().unwrap(); // FIX: Don't unwrap obviously

            popup
                .borrow()
                .render(render_target, renderer, &state, theme_selector, position, screen_clip, false);
        };
    }

    pub fn render_anchors(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &Context<App>,
        theme_selector: App::ThemeSelector,
        available_space: App::Size,
    ) {
        self.anchor
            .render_window_anchors(render_target, renderer, state, theme_selector, self.position, self.size);
        self.anchor
            .render_screen_anchors(render_target, renderer, state, theme_selector, available_space);
    }
}

// Needed so that we can deallocate Window in another thread.
unsafe impl<App> Send for Window<App> where App: Application {}
unsafe impl<App> Sync for Window<App> where App: Application {}
