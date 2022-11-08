use cgmath::{Vector2, Vector4};
use procedural::dimension;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

#[derive(Default)]
pub struct WindowBuilder {
    window_title: Option<String>,
    window_class: Option<String>,
    size_constraint: SizeConstraint,
    elements: Vec<ElementCell>,
    closable: bool,
    background_color: Option<ColorSelector>,
}

impl WindowBuilder {
    pub fn with_title(mut self, window_title: String) -> Self {
        assert!(self.window_title.is_none()); // TODO: do this check everywhere?
        self.window_title = Some(window_title);
        self
    }

    pub fn with_class(mut self, window_class: String) -> Self {
        assert!(self.window_class.is_none());
        self.window_class = Some(window_class);
        self
    }

    /// To simplify PrototypeWindow proc macro. Migth be removed later
    pub fn with_class_option(self, window_class: Option<String>) -> Self {
        Self { window_class, ..self }
    }

    pub fn with_size(self, size_constraint: SizeConstraint) -> Self {
        Self { size_constraint, ..self }
    }

    pub fn with_elements(self, elements: Vec<ElementCell>) -> Self {
        Self { elements, ..self }
    }

    pub fn with_background_color(mut self, background_color: ColorSelector) -> Self {
        self.background_color = Some(background_color);
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    pub fn build(self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let WindowBuilder {
            window_title,
            window_class,
            size_constraint,
            mut elements,
            closable,
            background_color,
        } = self;

        if closable {
            assert!(window_title.is_some(), "closable window must also have a title");
            let close_button = cell!(CloseButton::default());
            elements.insert(0, close_button);
        }

        let width_constraint = match closable {
            true => dimension!(70%),
            false => dimension!(!),
        };

        if let Some(title) = window_title {
            let drag_button = cell!(DragButton::new(title, width_constraint));
            elements.insert(0, drag_button);
        }

        let elements = vec![Container::new(elements).wrap()];

        // very imporant: give every element a link to its parent to allow propagation
        // of events such as scrolling
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

        Window {
            window_class,
            position,
            size_constraint,
            size,
            elements,
            closable,
            background_color,
        }
    }
}

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

    pub fn has_transparency(&self, theme: &Theme) -> bool {
        theme.window.background_color.alpha != 255
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

    pub fn validate_position(&mut self, avalible_space: Size) {
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

    pub fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size) {
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

        let background_color = self
            .background_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(*theme.window.background_color);

        renderer.render_rectangle(
            render_target,
            self.position,
            self.size,
            clip_size,
            *theme.window.border_radius,
            background_color,
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
