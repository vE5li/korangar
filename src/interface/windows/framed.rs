use cgmath::{Vector2, Vector4};
use procedural::dimension;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{
    CloseButton, DragButton, Element, ElementCell, PartialSize, Position, Size, SizeConstraint, StateProvider, Window, WindowCache, *,
};

pub struct FramedWindow {
    window_class: Option<String>,
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    elements: Vec<ElementCell>,
}

impl FramedWindow {

    pub fn new(
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
        window_title: String,
        window_class: Option<String>,
        mut elements: Vec<ElementCell>,
        size_constraint: SizeConstraint,
        closeable: bool,
    ) -> Self {

        if closeable {
            let close_button = cell!(CloseButton::default());
            elements.insert(0, close_button);
        }

        let width_constraint = match closeable {
            true => dimension!(70%),
            false => dimension!(!),
        };

        let drag_button = cell!(DragButton::new(window_title, width_constraint));
        elements.insert(0, drag_button);

        let elements = vec![Container::new(elements).wrap()];

        // very imporant: give every element a link to its parent to allow propagation of events such as
        // scrolling
        elements.iter().for_each(|element| {

            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, None);
        });

        let (cached_position, cached_size) = window_class
            .as_ref()
            .and_then(|window_class| window_cache.get_window_state(window_class))
            .unzip();

        let size = cached_size
            .map(|size| size_constraint.validated_size(size, avalible_space, *interface_settings.scaling))
            .unwrap_or_else(|| {

                size_constraint
                    .resolve(avalible_space, avalible_space, *interface_settings.scaling)
                    .finalize_or(0.0)
            });

        let position = cached_position
            .map(|position| size_constraint.validated_position(position, size, avalible_space))
            .unwrap_or((avalible_space - size) / 2.0);

        Self {
            window_class,
            position,
            size_constraint,
            size,
            elements,
        }
    }
}

impl Window for FramedWindow {

    fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_deref()
    }

    fn has_transparency(&self, theme: &Theme) -> bool {
        theme.window.background_color.alpha != 255
    }

    fn resolve(
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

    fn update(&mut self) -> Option<ChangeEvent> {

        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::combine).or(current).or(other)
            })
    }

    fn first_focused_element(&self) -> Option<ElementCell> {

        let element_cell = self.elements[0].clone();
        self.elements[0].borrow().focus_next(element_cell, None, Focus::downwards())
    }

    fn restore_focus(&self) -> Option<ElementCell> {
        self.elements[0].borrow().restore_focus(self.elements[0].clone())
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>, mouse_mode: &MouseInputMode) -> HoverInformation {

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

    fn get_area(&self) -> (Position, Size) {
        (self.position, self.size)
    }

    fn hovers_area(&self, position: Position, size: Size) -> bool {

        let self_combined = self.position + self.size;
        let area_combined = position + size;

        self_combined.x > position.x
            && self.position.x < area_combined.x
            && self_combined.y > position.y
            && self.position.y < area_combined.y
    }

    fn offset(&mut self, avalible_space: Size, offset: Position) -> Option<(&str, Position)> {

        self.position += offset;
        self.validate_position(avalible_space);
        self.window_class
            .as_ref()
            .map(|window_class| (window_class.as_str(), self.position))
    }

    fn validate_position(&mut self, avalible_space: Size) {
        self.position = self.size_constraint.validated_position(self.position, self.size, avalible_space);
    }

    fn resize(
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

    fn render(
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
            *theme.window.background_color,
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
