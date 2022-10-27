mod close;
mod default;
mod drag;
mod state;

pub use self::close::CloseButton;
pub use self::default::Button;
pub use self::drag::DragButton;
pub use self::state::StateButton;
use crate::input::UserEvent;
use crate::interface::ClickAction;

enum ElementText {
    Static(&'static str),
    Dynamic(String),
}

impl ElementText {

    pub fn get_str(&self) -> &str {
        match self {
            Self::Static(text) => text,
            Self::Dynamic(text) => text,
        }
    }
}

enum ElementEvent {
    Event(UserEvent),
    ActionClosure(Box<dyn Fn() -> Option<ClickAction>>),
    Closure(Box<dyn FnMut()>),
}

impl ElementEvent {

    pub fn execute(&mut self) -> Option<ClickAction> {
        match self {

            Self::Event(user_event) => Some(ClickAction::Event(user_event.clone())),

            Self::ActionClosure(action_closure) => action_closure(),

            Self::Closure(closure) => {

                closure();
                None
            }
        }
    }
}
