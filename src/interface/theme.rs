use procedural::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Color;
use crate::interface::*;

pub struct Menu;
pub struct Main;

pub trait ThemeType {}

impl ThemeType for Menu {}
impl ThemeType for Main {}

pub trait ThemeDefault<T: ThemeType> {
    fn default() -> Self;
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ButtonTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub disabled_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub hovered_foreground_color: Mutable<Color, Render>,
    pub disabled_foreground_color: Mutable<Color, Render>,
    pub debug_foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub icon_offset: MutableRange<ScreenPosition, Render>,
    pub icon_size: MutableRange<ScreenSize, Render>,
    pub icon_text_offset: MutableRange<ScreenPosition, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub height_constraint: DimensionConstraint,
}

impl ThemeDefault<Menu> for ButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(150, 70, 255)),
            hovered_background_color: Mutable::new(Color::rgb(200, 70, 255)),
            disabled_background_color: Mutable::new(Color::monochrome(70)),
            foreground_color: Mutable::new(Color::monochrome(200)),
            hovered_foreground_color: Mutable::new(Color::rgb(220, 170, 215)),
            disabled_foreground_color: Mutable::new(Color::monochrome(140)),
            debug_foreground_color: Mutable::new(Color::rgb(230, 140, 230)),
            corner_radius: MutableRange::new(
                CornerRadius::uniform(26.0),
                CornerRadius::default(),
                CornerRadius::uniform(30.0),
            ),
            icon_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: 2.5 },
                ScreenPosition::default(),
                ScreenPosition::uniform(20.0),
            ),
            icon_size: MutableRange::new(ScreenSize::uniform(16.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            icon_text_offset: MutableRange::new(
                ScreenPosition { left: 40.0, top: 4.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 100.0, top: 20.0 },
            ),
            text_offset: MutableRange::new(
                ScreenPosition { left: 15.0, top: 6.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 100.0, top: 20.0 },
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            height_constraint: dimension!(26),
        }
    }
}

impl ThemeDefault<Main> for ButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(100)),
            hovered_background_color: Mutable::new(Color::rgb(140, 120, 140)),
            disabled_background_color: Mutable::new(Color::monochrome(70)),
            foreground_color: Mutable::new(Color::monochrome(200)),
            hovered_foreground_color: Mutable::new(Color::rgb(220, 170, 215)),
            disabled_foreground_color: Mutable::new(Color::monochrome(140)),
            debug_foreground_color: Mutable::new(Color::rgb(230, 140, 230)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            icon_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: 2.5 },
                ScreenPosition::default(),
                ScreenPosition::uniform(20.0),
            ),
            icon_size: MutableRange::new(ScreenSize::uniform(10.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            icon_text_offset: MutableRange::new(
                ScreenPosition { left: 20.0, top: 1.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 100.0, top: 20.0 },
            ),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 1.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 100.0, top: 20.0 },
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            height_constraint: dimension!(16),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct WindowTheme {
    pub background_color: Mutable<Color, Render>,
    pub title_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub title_corner_radius: MutableRange<CornerRadius, Render>,
    pub border_size: MutableRange<ScreenSize, Resolve>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub gaps: MutableRange<ScreenSize, Resolve>,
    pub font_size: MutableRange<f32, Render>,
    pub title_height: DimensionConstraint,
}

impl ThemeDefault<Menu> for WindowTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(30)),
            title_background_color: Mutable::new(Color::rgba(70, 60, 70, 0)),
            foreground_color: Mutable::new(Color::rgb(150, 70, 255)),
            corner_radius: MutableRange::new(
                CornerRadius::uniform(30.0),
                CornerRadius::default(),
                CornerRadius::uniform(30.0),
            ),
            title_corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            border_size: MutableRange::new(ScreenSize::uniform(30.0), ScreenSize::default(), ScreenSize::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: -1.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 50.0, top: 30.0 },
            ),
            gaps: MutableRange::new(
                ScreenSize { width: 9.0, height: 19.0 },
                ScreenSize::default(),
                ScreenSize::uniform(20.0),
            ),
            font_size: MutableRange::new(20.0, 6.0, 30.0),
            title_height: dimension!(30),
        }
    }
}

