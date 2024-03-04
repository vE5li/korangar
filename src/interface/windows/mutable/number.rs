use std::cmp::PartialOrd;

use derive_new::new;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::interface::*;

#[derive(new)]
pub struct NumberWindow<T: 'static> {
    name: String,
    reference: &'static T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + 'static> PrototypeWindow for NumberWindow<T> {
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![
            Headline::new("value".to_string(), Headline::DEFAULT_SIZE).wrap(),
            Slider::new(self.reference, self.minimum_value, self.maximum_value, self.change_event).wrap(),
        ];

        WindowBuilder::default()
            .with_title(self.name.clone())
            .with_size(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
