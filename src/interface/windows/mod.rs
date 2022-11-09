mod account;
mod builder;
mod cache;
mod character;
#[cfg(feature = "debug")]
mod debug;
mod generic;
mod mutable;
mod prototype;
mod settings;

use cgmath::{Vector2, Vector4};

pub use self::account::*;
pub use self::builder::WindowBuilder;
pub use self::cache::*;
pub use self::character::*;
#[cfg(feature = "debug")]
pub use self::debug::*;
pub use self::generic::*;
pub use self::mutable::*;
pub use self::prototype::PrototypeWindow;
pub use self::settings::*;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

pub struct Window {
    window_class: Option<String>,
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    elements: Vec<ElementCell>,
    closable: bool,
    background_color: Option<ColorSelector>,
}

impl Window {
    pub fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_deref()
    }

    fn get_background_color(&self, theme: &Theme) -> Color {
        self.background_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(*theme.window.background_color)
    }

    pub fn has_transparency(&self, theme: &Theme) -> bool {
        self.get_background_color(theme).alpha != 255
    }

    pub fn is_closable(&self) -> bool {
        self.closable
    }

    pub fn resolve(
        &mut self,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        avalible_space: Size,
    ) -> (Option<&str>, Vector2<f32>, Size) {
        let height = match self.size_constraint.height.is_flexible() {
            true => None,
            false => Some(self.size.y),
        };

        let mut placement_resolver = PlacementResolver::new(
            PartialSize::new(self.size.x, height),
            Vector2::new(0.0, 0.0),
            *theme.window.border_size,
            *theme.window.gaps,
            *interface_settings.scaling,
        );

        self.elements
            .iter()
            .for_each(|element| element.borrow_mut().resolve(&mut placement_resolver, interface_settings, theme));

        if self.size_constraint.height.is_flexible() {
            let final_height = theme.window.border_size.y + placement_resolver.final_height();
            let final_height = self.size_constraint.validated_height(
                final_height,
                avalible_space.y.into(),
                avalible_space.y.into(),
                *interface_settings.scaling,
            );
            self.size.y = final_height;
            self.validate_size(interface_settings, avalible_space);
        }

        self.validate_position(avalible_space);

        (self.window_class.as_deref(), self.position, self.size)
    }

    pub fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::combine).or(current).or(other)
            })
    }

    pub fn first_focused_element(&self) -> Option<ElementCell> {
        let element_cell = self.elements[0].clone();
        self.elements[0].borrow().focus_next(element_cell, None, Focus::downwards())
    }

    pub fn restore_focus(&self) -> Option<ElementCell> {
        self.elements[0].borrow().restore_focus(self.elements[0].clone())
    }

    pub fn hovered_element(&self, mouse_position: Vector2<f32>, mouse_mode: &MouseInputMode) -> HoverInformation {
        let absolute_position = mouse_position - self.position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.size.x
            && absolute_position.y <= self.size.y
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

    pub fn get_area(&self) -> (Position, Size) {
        (self.position, self.size)
    }

    pub fn hovers_area(&self, position: Position, size: Size) -> bool {
        let self_combined = self.position + self.size;
        let area_combined = position + size;

        self_combined.x > position.x
            && self.position.x < area_combined.x
            && self_combined.y > position.y
            && self.position.y < area_combined.y
    }

    pub fn offset(&mut self, avalible_space: Size, offset: Position) -> Option<(&str, Position)> {
        self.position += offset;
        self.validate_position(avalible_space);
        self.window_class
            .as_ref()
            .map(|window_class| (window_class.as_str(), self.position))
    }

    fn validate_position(&mut self, avalible_space: Size) {
        self.position = self.size_constraint.validated_position(self.position, self.size, avalible_space);
    }

    pub fn resize(
        &mut self,
        interface_settings: &InterfaceSettings,
        _theme: &Theme,
        avalible_space: Size,
        growth: Size,
    ) -> (Option<&str>, Size) {
        self.size += growth;
        self.validate_size(interface_settings, avalible_space);
        (self.window_class.as_deref(), self.size)
    }

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size) {
        self.size = self
            .size_constraint
            .validated_size(self.size, avalible_space, *interface_settings.scaling);
    }

    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
    ) {
        let clip_size = Vector4::new(
            self.position.x,
            self.position.y,
            self.position.x + self.size.x,
            self.position.y + self.size.y,
        );

        renderer.render_rectangle(
            render_target,
            self.position,
            self.size,
            clip_size,
            *theme.window.border_radius,
            self.get_background_color(theme),
        );

        self.elements.iter().for_each(|element| {
            element.borrow().render(
                render_target,
                renderer,
                state_provider,
                interface_settings,
                theme,
                self.position,
                clip_size,
                hovered_element,
                focused_element,
                mouse_mode,
                false,
            )
        });
    }
}

// Needed so that we can deallocate FramedWindow in another thread.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}