impl ThemeDefault<Main> for WindowTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(40)),
            title_background_color: Mutable::new(Color::rgb(170, 60, 70)),
            foreground_color: Mutable::new(Color::monochrome(160)),
            corner_radius: MutableRange::new(CornerRadius::uniform(4.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            title_corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            border_size: MutableRange::new(
                ScreenSize { width: 12.0, height: 6.0 },
                ScreenSize::default(),
                ScreenSize::uniform(30.0),
            ),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: -1.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 50.0, top: 30.0 },
            ),
            gaps: MutableRange::new(
                ScreenSize { width: 4.0, height: 5.0 },
                ScreenSize::default(),
                ScreenSize::uniform(20.0),
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            title_height: dimension!(12),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ExpandableTheme {
    pub background_color: Mutable<Color, Render>,
    pub second_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub hovered_foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub border_size: MutableRange<ScreenSize, Resolve>,
    pub element_offset: MutableRange<ScreenPosition, Resolve>,
    pub icon_offset: MutableRange<ScreenPosition, Render>,
    pub icon_size: MutableRange<ScreenSize, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub gaps: MutableRange<ScreenSize, Resolve>,
    pub font_size: MutableRange<f32, Render>,
}

impl ThemeDefault<Menu> for ExpandableTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(60)),
            second_background_color: Mutable::new(Color::monochrome(45)),
            foreground_color: Mutable::new(Color::monochrome(170)),
            hovered_foreground_color: Mutable::new(Color::rgb(190, 145, 185)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            border_size: MutableRange::new(ScreenSize::uniform(5.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            element_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: -2.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 30.0, top: 30.0 },
            ),
            icon_offset: MutableRange::new(
                ScreenPosition { left: 6.0, top: 5.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 30.0, top: 50.0 },
            ),
            icon_size: MutableRange::new(ScreenSize::uniform(6.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 14.0, top: 1.5 },
                ScreenPosition::default(),
                ScreenPosition { left: 50.0, top: 20.0 },
            ),
            gaps: MutableRange::new(ScreenSize::uniform(6.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
        }
    }
}

impl ThemeDefault<Main> for ExpandableTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(60)),
            second_background_color: Mutable::new(Color::monochrome(45)),
            foreground_color: Mutable::new(Color::monochrome(170)),
            hovered_foreground_color: Mutable::new(Color::rgb(190, 145, 185)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            border_size: MutableRange::new(ScreenSize::uniform(5.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            element_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: -2.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(30.0),
            ),
            icon_offset: MutableRange::new(
                ScreenPosition { left: 6.0, top: 5.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 30.0, top: 50.0 },
            ),
            icon_size: MutableRange::new(ScreenSize::uniform(6.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 14.0, top: 1.5 },
                ScreenPosition::default(),
                ScreenPosition { left: 50.0, top: 20.0 },
            ),
            gaps: MutableRange::new(ScreenSize::uniform(6.0), ScreenSize::default(), ScreenSize::uniform(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct LabelTheme {
    pub background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub size_constraint: SizeConstraint,
}

impl ThemeDefault<Menu> for LabelTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(130)),
            foreground_color: Mutable::new(Color::monochrome(255)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(120 > 50% < 300, 0),
        }
    }
}

impl ThemeDefault<Main> for LabelTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(130)),
            foreground_color: Mutable::new(Color::monochrome(255)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(120 > 50% < 300, 0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ValueTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub size_constraint: SizeConstraint,
}

impl ThemeDefault<Menu> for ValueTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(100, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(130, 100, 120)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(60 > !, 14),
        }
    }
}

impl ThemeDefault<Main> for ValueTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(100, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(130, 100, 120)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(60 > !, 14),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CloseButtonTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub size_constraint: SizeConstraint,
}

