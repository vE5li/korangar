mod builder;

use interface_procedural::dimension_bound;
use rust_state::RustState;

pub use self::builder::ButtonBuilder;
use crate::application::Application;
use crate::elements::{Element2State, ElementRenderer, World};
use crate::event::ClickAction;
use crate::layout::SizeBound;
use crate::theme::ButtonTheme;
use crate::{ClickEvaluator, ColorEvaluator, DimensionBoundEvaluator, DisabledEvaluator, TextEvaluator};

#[derive(RustState)]
struct ButtonState<App>
where
    App: Application,
{
    background_color: Option<ColorEvaluator<App>>,
    foreground_color: Option<ColorEvaluator<App>>,
    width_bound: Option<DimensionBoundEvaluator<App>>,
    click_event: ClickEvaluator<App>,
    disabled: Option<DisabledEvaluator<App>>,
    text: TextEvaluator<App>,
}

fn focusable<App: Application>(world: &World<App>) -> bool {
    !world
        .evaluator_option(&ButtonState::<App>::disabled(Element2State::<App>::custom()))
        .unwrap_or_default()
}

fn size_bound<App: Application>(world: &World<App>) -> SizeBound {
    let width_bound = world
        .evaluator_option(&ButtonState::<App>::width_bound(Element2State::<App>::custom()))
        .unwrap_or(dimension_bound!(100%));
    let height_bound = world.global.get_safe(&ButtonTheme::height_bound(world.theme_selector)).clone();

    width_bound.add_height(height_bound)
}

fn on_click<App: Application>(world: &World<App>) -> Vec<ClickAction<App>> {
    world.evaluator(&ButtonState::<App>::click_event(Element2State::<App>::custom()))
}

fn background_color_thing<App: Application>(world: &World<App>) -> (App::Color, App::CornerRadius) {
    // TODO: Disabled etc.

    let color = world.evaluator_option_fallback(
        &ButtonState::<App>::background_color(Element2State::<App>::custom()),
        &ButtonTheme::background_color(world.theme_selector),
    );

    let corner_radius = world
        .global
        .get_safe(&ButtonTheme::<App>::corner_radius(world.theme_selector))
        .clone();

    (color, corner_radius)
}

fn render<App: Application>(world: &World<App>, renderer: &mut ElementRenderer<App>) {
    let disabled = world
        .evaluator_option(&ButtonState::<App>::disabled(Element2State::<App>::custom()))
        .unwrap_or_default();

    let foreground_color = if disabled {
        world
            .global
            .get_safe(&ButtonTheme::disabled_foreground_color(world.theme_selector))
            .clone()
    } else {
        world.evaluator_option_fallback(
            &ButtonState::<App>::foreground_color(Element2State::<App>::custom()),
            &ButtonTheme::foreground_color(world.theme_selector),
        )
    };

    let text = world.evaluator(&ButtonState::<App>::text(Element2State::<App>::custom()));
    let text_offset = world.global.get_safe(&ButtonTheme::text_offset(world.theme_selector));
    let font_size = world.global.get_safe(&ButtonTheme::font_size(world.theme_selector));

    renderer.render_text(text.as_ref(), *text_offset, foreground_color, *font_size);
}
