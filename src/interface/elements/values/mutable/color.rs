use derive_new::new;
use num::Zero;

use crate::graphics::{ Renderer, Color };
use crate::interface::traits::Element;
use crate::interface::windows::ColorWindow;
use crate::interface::types::*;

#[derive(new)]
pub struct MutableColorValue {
    name: String,
    color_pointer: *const Color,
    change_event: Option<ChangeEvent>,
    #[new(default)]
    cached_color: Color,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for MutableColorValue {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;

        //self.cached_color = unsafe { *self.color_pointer };
        //self.cached_values = format!("{}, {}, {}", self.cached_color.red, self.cached_color.green, self.cached_color.blue);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_color = unsafe { *self.color_pointer };

        if self.cached_color != current_color {
            self.cached_color = current_color;
            self.cached_values = format!("{}, {}, {}", self.cached_color.red, self.cached_color.green, self.cached_color.blue);
            return Some(ChangeEvent::RerenderWindow);
        }

        None
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        Some(ClickAction::OpenWindow(Box::new(ColorWindow::new(self.name.clone(), self.color_pointer, self.change_event))))
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));

        match matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {
            true => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, self.cached_color.shade()),
            false => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, self.cached_color),
        }

        renderer.render_text(&self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, self.cached_color.invert(), *theme.value.font_size * *interface_settings.scaling);
    }
}
