use std::vec::Drain;

use rust_state::Context;

use super::ClickHandler;
use crate::MouseMode;
use crate::application::Application;
use crate::element::ElementBox;
use crate::element::id::{ElementId, FocusId};

pub enum Event<App: Application> {
    FocusElement {
        focus_id: FocusId,
    },
    /// This is an internal variant used to replace FocusElement in the event
    /// queue after the UI was built. Since we can only do the lookup form
    /// FocusId to ElementId while the Layout still exists.
    ///
    /// This is a bit hacky and might be reworked in the future.
    FocusElementPost {
        element_id: ElementId,
    },
    Unfocus,
    SetMouseMode {
        mouse_mode: MouseMode<App>,
    },
    Application {
        custom_event: App::CustomEvent,
    },
    OpenOverlay {
        element: ElementBox<App>,
        position: App::Position,
        size: App::Size,
        window_id: u64,
    },
    MoveWindowToTop {
        window_id: u64,
    },
    CloseWindow {
        window_id: u64,
    },
    CloseOverlay,
}

impl<App: Application> Clone for Event<App> {
    fn clone(&self) -> Self {
        match self {
            Self::FocusElement { focus_id } => Self::FocusElement { focus_id: *focus_id },
            Self::FocusElementPost { element_id } => Self::FocusElementPost { element_id: *element_id },
            Self::Unfocus => Self::Unfocus,
            // TODO: Find a better solution for this. Ideally Event wouldn't need to be clone.
            Self::SetMouseMode { .. } => unimplemented!(),
            Self::Application { custom_event } => Self::Application {
                custom_event: custom_event.clone(),
            },
            // TODO: Find a better solution for this. Ideally Event wouldn't need to be clone.
            Self::OpenOverlay { .. } => unimplemented!(),
            Self::MoveWindowToTop { window_id } => Self::MoveWindowToTop { window_id: *window_id },
            Self::CloseWindow { window_id } => Self::CloseWindow { window_id: *window_id },
            Self::CloseOverlay => Self::CloseOverlay,
        }
    }
}

impl<App: Application> ClickHandler<App> for Event<App> {
    fn execute(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
        queue.queue(self.clone());
    }
}

pub struct EventQueue<App: Application> {
    events: Vec<Event<App>>,
}

impl<App: Application> Default for EventQueue<App> {
    fn default() -> Self {
        Self {
            events: Default::default(),
        }
    }
}

impl<App: Application> EventQueue<App> {
    pub fn queue(&mut self, event: impl Into<Event<App>>) {
        self.events.push(event.into());
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Event<App>> {
        self.events.iter_mut()
    }

    pub fn drain(&mut self) -> Drain<'_, Event<App>> {
        self.events.drain(..)
    }
}
