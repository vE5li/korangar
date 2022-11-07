use std::cmp::PartialOrd;
use std::fmt::Display;

use cgmath::Array;
use derive_new::new;
use num::traits::NumOps;
use num::{NumCast, Zero};
use procedural::*;

use crate::interface::*;

#[derive(new)]
pub struct VectorWindow<T>
where
    T: Array,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{
    name: String,
    inner_pointer: *const T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
}

impl<T> PrototypeWindow for VectorWindow<T>
where
    T: Array,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        const LABELS: [char; 4] = ['x', 'y', 'z', 'w'];

        let mut elements = Vec::new();
        let inner_value = unsafe { &*self.inner_pointer };

        for index in 0..<T as Array>::len() {
            let label = LABELS[index].to_string();
            let pointer = &inner_value[index] as *const T::Element;

            elements.push(cell!(Headline::new(label, Headline::DEFAULT_SIZE)) as _);
            elements.push(cell!(Slider::new(
                pointer,
                self.minimum_value[index],
                self.maximum_value[index],
                self.change_event
            )) as _);
        }

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            self.name.clone(),
            None,
            elements,
            constraint!(200 > 250 < 300, ?),
            true,
        )
    }
}
