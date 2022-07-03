use derive_new::new;
use num::{ Zero, NumCast, clamp };
use num::traits::NumOps;
use std::cmp::PartialOrd;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::Renderer;

#[derive(new)]
pub struct Slider<T: Zero + NumOps + NumCast + Copy + PartialOrd> {
    value_pointer: *const T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
    #[new(value = "T::zero()")]
    cached_value: T,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd> Element for Slider<T> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.slider.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;

        //self.cached_value = unsafe { *self.value_pointer };
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_value = unsafe { *self.value_pointer };

        if self.cached_value != current_value {
            self.cached_value = current_value;
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
        Some(ClickAction::DragElement)
    }

    fn drag(&mut self, mouse_delta: Position) -> Option<ChangeEvent> {

        let total_range = self.maximum_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap();
        let raw_value = self.cached_value.to_f32().unwrap() + (mouse_delta.x * total_range * 0.005);
        let new_value = clamp(raw_value, self.minimum_value.to_f32().unwrap(), self.maximum_value.to_f32().unwrap());

        unsafe { std::ptr::write(self.value_pointer as *mut T, T::from(new_value).unwrap()); }
        self.change_event
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));

        if matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ())) {
            renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.button.border_radius * *interface_settings.scaling, *theme.slider.background_color);
        }

        let bar_size = Size::new(self.cached_size.x * 0.9, self.cached_size.y / 4.0);
        let offset = (self.cached_size - bar_size) / 2.0;
        renderer.render_rectangle(absolute_position + offset, bar_size, clip_size, vector4!(0.5) * *interface_settings.scaling, *theme.slider.rail_color);

        let knob_size = Size::new(20.0 * *interface_settings.scaling, self.cached_size.y * 0.8);
        let total_range = self.maximum_value - self.minimum_value;
        let offset = Position::new((self.cached_size.x - knob_size.x) / total_range.to_f32().unwrap() * (self.cached_value.to_f32().unwrap() - self.minimum_value.to_f32().unwrap()), (self.cached_size.y - knob_size.y) / 2.0);
        renderer.render_rectangle(absolute_position + offset, knob_size, clip_size, vector4!(4.0) * *interface_settings.scaling, *theme.slider.knob_color);
    }
}
