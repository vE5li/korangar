mod anchor;
mod builder;
mod prototype;

use std::rc::Rc;

pub use self::anchor::{Anchor, AnchorPoint};
pub use self::builder::WindowBuilder;
pub use self::prototype::PrototypeWindow;
use crate::application::{Application, ClipTrait, ColorTrait, InterfaceRenderer, PositionTrait, PositionTraitExt, SizeTrait, SizeTraitExt};
use crate::elements::{Element, ElementCell, Focus};
use crate::event::{ChangeEvent, HoverInformation};
use crate::layout::{Dimension, PlacementResolver, SizeBound};
use crate::theme::{InterfaceTheme, WindowTheme};
use crate::{ColorSelector, Tracker};

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
    #[allow(clippy::type_complexity)]
    popup_element: Option<(ElementCell<App>, Tracker<App::Position>, Tracker<App::Size>)>,
    closable: bool,
    background_color: Option<ColorSelector<App>>,
    theme_kind: App::ThemeKind,
}

impl<App> Window<App>
where
    App: Application,
{
    pub fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_deref()
    }

    fn get_background_color(&self, theme: &App::Theme) -> App::Color {
        self.background_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(theme.window().background_color())
    }

    pub fn has_transparency(&self, theme: &App::Theme) -> bool {
        self.get_background_color(theme).is_transparent()
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
        application: &App,
        theme: &App::Theme,
        available_space: App::Size,
    ) -> App::Size {
        let mut placement_resolver = PlacementResolver::new(
            font_loader.clone(),
            available_space,
            self.size,
            &self.size_bound,
            theme.window().border_size(),
            theme.window().gaps(),
            application.get_scaling(),
        );

        self.elements
            .iter()
            .for_each(|element| element.borrow_mut().resolve(&mut placement_resolver, application, theme));

        if self.size_bound.height.is_flexible() {
            let parent_limits = placement_resolver.get_parent_limits();
            let final_height = placement_resolver.final_height();

            let final_height = self.size_bound.validated_height(
                final_height,
                available_space.height().into(),
                available_space.height().into(),
                &parent_limits,
                application.get_scaling(),
            );

            self.size = App::Size::new(self.size.width(), final_height);
            self.validate_size(application, available_space);
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
                application.get_scaling(),
            );

            popup.borrow_mut().resolve(&mut placement_resolver, application, theme);
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

    pub fn resize(&mut self, application: &App, available_space: App::Size, growth: App::Size) -> (Option<&str>, App::Size) {
        self.size = self.size.grow(growth);
        self.validate_size(application, available_space);
        (self.window_class.as_deref(), self.size)
    }

    fn validate_size(&mut self, application: &App, available_space: App::Size) {
        self.size = self
            .size_bound
            .validated_window_size(self.size, available_space, application.get_scaling());
    }

    pub fn open_popup(&mut self, element: ElementCell<App>, position_tracker: Tracker<App::Position>, size_tracker: Tracker<App::Size>) {
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
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
    ) {
        let screen_clip = App::Clip::new(
            self.position.left(),
            self.position.top(),
            self.position.left() + self.size.width(),
            self.position.top() + self.size.height(),
        );

        renderer.render_rectangle(
            self.position,
            self.size,
            screen_clip,
            theme.window().corner_radius(),
            self.get_background_color(theme),
        );

        self.elements.iter().for_each(|element| {
            element.borrow().render(
                renderer,
                application,
                theme,
                self.position,
                screen_clip,
                hovered_element,
                focused_element,
                mouse_mode,
                false,
            )
        });

        if let Some((popup, position_tracker, _)) = &self.popup_element {
            let position = position_tracker().unwrap(); // FIX: Don't unwrap obviously

            popup.borrow().render(
                renderer,
                application,
                theme,
                position,
                screen_clip,
                hovered_element,
                focused_element,
                mouse_mode,
                false,
            );
        };
    }

    pub fn render_anchors(&self, renderer: &App::Renderer, theme: &App::Theme, available_space: App::Size) {
        self.anchor.render_window_anchors(renderer, theme, self.position, self.size);
        self.anchor.render_screen_anchors(renderer, theme, available_space);
    }
}

// Needed so that we can deallocate Window in another thread.
unsafe impl<App> Send for Window<App> where App: Application {}
unsafe impl<App> Sync for Window<App> where App: Application {}
