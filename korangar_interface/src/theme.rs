use rust_state::RustState;

use crate::application::Application;
use crate::layout::{DimensionBound, SizeBound};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ButtonTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub hovered_background_color: App::Color,
    pub disabled_background_color: App::Color,
    pub foreground_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub disabled_foreground_color: App::Color,
    pub debug_foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub icon_offset: App::Position,
    pub icon_size: App::Size,
    pub icon_text_offset: App::Position,
    pub text_offset: App::Position,
    pub font_size: App::FontSize,
    pub height_bound: DimensionBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub title_background_color: App::Color,
    pub foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub title_corner_radius: App::CornerRadius,
    pub border_size: App::Size,
    pub text_offset: App::Position,
    pub gaps: App::Size,
    pub font_size: App::FontSize,
    pub title_height: DimensionBound,
    pub anchor_color: App::Color,
    pub closest_anchor_color: App::Color,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpandableTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub second_background_color: App::Color,
    pub foreground_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub border_size: App::Size,
    pub element_offset: App::Position,
    pub icon_offset: App::Position,
    pub icon_size: App::Size,
    pub text_offset: App::Position,
    pub gaps: App::Size,
    pub font_size: App::FontSize,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub text_offset: App::Position,
    pub font_size: App::FontSize,
    pub size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub hovered_background_color: App::Color,
    pub foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub text_offset: App::Position,
    pub font_size: App::FontSize,
    pub size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseButtonTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub hovered_background_color: App::Color,
    pub foreground_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub text_offset: App::Position,
    pub font_size: App::FontSize,
    pub size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SliderTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub rail_color: App::Color,
    pub knob_color: App::Color,
    pub size_bound: SizeBound,
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InputTheme<App>
where
    App: Application,
{
    pub background_color: App::Color,
    pub hovered_background_color: App::Color,
    pub focused_background_color: App::Color,
    pub text_color: App::Color,
    pub ghost_text_color: App::Color,
    pub focused_text_color: App::Color,
    pub corner_radius: App::CornerRadius,
    pub font_size: App::FontSize,
    pub text_offset: App::Position,
    pub cursor_offset: f32,
    pub cursor_width: f32,
    pub height_bound: DimensionBound,
}