impl ThemeDefault<Menu> for CloseButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(200, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(200, 140, 100)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            corner_radius: MutableRange::new(
                CornerRadius::uniform(26.0),
                CornerRadius::default(),
                CornerRadius::uniform(30.0),
            ),
            text_offset: MutableRange::new(
                ScreenPosition { left: 8.35, top: 2.55 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(20.0, 6.0, 30.0),
            size_constraint: constraint!(26, 26),
        }
    }
}

impl ThemeDefault<Main> for CloseButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(200, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(200, 140, 100)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(1.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 9.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(12.0, 6.0, 30.0),
            size_constraint: constraint!(25, 12),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct OverlayTheme {
    pub foreground_color: Mutable<Color, Nothing>,
    pub text_offset: MutableRange<ScreenPosition, Nothing>,
    pub font_size: MutableRange<f32, Nothing>,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            foreground_color: Mutable::new(Color::monochrome(220)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 20.0, top: 10.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 1000.0, top: 500.0 },
            ),
            font_size: MutableRange::new(18.0, 6.0, 50.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct SliderTheme {
    pub background_color: Mutable<Color, Render>,
    pub rail_color: Mutable<Color, Render>,
    pub knob_color: Mutable<Color, Render>,
    pub size_constraint: SizeConstraint,
}

impl ThemeDefault<Menu> for SliderTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(140, 80, 100)),
            rail_color: Mutable::new(Color::rgb(150, 130, 150)),
            knob_color: Mutable::new(Color::rgb(100, 180, 180)),
            size_constraint: constraint!(100%, 18),
        }
    }
}

impl ThemeDefault<Main> for SliderTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(140, 80, 100)),
            rail_color: Mutable::new(Color::rgb(150, 130, 150)),
            knob_color: Mutable::new(Color::rgb(100, 180, 180)),
            size_constraint: constraint!(100%, 18),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct InputTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub focused_background_color: Mutable<Color, Render>,
    pub text_color: Mutable<Color, Render>,
    pub ghost_text_color: Mutable<Color, Render>,
    pub focused_text_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub cursor_offset: MutableRange<f32, Render>,
    pub cursor_width: MutableRange<f32, Render>,
    pub height_constraint: DimensionConstraint,
}

impl ThemeDefault<Menu> for InputTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(45)),
            hovered_background_color: Mutable::new(Color::rgb(70, 60, 80)),
            focused_background_color: Mutable::new(Color::monochrome(100)),
            text_color: Mutable::new(Color::monochrome(200)),
            ghost_text_color: Mutable::new(Color::monochrome(100)),
            focused_text_color: Mutable::new(Color::monochrome(200)),
            corner_radius: MutableRange::new(
                CornerRadius::uniform(26.0),
                CornerRadius::default(),
                CornerRadius::uniform(30.0),
            ),
            font_size: MutableRange::new(15.0, 6.0, 50.0),
            text_offset: MutableRange::new(
                ScreenPosition { left: 15.0, top: 6.0 },
                ScreenPosition::default(),
                ScreenPosition::uniform(50.0),
            ),
            cursor_offset: MutableRange::new(2.0, 0.0, 10.0),
            cursor_width: MutableRange::new(3.0, 2.0, 30.0),
            height_constraint: dimension!(26),
        }
    }
}

impl ThemeDefault<Main> for InputTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(60)),
            hovered_background_color: Mutable::new(Color::monochrome(80)),
            focused_background_color: Mutable::new(Color::monochrome(100)),
            text_color: Mutable::new(Color::monochrome(200)),
            ghost_text_color: Mutable::new(Color::monochrome(100)),
            focused_text_color: Mutable::new(Color::monochrome(200)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            font_size: MutableRange::new(14.0, 6.0, 50.0),
            text_offset: MutableRange::new(
                ScreenPosition { left: 4.0, top: 0.0 },
                ScreenPosition::default(),
                ScreenPosition::uniform(50.0),
            ),
            cursor_offset: MutableRange::new(2.0, 0.0, 10.0),
            cursor_width: MutableRange::new(3.0, 2.0, 30.0),
            height_constraint: dimension!(15),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ChatTheme {
    pub background_color: Mutable<Color, Render>,
    pub font_size: MutableRange<f32, Render>,
}

impl ThemeDefault<Menu> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgba(0, 0, 0, 170)),
            font_size: MutableRange::new(14.0, 6.0, 50.0),
        }
    }
}

