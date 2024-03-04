use std::cmp::PartialOrd;
use std::fmt::Display;

use derive_new::new;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::interface::*;

#[derive(new)]
pub struct ArrayWindow<T>
where
    T: ArrayType + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    name: String,
    reference: &'static T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
}

impl<T> PrototypeWindow for ArrayWindow<T>
where
    T: ArrayType + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let mut elements = Vec::new();

        let minimum_value = self.minimum_value.get_inner();
        let maximum_value = self.maximum_value.get_inner();

        for (index, (label, pointer)) in self.reference.get_array_fields().into_iter().enumerate() {
            elements.push(Headline::new(label, Headline::DEFAULT_SIZE).wrap());
            elements.push(Slider::new(pointer, minimum_value[index], maximum_value[index], self.change_event).wrap());
        }

        WindowBuilder::new()
            .with_title(self.name.clone())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
