use procedural::*;
use derive_new::new;
use num::{ Zero, NumCast };
use num::traits::NumOps;
use std::cmp::PartialOrd;

use crate::types::maths::*;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::{ InterfaceSettings, ChangeEvent };
use crate::interface::{ WindowCache, ElementCell, Size };
use crate::interface::elements::*;
use crate::interface::FramedWindow;

#[derive(new)]
pub struct Vector3Window<T> {
    name: String,
    inner_pointer: *const Vector3<T>,
    minimum_value: Vector3<T>,
    maximum_value: Vector3<T>,
    change_event: Option<ChangeEvent>,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + 'static> PrototypeWindow for Vector3Window<T> {

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Headline::new("x".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).x as *const T }, self.minimum_value.x, self.maximum_value.x, self.change_event)),
            cell!(Headline::new("y".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).y as *const T }, self.minimum_value.y, self.maximum_value.y, self.change_event)),
            cell!(Headline::new("z".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.inner_pointer).z as *const T }, self.minimum_value.z, self.maximum_value.z, self.change_event)),
        ];

        Box::new(FramedWindow::new(window_cache, interface_settings, avalible_space, self.name.clone(), None, elements, constraint!(200 > 250 < 300, ?)))
    }
}