impl ThemeDefault<Main> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgba(0, 0, 0, 170)),
            font_size: MutableRange::new(14.0, 6.0, 50.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CursorTheme {
    pub color: Mutable<Color, Nothing>,
}

impl Default for CursorTheme {
    fn default() -> Self {
        Self {
            color: Mutable::new(Color::monochrome(255)),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ProfilerTheme {
    pub background_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub line_color: Mutable<Color, Render>,
    pub line_width: MutableRange<f32, Render>,
    pub bar_height: MutableRange<f32, Render>,
    pub bar_gap: MutableRange<ScreenSize, Render>,
    pub bar_corner_radius: MutableRange<CornerRadius, Render>,
    pub bar_text_color: Mutable<Color, Render>,
    pub bar_text_size: MutableRange<f32, Render>,
    pub bar_text_offset: MutableRange<ScreenPosition, Render>,
    pub distance_text_size: MutableRange<f32, Render>,
    pub distance_text_offset: MutableRange<f32, Render>,
}

impl ThemeDefault<Menu> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(55)),
            corner_radius: MutableRange::new(CornerRadius::uniform(2.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            line_color: Mutable::new(Color::rgb(80, 90, 80)),
            line_width: MutableRange::new(2.0, 0.5, 4.0),
            bar_height: MutableRange::new(15.0, 5.0, 30.0),
            bar_gap: MutableRange::new(ScreenSize { width: 1.0, height: 5.0 }, ScreenSize::default(), ScreenSize {
                width: 10.0,
                height: 20.0,
            }),
            bar_corner_radius: MutableRange::new(CornerRadius::default(), CornerRadius::default(), CornerRadius::uniform(15.0)),
            bar_text_color: Mutable::new(Color::monochrome(0)),
            bar_text_size: MutableRange::new(14.0, 6.0, 50.0),
            bar_text_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: 0.0 },
                ScreenPosition { left: 0.0, top: -10.0 },
                ScreenPosition { left: 40.0, top: 10.0 },
            ),
            distance_text_size: MutableRange::new(12.0, 6.0, 50.0),
            distance_text_offset: MutableRange::new(20.0, 0.0, 200.0),
        }
    }
}

