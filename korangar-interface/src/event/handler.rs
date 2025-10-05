use rust_state::{Context, Path};

use super::EventQueue;
use crate::MouseMode;
use crate::application::Application;

/// Handler for mouse clicks.
pub trait ClickHandler<App: Application> {
    fn handle_click(&self, state: &Context<App>, queue: &mut EventQueue<App>);
}

impl<App, F> ClickHandler<App> for F
where
    App: Application,
    F: Fn(&Context<App>, &mut EventQueue<App>),
{
    fn handle_click(&self, state: &Context<App>, queue: &mut EventQueue<App>) {
        self(state, queue)
    }
}

pub struct Toggle<T>(pub T);

impl<T, App> ClickHandler<App> for Toggle<T>
where
    App: Application,
    T: Path<App, bool>,
{
    fn handle_click(&self, state: &Context<App>, _: &mut EventQueue<App>) {
        state.update_value_with(self.0, |value| {
            *value = !*value;
        });
    }
}

pub struct SetToTrue<T>(pub T);

impl<T, App> ClickHandler<App> for SetToTrue<T>
where
    App: Application,
    T: Path<App, bool>,
{
    fn handle_click(&self, state: &Context<App>, _: &mut EventQueue<App>) {
        state.update_value(self.0, true);
    }
}

pub struct SetToFalse<T>(pub T);

impl<T, App> ClickHandler<App> for SetToFalse<T>
where
    App: Application,
    T: Path<App, bool>,
{
    fn handle_click(&self, state: &Context<App>, _: &mut EventQueue<App>) {
        state.update_value(self.0, false);
    }
}

/// Handler for dropping a resource.
pub trait DropHandler<App: Application> {
    fn handle_drop(&self, state: &Context<App>, queue: &mut EventQueue<App>, mouse_mode: &MouseMode<App>);
}

/// Handler for scroll input.
pub trait ScrollHandler<App: Application> {
    fn handle_scroll(&self, state: &Context<App>, queue: &mut EventQueue<App>, delta: f32) -> bool;
}

/// Handler for receiving keyboard input.
pub trait InputHandler<App: Application> {
    fn handle_character(&self, state: &Context<App>, queue: &mut EventQueue<App>, character: char);
}
