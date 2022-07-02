use derive_new::new;
use num::{ Zero, NumCast };
use num::traits::NumOps;
use std::cmp::PartialOrd;
use std::fmt::Display;

use types::maths::*;
use graphics::Renderer;
use interface::traits::Element;
use interface::windows::Vector2Window;
use interface::types::*;

#[derive(new)]
pub struct MutableVector2Value<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> {
    name: String,
    inner_pointer: *const Vector2<T>,
    minimum_value: Vector2<T>,
    maximum_value: Vector2<T>,
    change_event: Option<ChangeEvent>,
    #[new(value = "Vector2::new(T::zero(), T::zero())")]
    cached_inner: Vector2<T>,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> Element for MutableVector2Value<T> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;

        //self.cached_inner = unsafe { *self.inner_pointer };
        //self.cached_values = format!("{:.1}, {:.1}", self.cached_inner.x, self.cached_inner.y);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_value = unsafe { *self.inner_pointer };

        if self.cached_inner != current_value {
            self.cached_inner = current_value;
            self.cached_values = format!("{:.1}, {:.1}", self.cached_inner.x, self.cached_inner.y);
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
        Some(ClickAction::OpenWindow(Box::new(Vector2Window::new(self.name.clone(), self.inner_pointer, self.minimum_value, self.maximum_value, self.change_event))))
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));

        match matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {
            true => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, *theme.value.hovered_background_color),
            false => renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, *theme.value.background_color),
        }

        renderer.render_text(&self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, *theme.value.foreground_color, *theme.value.font_size * *interface_settings.scaling);
    }
}
