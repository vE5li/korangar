use std::cmp::PartialOrd;
use std::fmt::Display;

use derive_new::new;
use korangar_interface::elements::{ElementWrap, Headline, Slider};
use korangar_interface::event::ChangeEvent;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, ScreenSize};
use crate::interface::windows::WindowCache;

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

impl<T> PrototypeWindow<InterfaceSettings> for ArrayWindow<T>
where
    T: ArrayType + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let mut elements = Vec::new();

        let minimum_value = self.minimum_value.get_inner();
        let maximum_value = self.maximum_value.get_inner();

        for (index, (label, pointer)) in self.reference.get_array_fields().into_iter().enumerate() {
            elements.push(Headline::new(label, size_bound!(100%, 12)).wrap());
            elements.push(Slider::new(pointer, minimum_value[index], maximum_value[index], self.change_event).wrap());
        }

        WindowBuilder::new()
            .with_title(self.name.clone())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
