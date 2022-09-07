use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Array, Vector2, Vector4};
use derive_new::new;
use procedural::*;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::UserEvent;
use crate::interface::{Element, Position, PrototypeWindow, Size, SizeConstraint, StateProvider, Window, WindowCache, *};
use crate::network::ChatMessage;

#[derive(new)]
pub struct PrototypeChatWindow {
    messages: Rc<RefCell<Vec<ChatMessage>>>,
}

impl PrototypeWindow for PrototypeChatWindow {

    fn window_class(&self) -> Option<&str> {
        ChatWindow::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {

        Box::from(ChatWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            self.messages.clone(),
            constraint!(200 > 500 < 700, 100 > 100 < 600),
        ))
    }
}

pub struct ChatWindow {
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    elements: Vec<ElementCell>,
    cached_message_count: usize,
}

impl ChatWindow {

    pub const WINDOW_CLASS: &'static str = "chat";

    pub fn new(
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
        messages: Rc<RefCell<Vec<ChatMessage>>>,
        size_constraint: SizeConstraint,
    ) -> Self {

        let (cached_position, cached_size) = window_cache.get_window_state(Self::WINDOW_CLASS).unzip();

        let size = cached_size
            .map(|size| size_constraint.validated_size(size, avalible_space, *interface_settings.scaling))
            .unwrap_or_else(|| {

                size_constraint
                    .resolve(avalible_space, avalible_space, *interface_settings.scaling)
                    .finalize_or(0.0)
            });

        let position = cached_position
            .map(|position| size_constraint.validated_position(position, size, avalible_space))
            .unwrap_or(Vector2::new(0.0, avalible_space.y - size.y));

        let cached_message_count = messages.borrow().len();

        let input_text = Rc::new(RefCell::new(String::new()));

        let button_selector = {

            let input_text = input_text.clone();
            Box::new(move || !input_text.borrow().is_empty())
        };

        let button_action = {

            let input_text = input_text.clone();
            Box::new(move || {

                let message = input_text.borrow_mut().drain(..).collect();
                UserEvent::SendMessage(message)
            })
        };

        let input_action = {

            let input_text = input_text.clone();
            Box::new(move || {
                //input_text.borrow().is_empty().not().then_some(ChangeEvent::LeftClickNext)
                None
            })
        };

        let elements: Vec<ElementCell> = vec![
            cell!(InputField::<30>::new(input_text, "write message or command", input_action)) as _,
            cell!(FormButton::new("send", button_selector, button_action)) as _,
        ];

        // very imporant: give every element a link to its parent to allow propagation of events such as
        // scrolling
        elements.iter().for_each(|element| {

            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, None);
        });

        Self {
            position,
            size_constraint,
            size,
            messages,
            elements,
            cached_message_count,
        }
    }
}

impl Window for ChatWindow {

    fn get_window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn has_transparency(&self, theme: &Theme) -> bool {
        theme.chat.background_color.alpha != 255
    }

    fn resolve(
        &mut self,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        avalible_space: Size,
    ) -> (Option<&str>, Vector2<f32>, Size) {

        let mut placement_resolver = PlacementResolver::new(
            PartialSize::new(self.size.x, self.size.y.into()),
            Vector2::new(0.0, 0.0),
            *theme.window.border_size,
            *theme.window.gaps,
            *interface_settings.scaling,
        );

        self.elements
            .iter_mut()
            .for_each(|element| element.borrow_mut().resolve(&mut placement_resolver, interface_settings, theme));

        self.validate_position(avalible_space);

        (Self::WINDOW_CLASS.into(), self.position, self.size)
    }

    fn update(&mut self) -> Option<ChangeEvent> {

        let messages = self.messages.borrow();
        if messages.len() != self.cached_message_count {

            self.cached_message_count = messages.len();
            return ChangeEvent::RerenderWindow.into();
        }

        None
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> HoverInformation {

        let absolute_position = mouse_position - self.position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.size.x
            && absolute_position.y <= self.size.y
        {

            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Element(element) => return HoverInformation::Element(element),
                    HoverInformation::Missed => {}
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
        (Self::WINDOW_CLASS, self.position).into()
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
        (Self::WINDOW_CLASS.into(), self.size)
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
    ) {

        let clip_size = Vector4::new(
            self.position.x,
            self.position.y,
            self.position.x + self.size.x,
            self.position.y + self.size.y,
        );
        let scaled_font_size = *theme.chat.font_size * *interface_settings.scaling;
        let scaled_shadow_offset = 1.0 * *interface_settings.scaling;

        renderer.render_rectangle(
            render_target,
            self.position,
            self.size,
            clip_size,
            *theme.chat.border_radius,
            *theme.chat.background_color,
        );

        for (message_index, message) in self.messages.borrow().iter().enumerate() {

            renderer.render_text(
                render_target,
                &message.text,
                self.position + Vector2::new(0.0, message_index as f32 * scaled_font_size) + Vector2::from_value(scaled_shadow_offset),
                clip_size,
                Color::monochrome(0),
                scaled_font_size,
            );

            renderer.render_text(
                render_target,
                &message.text,
                self.position + Vector2::new(0.0, message_index as f32 * scaled_font_size),
                clip_size,
                message.color,
                scaled_font_size,
            );
        }

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
                false,
            )
        });
    }
}
