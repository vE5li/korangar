use std::marker::PhantomData;

use super::Button;
use crate::application::Application;
use crate::builder::{Set, Unset};
use crate::layout::DimensionBound;
use crate::{ColorSelector, ElementEvent, Selector};

/// Type state [`Button`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Width>
where
    App: Application,
{
    text: Text,
    event: Event,
    disabled_selector: Option<Selector>,
    foreground_color: Option<ColorSelector<App>>,
    background_color: Option<ColorSelector<App>>,
    width_bound: DimensionBound,
    marker: PhantomData<(Disabled, Foreground, Background, Width)>,
}

impl<App> ButtonBuilder<App, Unset, Unset, Unset, Unset, Unset, Unset>
where
    App: Application,
{
    pub fn new() -> Self {
        Self {
            text: Unset,
            event: Unset,
            disabled_selector: None,
            foreground_color: None,
            background_color: None,
            width_bound: DimensionBound::RELATIVE_ONE_HUNDRED,
            marker: PhantomData,
        }
    }
}

impl<App, Event, Disabled, Foreground, Background, Width> ButtonBuilder<App, Unset, Event, Disabled, Foreground, Background, Width>
where
    App: Application,
{
    pub fn with_text<Text: AsRef<str> + 'static>(
        self,
        text: Text,
    ) -> ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Width> {
        ButtonBuilder { text, ..self }
    }
}

impl<App, Text, Disabled, Foreground, Background, Width> ButtonBuilder<App, Text, Unset, Disabled, Foreground, Background, Width>
where
    App: Application,
{
    pub fn with_event<Event: ElementEvent<App> + 'static>(
        self,
        event: Event,
    ) -> ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Width> {
        ButtonBuilder { event, ..self }
    }
}

impl<App, Text, Event, Foreground, Background, Width> ButtonBuilder<App, Text, Event, Unset, Foreground, Background, Width>
where
    App: Application,
{
    pub fn with_disabled_selector(
        self,
        selector: impl Fn() -> bool + 'static,
    ) -> ButtonBuilder<App, Text, Event, Set, Foreground, Background, Width> {
        ButtonBuilder {
            disabled_selector: Some(Box::new(selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, Disabled, Background, Width> ButtonBuilder<App, Text, Event, Disabled, Unset, Background, Width>
where
    App: Application,
{
    pub fn with_foreground_color(
        self,
        color_selector: impl Fn(&App::Theme) -> App::Color + 'static,
    ) -> ButtonBuilder<App, Text, Event, Disabled, Set, Background, Width> {
        ButtonBuilder {
            foreground_color: Some(Box::new(color_selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, Disabled, Foreground, Width> ButtonBuilder<App, Text, Event, Disabled, Foreground, Unset, Width>
where
    App: Application,
{
    pub fn with_background_color(
        self,
        color_selector: impl Fn(&App::Theme) -> App::Color + 'static,
    ) -> ButtonBuilder<App, Text, Event, Disabled, Foreground, Set, Width> {
        ButtonBuilder {
            background_color: Some(Box::new(color_selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, Disabled, Foreground, Background> ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Unset>
where
    App: Application,
{
    pub fn with_width_bound(self, width_bound: DimensionBound) -> ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Set> {
        ButtonBuilder {
            width_bound,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, Disabled, Foreground, Background, Width> ButtonBuilder<App, Text, Event, Disabled, Foreground, Background, Width>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
{
    /// Take the builder and turn it into a [`Button`].
    ///
    /// NOTE: This method is only available if [`with_text`](Self::with_text)
    /// and [`with_event`](Self::with_event) have been called on
    /// the builder.
    pub fn build(self) -> Button<App, Text, Event> {
        let Self {
            text,
            event,
            disabled_selector,
            foreground_color,
            background_color,
            width_bound,
            ..
        } = self;

        Button {
            text,
            event,
            disabled_selector,
            foreground_color,
            background_color,
            width_bound,
            state: Default::default(),
        }
    }
}
