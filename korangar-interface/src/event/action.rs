use rust_state::{Context, Path};

use super::EventQueue;
use crate::application::Application;

pub trait ClickHandler<App: Application> {
    fn execute(&self, state: &Context<App>, queue: &mut EventQueue<App>);
}

impl<App, F> ClickHandler<App> for F
where
    App: Application,
    F: Fn(&Context<App>, &mut EventQueue<App>),
{
    fn execute(&self, state: &Context<App>, queue: &mut EventQueue<App>) {
        self(state, queue)
    }
}

pub struct Toggle<T>(pub T);

impl<T, App> ClickHandler<App> for Toggle<T>
where
    App: Application,
    T: Path<App, bool>,
{
    fn execute(&self, state: &Context<App>, _: &mut EventQueue<App>) {
        state.update_value_with(self.0, |value| {
            *value = !*value;
        });
    }
}
