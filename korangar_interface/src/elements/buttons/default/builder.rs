use std::borrow::Cow;
use std::marker::PhantomData;

use rust_state::{Context, SafeUnwrap, Selector};

use crate::application::Application;
use crate::builder::{Set, Unset};
use crate::elements::base::VTable;
use crate::elements::{Element2, ElementAllocator, ElementHandle, Focusable, HoverCheck, ModeCheck, Resolve, World};
use crate::event::ClickAction;
use crate::layout::DimensionBound;
use crate::{ClickEvaluator, ColorEvaluator, DimensionBoundEvaluator, DisabledEvaluator, TextEvaluator};

pub struct ButtonBuilder<App, Background, Foreground, Width, Event, Disabled, Text>
where
    App: Application,
{
    background_color: Option<ColorEvaluator<App>>,
    foreground_color: Option<ColorEvaluator<App>>,
    width_bound: Option<DimensionBoundEvaluator<App>>,
    click_event: Event,
    disabled: Option<DisabledEvaluator<App>>,
    text: Text,
    _marker: PhantomData<(Background, Foreground, Width, Disabled)>,
}

impl<App: Application> ButtonBuilder<App, Unset, Unset, Unset, Unset, Unset, Unset> {
    pub fn new() -> Self {
        Self {
            background_color: None,
            foreground_color: None,
            width_bound: None,
            click_event: Unset,
            disabled: None,
            text: Unset,
            _marker: PhantomData,
        }
    }
}

impl<App, Foreground, Width, Event, Disabled, Text> ButtonBuilder<App, Unset, Foreground, Width, Event, Disabled, Text>
where
    App: Application,
{
    pub fn background_color(
        self,
        evaluator: impl Fn(&World<App>) -> App::Color,
    ) -> ButtonBuilder<App, Set, Foreground, Width, Event, Disabled, Text> {
        ButtonBuilder {
            background_color: Some(Box::new(evaluator)),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Foreground, Width, Event, Disabled, Text> ButtonBuilder<App, Unset, Foreground, Width, Event, Disabled, Text>
where
    App: Application,
{
    pub fn background_color_selector(
        self,
        selector: impl Selector<App, App::Color> + SafeUnwrap,
    ) -> ButtonBuilder<App, Set, Foreground, Width, Event, Disabled, Text> {
        ButtonBuilder {
            background_color: Some(Box::new(move |world| world.global.get_safe(&selector).clone())),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Width, Event, Disabled, Text> ButtonBuilder<App, Background, Unset, Width, Event, Disabled, Text>
where
    App: Application,
{
    pub fn foreground_color(
        self,
        evaluator: impl Fn(&World<App>) -> App::Color,
    ) -> ButtonBuilder<App, Background, Set, Width, Event, Disabled, Text> {
        ButtonBuilder {
            foreground_color: Some(Box::new(evaluator)),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Width, Event, Disabled, Text> ButtonBuilder<App, Background, Unset, Width, Event, Disabled, Text>
where
    App: Application,
{
    pub fn foreground_color_selector(
        self,
        selector: impl Selector<App, App::Color> + SafeUnwrap,
    ) -> ButtonBuilder<App, Background, Set, Width, Event, Disabled, Text> {
        ButtonBuilder {
            foreground_color: Some(Box::new(move |world| world.global.get_safe(&selector).clone())),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Event, Disabled, Text> ButtonBuilder<App, Background, Foreground, Unset, Event, Disabled, Text>
where
    App: Application,
{
    pub fn width_bound(
        self,
        evaluator: impl Fn(&World<App>) -> DimensionBound,
    ) -> ButtonBuilder<App, Background, Foreground, Set, Event, Disabled, Text> {
        ButtonBuilder {
            width_bound: Some(Box::new(evaluator)),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Event, Disabled, Text> ButtonBuilder<App, Background, Foreground, Unset, Event, Disabled, Text>
where
    App: Application,
{
    pub fn width_bound_selector(
        self,
        selector: impl Selector<App, DimensionBound> + SafeUnwrap,
    ) -> ButtonBuilder<App, Background, Foreground, Set, Event, Disabled, Text> {
        ButtonBuilder {
            width_bound: Some(Box::new(move |world| world.global.get_safe(&selector).clone())),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Width, Disabled, Text> ButtonBuilder<App, Background, Foreground, Width, Unset, Disabled, Text>
where
    App: Application,
{
    pub fn click_event(
        self,
        click_event: impl Fn(&World<App>) -> Vec<ClickAction<App>>,
    ) -> ButtonBuilder<App, Background, Foreground, Width, ClickEvaluator<App>, Disabled, Text> {
        ButtonBuilder {
            click_event: Box::new(click_event),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Width, Event, Text> ButtonBuilder<App, Background, Foreground, Width, Event, Unset, Text>
where
    App: Application,
{
    pub fn disabled(self, evaluator: impl Fn(&World<App>) -> bool) -> ButtonBuilder<App, Background, Foreground, Width, Event, Set, Text> {
        ButtonBuilder {
            disabled: Some(Box::new(evaluator)),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Width, Event, Text> ButtonBuilder<App, Background, Foreground, Width, Event, Unset, Text>
where
    App: Application,
{
    pub fn disabled_selector(
        self,
        selector: impl Selector<App, bool> + SafeUnwrap,
    ) -> ButtonBuilder<App, Background, Foreground, Width, Event, Set, Text> {
        ButtonBuilder {
            disabled: Some(Box::new(move |world| world.global.get_safe(&selector).clone())),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Width, Event, Disabled> ButtonBuilder<App, Background, Foreground, Width, Event, Disabled, Unset>
where
    App: Application,
{
    pub fn text(
        self,
        evaluator: impl for<'a> Fn(&'a World<'a, App>) -> Cow<'a, str>,
    ) -> ButtonBuilder<App, Background, Foreground, Width, Event, Disabled, TextEvaluator<App>> {
        ButtonBuilder {
            text: Box::new(evaluator),
            _marker: PhantomData,
            ..self
        }
    }
}

impl<App, Background, Foreground, Width, Disabled>
    ButtonBuilder<App, Background, Foreground, Width, ClickEvaluator<App>, Disabled, TextEvaluator<App>>
where
    App: Application,
{
    pub fn build(
        self,
        state: &Context<App>,
        allocator: &mut ElementAllocator<App>,
        parent_handle: Option<ElementHandle>,
        theme_selector: App::ThemeSelector,
    ) -> ElementHandle {
        let vtable = const {
            &VTable {
                on_initialize: None,
                on_is_focusable: Focusable::Dynamic(super::focusable::<App>),
                on_resolve: Resolve::Default(super::size_bound::<App>),
                hover_check: HoverCheck::Default,
                mode_check: ModeCheck::Default,
                on_left_click: Some(super::on_click::<App>),
                on_right_click: None,
                on_drag: None,
                on_input_character: None,
                on_drop_resource: None,
                on_scroll: None,
                background: Some(super::background_color_thing::<App>),
                render: super::render::<App>,
            }
        };

        let ButtonBuilder {
            background_color,
            foreground_color,
            width_bound,
            click_event,
            disabled,
            text,
            ..
        } = self;

        let button_state = super::ButtonState {
            background_color,
            foreground_color,
            width_bound,
            click_event,
            disabled,
            text,
        };

        Element2::new(
            vtable,
            Some(Box::new(button_state)),
            state,
            allocator,
            parent_handle,
            theme_selector,
        )
    }
}
