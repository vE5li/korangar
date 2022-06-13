use derive_new::new;
use num::{ Zero, NumCast };
use num::traits::NumOps;
use std::cmp::PartialOrd;

use types::maths::*;
use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::{ WindowCache, ElementCell, Size };
use interface::elements::*;
use interface::FramedWindow;

#[derive(new)]
pub struct Vector4Window<T> {
    name: String,
    inner_pointer: *const Vector4<T>,
    minimum_value: Vector4<T>,
    maximum_value: Vector4<T>,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + 'static> PrototypeWindow for Vector4Window<T> {

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Headline::new(self.name.clone(), Headline::DEFAULT_SIZE)),
            cell!(Headline::new("x".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).x as *const T }, self.minimum_value.x, self.maximum_value.x)),
            cell!(Headline::new("y".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).y as *const T }, self.minimum_value.y, self.maximum_value.y)),
            cell!(Headline::new("z".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).z as *const T }, self.minimum_value.z, self.maximum_value.z)),
            cell!(Headline::new("w".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).w as *const T }, self.minimum_value.w, self.maximum_value.y)),
        ];

        Box::new(FramedWindow::new(window_cache, interface_settings, avalible_space, "vector 4".to_string(), None, elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
