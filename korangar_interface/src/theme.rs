use crate::application::Application;
use crate::layout::{DimensionBound, SizeBound};

pub trait ButtonTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn hovered_background_color(&self) -> App::Color;
    fn disabled_background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn hovered_foreground_color(&self) -> App::Color;
    fn disabled_foreground_color(&self) -> App::Color;
    fn debug_foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn icon_offset(&self) -> App::Position;
    fn icon_size(&self) -> App::Size;
    fn icon_text_offset(&self) -> App::Position;
    fn text_offset(&self) -> App::Position;
    fn font_size(&self) -> App::FontSize;
    fn height_bound(&self) -> DimensionBound;
}

pub trait WindowTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn title_background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn title_corner_radius(&self) -> App::CornerRadius;
    fn border_size(&self) -> App::Size;
    fn text_offset(&self) -> App::Position;
    fn gaps(&self) -> App::Size;
    fn font_size(&self) -> App::FontSize;
    fn title_height(&self) -> DimensionBound;
}

pub trait ExpandableTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn second_background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn hovered_foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn border_size(&self) -> App::Size;
    fn element_offset(&self) -> App::Position;
    fn icon_offset(&self) -> App::Position;
    fn icon_size(&self) -> App::Size;
    fn text_offset(&self) -> App::Position;
    fn gaps(&self) -> App::Size;
    fn font_size(&self) -> App::FontSize;
}

pub trait LabelTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn text_offset(&self) -> App::Position;
    fn font_size(&self) -> App::FontSize;
    fn size_bound(&self) -> SizeBound;
}

pub trait ValueTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn hovered_background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn text_offset(&self) -> App::Position;
    fn font_size(&self) -> App::FontSize;
    fn size_bound(&self) -> SizeBound;
}

pub trait CloseButtonTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn hovered_background_color(&self) -> App::Color;
    fn foreground_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn text_offset(&self) -> App::Position;
    fn font_size(&self) -> App::FontSize;
    fn size_bound(&self) -> SizeBound;
}

pub trait OverlayTheme<App>
where
    App: Application,
{
    fn foreground_color(&self) -> App::Color;
    fn text_offset(&self) -> App::Position;
    fn font_size(&self) -> App::FontSize;
}

pub trait SliderTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn rail_color(&self) -> App::Color;
    fn knob_color(&self) -> App::Color;
    fn size_bound(&self) -> SizeBound;
}

pub trait InputTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn hovered_background_color(&self) -> App::Color;
    fn focused_background_color(&self) -> App::Color;
    fn text_color(&self) -> App::Color;
    fn ghost_text_color(&self) -> App::Color;
    fn focused_text_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn font_size(&self) -> App::FontSize;
    fn text_offset(&self) -> App::Position;
    fn cursor_offset(&self) -> f32;
    fn cursor_width(&self) -> f32;
    fn height_bound(&self) -> DimensionBound;
}

pub trait ChatTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn font_size(&self) -> App::FontSize;
    fn broadcast_color(&self) -> App::Color;
    fn server_color(&self) -> App::Color;
    fn error_color(&self) -> App::Color;
    fn information_color(&self) -> App::Color;
}

pub trait CursorTheme<App>
where
    App: Application,
{
    fn color(&self) -> App::Color;
}

pub trait ProfilerTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn corner_radius(&self) -> App::CornerRadius;
    fn line_color(&self) -> App::Color;
    fn line_width(&self) -> f32;
    fn bar_height(&self) -> f32;
    fn bar_gap(&self) -> App::Size;
    fn bar_corner_radius(&self) -> App::CornerRadius;
    fn bar_text_color(&self) -> App::Color;
    fn bar_text_size(&self) -> f32;
    fn bar_text_offset(&self) -> App::Position;
    fn distance_text_size(&self) -> f32;
    fn distance_text_offset(&self) -> f32;
}

pub trait StatusBarTheme<App>
where
    App: Application,
{
    fn background_color(&self) -> App::Color;
    fn player_health_color(&self) -> App::Color;
    fn enemy_health_color(&self) -> App::Color;
    fn spell_point_color(&self) -> App::Color;
    fn activity_point_color(&self) -> App::Color;
    fn player_bar_width(&self) -> f32;
    fn enemy_bar_width(&self) -> f32;
    fn health_height(&self) -> f32;
    fn enemy_health_height(&self) -> f32;
    fn spell_point_height(&self) -> f32;
    fn activity_point_height(&self) -> f32;
    fn border_size(&self) -> App::Size;
    fn gap(&self) -> f32;
}

pub trait IndicatorTheme<App>
where
    App: Application,
{
    fn walking(&self) -> App::Color;
}

pub trait InterfaceTheme {
    type Settings: Application;
    type Button: ButtonTheme<Self::Settings>;
    type Window: WindowTheme<Self::Settings>;
    type Expandable: ExpandableTheme<Self::Settings>;
    type Label: LabelTheme<Self::Settings>;
    type Value: ValueTheme<Self::Settings>;
    type CloseButton: CloseButtonTheme<Self::Settings>;
    type Slider: SliderTheme<Self::Settings>;
    type Input: InputTheme<Self::Settings>;
    type Profiler: ProfilerTheme<Self::Settings>;
    type Chat: ChatTheme<Self::Settings>;

    fn button(&self) -> &Self::Button;
    fn window(&self) -> &Self::Window;
    fn expandable(&self) -> &Self::Expandable;
    fn label(&self) -> &Self::Label;
    fn value(&self) -> &Self::Value;
    fn close_button(&self) -> &Self::CloseButton;
    fn slider(&self) -> &Self::Slider;
    fn input(&self) -> &Self::Input;
    fn profiler(&self) -> &Self::Profiler;
    fn chat(&self) -> &Self::Chat;
}
