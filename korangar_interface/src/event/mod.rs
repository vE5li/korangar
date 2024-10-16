mod action;

pub use self::action::{ClickAction, Toggle};
pub use self::queue::{Event, EventQueue};

mod queue {
    use std::vec::Drain;

    use rust_state::Context;

    use super::ClickAction;
    use crate::application::Appli;

    pub enum Event<App: Appli> {
        FocusNext,
        FocusPrevious,
        Application(App::Event),
    }

    impl<App: Appli> Clone for Event<App> {
        fn clone(&self) -> Self {
            match self {
                Self::FocusNext => Self::FocusNext,
                Self::FocusPrevious => Self::FocusPrevious,
                Self::Application(event) => Self::Application(event.clone()),
            }
        }
    }

    impl<App: Appli> ClickAction<App> for Event<App> {
        fn execute(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
            queue.queue(self.clone());
        }
    }

    pub struct EventQueue<App: Appli> {
        events: Vec<Event<App>>,
    }

    impl<App: Appli> EventQueue<App> {
        pub fn new() -> Self {
            Self { events: Vec::new() }
        }

        pub fn queue(&mut self, event: impl Into<Event<App>>) {
            self.events.push(event.into());
        }

        pub fn drain(&mut self) -> Drain<'_, Event<App>> {
            self.events.drain(..)
        }
    }
}
