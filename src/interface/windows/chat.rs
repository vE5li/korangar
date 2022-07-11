use derive_new::new;
use std::rc::Rc;
use std::cell::RefCell;

use crate::types::ChatMessage;
use crate::types::maths::Vector2;
use crate::graphics::{ Renderer, Color };
use crate::interface::traits::{ Element, Window, PrototypeWindow };
use crate::interface::types::*;
use crate::interface::{ StateProvider, WindowCache, SizeConstraint, Size, Position };

#[derive(new)]
pub struct PrototypeChatWindow {
    messages: Rc<RefCell<Vec<ChatMessage>>>,
}

impl PrototypeWindow for PrototypeChatWindow{

    fn window_class(&self) -> Option<&str> {
        ChatWindow::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {
        Box::from(ChatWindow::new(window_cache, interface_settings, avalible_space, Rc::clone(&self.messages), constraint!(200.0 > 500.0 < 700.0, 100.0 > 100.0 < 600.0)))
    }
}

pub struct ChatWindow {
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    cached_message_count: usize,
}

impl ChatWindow {

    pub const WINDOW_CLASS: &'static str = "chat";

    pub fn new(window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size, messages: Rc<RefCell<Vec<ChatMessage>>>, size_constraint: SizeConstraint) -> Self {

        let (cached_position, cached_size) = window_cache.get_window_state(Self::WINDOW_CLASS).unzip();

        let size = cached_size
            .map(|size| size_constraint.validated_size(size, avalible_space, *interface_settings.scaling))
            .unwrap_or_else(|| size_constraint.resolve(avalible_space, avalible_space, *interface_settings.scaling).finalize_or(0.0));

        let position = cached_position
            .map(|position| size_constraint.validated_position(position, size, avalible_space))
            .unwrap_or(vector2!(0.0, avalible_space.y - size.y));

        let cached_message_count = messages.borrow().len();

        Self {
            position,
            size_constraint,
            size,
            messages,
            cached_message_count,
        }
    }
}

impl Window for ChatWindow {

    fn get_window_class(&self) -> Option<&str>{
        Self::WINDOW_CLASS.into()
    }

    fn has_transparency(&self, theme: &Theme) -> bool {
        theme.chat.background_color.alpha != 255
    }

    fn resolve(&mut self, _interface_settings: &InterfaceSettings, _theme: &Theme, _avalible_space: Size) -> (Option<&str>, Vector2<f32>, Size) {
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

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.size.x && absolute_position.y <= self.size.y {
            // TODO
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

        self_combined.x > position.x && self.position.x < area_combined.x && self_combined.y > position.y && self.position.y < area_combined.y
    }

    fn offset(&mut self, avalible_space: Size, offset: Position) -> Option<(&str, Position)> {
        self.position += offset;
        self.validate_position(avalible_space);
        (Self::WINDOW_CLASS, self.position).into()
    }

    fn validate_position(&mut self, avalible_space: Size) {
        self.position = self.size_constraint.validated_position(self.position, self.size, avalible_space);
    }

    fn resize(&mut self, interface_settings: &InterfaceSettings, _theme: &Theme, avalible_space: Size, growth: Size) -> (Option<&str>, Size) {
        self.size += growth;
        self.validate_size(interface_settings, avalible_space);
        (Self::WINDOW_CLASS.into(), self.size)
    }

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size) {
        self.size = self.size_constraint.validated_size(self.size, avalible_space, *interface_settings.scaling);
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, _hovered_element: Option<&dyn Element>) {
        renderer.render_rectangle(self.position, self.size, self.position + self.size, *theme.chat.border_radius, *theme.chat.background_color);

        let clip_size = self.position + self.size;
        let scaled_font_size = *theme.chat.font_size * *interface_settings.scaling;
        let scaled_shadow_offset = 1.0 * *interface_settings.scaling;

        for (message_index, message) in self.messages.borrow().iter().enumerate() {
            renderer.render_text(&message.text, self.position + vector2!(0.0, message_index as f32 * scaled_font_size) + vector2!(scaled_shadow_offset), clip_size, Color::monochrome(0), scaled_font_size);
            renderer.render_text(&message.text, self.position + vector2!(0.0, message_index as f32 * scaled_font_size), clip_size, message.color, scaled_font_size);
        }
    }
}
