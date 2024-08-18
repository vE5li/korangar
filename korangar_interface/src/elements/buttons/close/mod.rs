mod builder;

pub use self::builder::CloseButtonBuilder;
use crate::application::Application;
use crate::elements::{ElementRenderer, World};
use crate::event::ClickAction;
use crate::layout::{PlacementResolver, SizeBound};
use crate::theme::CloseButtonTheme;

fn size_bound<App: Application>(world: &World<App>) -> SizeBound {
    world.global.get_safe(&CloseButtonTheme::size_bound(world.theme_selector)).clone()
}

fn on_click<App: Application>(world: &World<App>) -> Vec<ClickAction<App>> {
    vec![ClickAction::CloseWindow]
}

fn resolve<App: Application>(world: &World<App>, placement_resolver: &mut PlacementResolver<App>) {}

fn background_color_thing<App: Application>(world: &World<App>) -> (App::Color, App::CornerRadius) {
    // TODO: Disabled etc.

    let color = world
        .global
        .get_safe(&CloseButtonTheme::background_color(world.theme_selector))
        .clone();
    let corner_radius = world
        .global
        .get_safe(&CloseButtonTheme::<App>::corner_radius(world.theme_selector))
        .clone();

    (color, corner_radius)
}

fn render<App: Application>(world: &World<App>, renderer: &mut ElementRenderer<App>) {
    let foreground_color = world
        .global
        .get_safe(&CloseButtonTheme::foreground_color(world.theme_selector))
        .clone();

    let text_offset = world.global.get_safe(&CloseButtonTheme::text_offset(world.theme_selector)).clone();
    let foreground_color = world
        .global
        .get_safe(&CloseButtonTheme::foreground_color(world.theme_selector))
        .clone();
    let font_size = world.global.get_safe(&CloseButtonTheme::font_size(world.theme_selector)).clone();

    renderer.render_text("X", text_offset, foreground_color, font_size);
}
