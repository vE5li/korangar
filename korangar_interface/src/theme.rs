use rust_state::RustState;

use crate::application::Application;
use crate::layout::{DimensionBound, SizeBound};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ButtonTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    hovered_background_color: App::Color,
    disabled_background_color: App::Color,
    foreground_color: App::Color,
    hovered_foreground_color: App::Color,
    disabled_foreground_color: App::Color,
    debug_foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    icon_offset: App::Position,
    icon_size: App::Size,
    icon_text_offset: App::Position,
    text_offset: App::Position,
    font_size: App::FontSize,
    height_bound: DimensionBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    title_background_color: App::Color,
    foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    title_corner_radius: App::CornerRadius,
    border_size: App::Size,
    text_offset: App::Position,
    gaps: App::Size,
    font_size: App::FontSize,
    title_height: DimensionBound,
    anchor_color: App::Color,
    closest_anchor_color: App::Color,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpandableTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    second_background_color: App::Color,
    foreground_color: App::Color,
    hovered_foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    border_size: App::Size,
    element_offset: App::Position,
    icon_offset: App::Position,
    icon_size: App::Size,
    text_offset: App::Position,
    gaps: App::Size,
    font_size: App::FontSize,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    text_offset: App::Position,
    font_size: App::FontSize,
    size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    hovered_background_color: App::Color,
    foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    text_offset: App::Position,
    font_size: App::FontSize,
    size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseButtonTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    hovered_background_color: App::Color,
    foreground_color: App::Color,
    corner_radius: App::CornerRadius,
    text_offset: App::Position,
    font_size: App::FontSize,
    size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SliderTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    rail_color: App::Color,
    knob_color: App::Color,
    size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InputTheme<App>
where
    App: Application,
{
    background_color: App::Color,
    hovered_background_color: App::Color,
    focused_background_color: App::Color,
    text_color: App::Color,
    ghost_text_color: App::Color,
    focused_text_color: App::Color,
    corner_radius: App::CornerRadius,
    font_size: App::FontSize,
    text_offset: App::Position,
    cursor_offset: f32,
    cursor_width: f32,
    height_bound: DimensionBound,
}
