mod layout;
mod hover;
mod action;
mod event;
mod provider;
mod theme;
mod cache;
mod wrappers;
mod settings;

pub use self::layout::*;
pub use self::hover::HoverInformation;
pub use self::action::ClickAction;
pub use self::event::*;
pub use self::provider::StateProvider;
pub use self::theme::Theme;
pub use self::cache::WindowCache;
pub use self::wrappers::*;
pub use self::settings::InterfaceSettings;

use crate::interface::traits::Element;
use std::rc::Rc;
use std::cell::RefCell;

pub type ElementCell = Rc<RefCell<dyn Element>>;

macro_rules! cell {
    ($element:expr) => {
        std::rc::Rc::new(std::cell::RefCell::new($element))
    };
}