impl ThemeDefault<Main> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(55)),
            corner_radius: MutableRange::new(CornerRadius::uniform(2.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            line_color: Mutable::new(Color::rgb(80, 90, 80)),
            line_width: MutableRange::new(2.0, 0.5, 4.0),
            bar_height: MutableRange::new(15.0, 5.0, 30.0),
            bar_gap: MutableRange::new(ScreenSize { width: 1.0, height: 5.0 }, ScreenSize::default(), ScreenSize {
                width: 10.0,
                height: 20.0,
            }),
            bar_corner_radius: MutableRange::new(CornerRadius::default(), CornerRadius::default(), CornerRadius::uniform(15.0)),
            bar_text_color: Mutable::new(Color::monochrome(0)),
            bar_text_size: MutableRange::new(14.0, 6.0, 50.0),
            bar_text_offset: MutableRange::new(
                ScreenPosition { left: 7.0, top: 0.0 },
                ScreenPosition { left: 0.0, top: -10.0 },
                ScreenPosition { left: 40.0, top: 10.0 },
            ),
            distance_text_size: MutableRange::new(12.0, 6.0, 50.0),
            distance_text_offset: MutableRange::new(20.0, 0.0, 200.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct StatusBarTheme {
    pub background_color: Mutable<Color, Nothing>,
    pub player_health_color: Mutable<Color, Nothing>,
    pub enemy_health_color: Mutable<Color, Nothing>,
    pub spell_point_color: Mutable<Color, Nothing>,
    pub activity_point_color: Mutable<Color, Nothing>,
    pub player_bar_width: MutableRange<f32, Render>,
    pub enemy_bar_width: MutableRange<f32, Render>,
    pub health_height: MutableRange<f32, Render>,
    pub enemy_health_height: MutableRange<f32, Render>,
    pub spell_point_height: MutableRange<f32, Render>,
    pub activity_point_height: MutableRange<f32, Render>,
    pub border_size: MutableRange<ScreenSize, Render>,
    pub gap: MutableRange<f32, Render>,
}

impl Default for StatusBarTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(40)),
            player_health_color: Mutable::new(Color::rgb(67, 163, 83)),
            enemy_health_color: Mutable::new(Color::rgb(206, 49, 116)),
            spell_point_color: Mutable::new(Color::rgb(0, 129, 163)),
            activity_point_color: Mutable::new(Color::rgb(218, 145, 81)),
            player_bar_width: MutableRange::new(85.0, 20.0, 300.0),
            enemy_bar_width: MutableRange::new(60.0, 20.0, 300.0),
            health_height: MutableRange::new(8.0, 2.0, 30.0),
            enemy_health_height: MutableRange::new(6.0, 2.0, 30.0),
            spell_point_height: MutableRange::new(4.0, 2.0, 30.0),
            activity_point_height: MutableRange::new(4.0, 2.0, 30.0),
            border_size: MutableRange::new(
                ScreenSize { width: 2.0, height: 1.0 },
                ScreenSize::default(),
                ScreenSize::uniform(20.0),
            ),
            gap: MutableRange::new(1.0, 0.0, 10.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct IndicatorTheme {
    pub walking: Mutable<Color, Render>,
}

impl Default for IndicatorTheme {
    fn default() -> Self {
        Self {
            walking: Mutable::new(Color::rgba(0, 255, 170, 170)),
        }
    }
}

#[derive(Default)]
pub struct ThemeSelector;

impl PrototypeElement for ThemeSelector {
    fn to_element(&self, display: String) -> ElementCell {
        let theme_name = Rc::new(RefCell::new("".to_owned()));
        let name_action = Box::new(move || vec![ClickAction::FocusNext(FocusMode::FocusNext)]);
        let theme_kind = TrackedState::new(ThemeKind::Main);

        let load_action = {
            let theme_name = theme_name.clone();
            let theme_kind = theme_kind.clone();

            Box::new(move || {
                let mut taken_name = String::new();
                let mut theme_name = theme_name.borrow_mut();
                std::mem::swap(&mut taken_name, &mut theme_name);

                let file_name = format!("client/themes/{}.ron", taken_name);

                vec![ClickAction::Event(UserEvent::SetThemeFile {
                    theme_file: file_name,
                    theme_kind: theme_kind.get(),
                })]
            })
        };

        let save_action = {
            let theme_kind = theme_kind.clone();

            Box::new(move || {
                vec![ClickAction::Event(UserEvent::SaveTheme {
                    theme_kind: theme_kind.get(),
                })]
            })
        };

        let reload_action = {
            let theme_kind = theme_kind.clone();

            Box::new(move || {
                vec![ClickAction::Event(UserEvent::ReloadTheme {
                    theme_kind: theme_kind.get(),
                })]
            })
        };

        let elements = vec![
            PickList::default()
                .with_options(vec![
                    ("Menu", ThemeKind::Menu),
                    ("Main", ThemeKind::Main),
                    ("Game", ThemeKind::Game),
                ])
                .with_selected(theme_kind)
                .with_event(Box::new(Vec::new))
                .with_width(dimension!(!))
                .wrap(),
            InputField::<40>::new(theme_name, "Theme name", name_action, dimension!(75%)).wrap(),
            Button::default()
                .with_text("Load")
                .with_event(load_action)
                .with_width(dimension!(!))
                .wrap(),
            Button::default()
                .with_text("Save theme")
                .with_event(save_action)
                .with_width(dimension!(50%))
                .wrap(),
            Button::default()
                .with_text("Reload theme")
                .with_event(reload_action)
                .with_width(dimension!(!))
                .wrap(),
        ];

        Expandable::new(display, elements, false).wrap()
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct InterfaceTheme {
    pub button: ButtonTheme,
    pub window: WindowTheme,
    pub expandable: ExpandableTheme,
    pub label: LabelTheme,
    pub value: ValueTheme,
    pub close_button: CloseButtonTheme,
    pub slider: SliderTheme,
    pub input: InputTheme,
    pub profiler: ProfilerTheme,
    pub chat: ChatTheme,
}

impl<T: ThemeType> ThemeDefault<T> for InterfaceTheme
where
    ButtonTheme: ThemeDefault<T>,
    WindowTheme: ThemeDefault<T>,
    ExpandableTheme: ThemeDefault<T>,
    LabelTheme: ThemeDefault<T>,
    ValueTheme: ThemeDefault<T>,
    CloseButtonTheme: ThemeDefault<T>,
    SliderTheme: ThemeDefault<T>,
    InputTheme: ThemeDefault<T>,
    ProfilerTheme: ThemeDefault<T>,
    ChatTheme: ThemeDefault<T>,
{
    fn default() -> Self {
        Self {
            button: ThemeDefault::<T>::default(),
            window: ThemeDefault::<T>::default(),
            expandable: ThemeDefault::<T>::default(),
            label: ThemeDefault::<T>::default(),
            value: ThemeDefault::<T>::default(),
            close_button: ThemeDefault::<T>::default(),
            slider: ThemeDefault::<T>::default(),
            input: ThemeDefault::<T>::default(),
            profiler: ThemeDefault::<T>::default(),
            chat: ThemeDefault::<T>::default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, PrototypeElement)]
pub struct GameTheme {
    pub overlay: OverlayTheme,
    pub status_bar: StatusBarTheme,
    pub indicator: IndicatorTheme,
    pub cursor: CursorTheme,
}

#[derive(PrototypeWindow)]
#[window_title("Theme Viewer")]
#[window_class("theme_viewer")]
pub struct Themes {
    #[name("Theme selector")]
    pub theme_selector: ThemeSelector,
    #[name("Menu")]
    pub menu: InterfaceTheme,
    #[name("Main")]
    pub main: InterfaceTheme,
    #[name("Game")]
    pub game: GameTheme,
}

impl InterfaceTheme {
    pub fn new<T: ThemeType>(theme_file: &str) -> Self
    where
        Self: ThemeDefault<T>,
    {
        Self::load(theme_file).unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}{}{}", MAGENTA, theme_file, NONE);

            ThemeDefault::<T>::default()
        })
    }

    fn load(theme_file: &str) -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading theme from {}{}{}", MAGENTA, theme_file, NONE);

        std::fs::read_to_string(theme_file).ok().and_then(|data| ron::from_str(&data).ok())
    }

    pub fn reload(&mut self, theme_file: &str) -> bool {
        let Some(theme) = Self::load(theme_file) else {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}{}{}", MAGENTA, theme_file, NONE);

            return false;
        };

        *self = theme;
        true
    }

    pub fn save(&self, theme_file: &str) {
        #[cfg(feature = "debug")]
        print_debug!("saving theme to {}{}{}", MAGENTA, theme_file, NONE);

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(theme_file, data).expect("unable to write file");
    }
}

impl GameTheme {
    pub fn new(theme_file: &str) -> Self {
        Self::load(theme_file).unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}{}{}", MAGENTA, theme_file, NONE);

            Default::default()
        })
    }

    fn load(theme_file: &str) -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading theme from {}{}{}", MAGENTA, theme_file, NONE);

        std::fs::read_to_string(theme_file).ok().and_then(|data| ron::from_str(&data).ok())
    }

    pub fn reload(&mut self, theme_file: &str) -> bool {
        let Some(theme) = Self::load(theme_file) else {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}{}{}", MAGENTA, theme_file, NONE);

            return false;
        };

        *self = theme;
        true
    }

    pub fn save(&self, theme_file: &str) {
        #[cfg(feature = "debug")]
        print_debug!("saving theme to {}{}{}", MAGENTA, theme_file, NONE);

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(theme_file, data).expect("unable to write file");
    }
}
