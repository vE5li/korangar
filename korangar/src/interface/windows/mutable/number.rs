use std::cmp::PartialOrd;

use derive_new::new;
use korangar_interface::elements::{ElementWrap, Headline, Slider};
use korangar_interface::event::ChangeEvent;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(new)]
pub struct NumberWindow<T: 'static> {
    name: String,
    reference: &'static T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + 'static> PrototypeWindow<InterfaceSettings> for NumberWindow<T> {
    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![
            Headline::new("value".to_string(), size_bound!(100%, 12)).wrap(),
            Slider::new(self.reference, self.minimum_value, self.maximum_value, self.change_event).wrap(),
        ];

        WindowBuilder::new()
            .with_title(self.name.clone())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
