use cgmath::{Array, Vector2, Vector4, Zero};
use procedural::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Color;
use crate::interface::*;

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ButtonTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub hovered_background_color: Mutable<Color, RERENDER>,
    pub disabled_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub hovered_foreground_color: Mutable<Color, RERENDER>,
    pub disabled_foreground_color: Mutable<Color, RERENDER>,
    pub debug_foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub icon_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub icon_size: MutableRange<Vector2<f32>, RERENDER>,
    pub icon_text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub height_constraint: DimensionConstraint,
}

impl Default for ButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(100)),
            hovered_background_color: Mutable::new(Color::rgb(140, 120, 140)),
            disabled_background_color: Mutable::new(Color::monochrome(70)),
            foreground_color: Mutable::new(Color::monochrome(200)),
            hovered_foreground_color: Mutable::new(Color::rgb(220, 170, 215)),
            disabled_foreground_color: Mutable::new(Color::monochrome(140)),
            debug_foreground_color: Mutable::new(Color::rgb(230, 140, 230)),
            border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            icon_offset: MutableRange::new(Vector2::new(7.0, 2.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_size: MutableRange::new(Vector2::new(10.0, 10.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_text_offset: MutableRange::new(Vector2::new(20.0, 0.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, 0.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            height_constraint: dimension!(16),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct WindowTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub title_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub title_border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub border_size: MutableRange<Vector2<f32>, RERESOLVE>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub gaps: MutableRange<Vector2<f32>, RERESOLVE>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub title_height: DimensionConstraint,
}

impl Default for WindowTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(40)),
            title_background_color: Mutable::new(Color::rgb(70, 60, 70)),
            foreground_color: Mutable::new(Color::monochrome(160)),
            border_radius: MutableRange::new(Vector4::from_value(4.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            title_border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            border_size: MutableRange::new(Vector2::new(12.0, 6.0), Vector2::zero(), Vector2::new(30.0, 30.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, -1.0), Vector2::zero(), Vector2::new(50.0, 30.0)),
            gaps: MutableRange::new(Vector2::new(4.0, 5.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            title_height: dimension!(12),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ExpandableTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub second_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub hovered_foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub border_size: MutableRange<Vector2<f32>, RERESOLVE>,
    pub element_offset: MutableRange<Vector2<f32>, RERESOLVE>,
    pub icon_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub icon_size: MutableRange<Vector2<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub gaps: MutableRange<Vector2<f32>, RERESOLVE>,
    pub font_size: MutableRange<f32, RERENDER>,
}

impl Default for ExpandableTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(60)),
            second_background_color: Mutable::new(Color::monochrome(45)),
            foreground_color: Mutable::new(Color::monochrome(170)),
            hovered_foreground_color: Mutable::new(Color::rgb(190, 145, 185)),
            border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            border_size: MutableRange::new(Vector2::new(5.0, 5.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            element_offset: MutableRange::new(Vector2::new(7.0, -2.0), Vector2::new(-10.0, -10.0), Vector2::new(30.0, 30.0)),
            icon_offset: MutableRange::new(Vector2::new(6.0, 5.0), Vector2::zero(), Vector2::new(30.0, 50.0)),
            icon_size: MutableRange::new(Vector2::new(6.0, 6.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            text_offset: MutableRange::new(Vector2::new(14.0, 1.0), Vector2::zero(), Vector2::new(50.0, 20.0)),
            gaps: MutableRange::new(Vector2::new(6.0, 6.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct LabelTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub size_constraint: SizeConstraint,
}

impl Default for LabelTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(130)),
            foreground_color: Mutable::new(Color::monochrome(255)),
            border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, 0.0), Vector2::from_value(-10.0), Vector2::from_value(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(120 > 50% < 300, 0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ValueTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub hovered_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub size_constraint: SizeConstraint,
}

impl Default for ValueTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(100, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(130, 100, 120)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, 0.0), Vector2::from_value(-10.0), Vector2::from_value(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(60 > !, 14),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CloseButtonTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub hovered_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub size_constraint: SizeConstraint,
}

impl Default for CloseButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb(200, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb(200, 140, 100)),
            foreground_color: Mutable::new(Color::rgb(220, 220, 220)),
            border_radius: MutableRange::new(Vector4::from_value(1.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            text_offset: MutableRange::new(Vector2::new(9.0, 0.0), Vector2::from_value(-10.0), Vector2::from_value(20.0)),
            font_size: MutableRange::new(12.0, 6.0, 30.0),
            size_constraint: constraint!(25, 12),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct OverlayTheme {
    pub foreground_color: Mutable<Color, NO_EVENT>,
    pub text_offset: MutableRange<Vector2<f32>, NO_EVENT>,
    pub font_size: MutableRange<f32, NO_EVENT>,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            foreground_color: Mutable::new(Color::monochrome(220)),
            text_offset: MutableRange::new(Vector2::new(20.0, 10.0), Vector2::zero(), Vector2::new(1000.0, 500.0)),
            font_size: MutableRange::new(18.0, 6.0, 50.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct SliderTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub rail_color: Mutable<Color, RERENDER>,
    pub knob_color: Mutable<Color, RERENDER>,
    pub size_constraint: SizeConstraint,
}

impl Default for SliderTheme {
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
    pub background_color: Mutable<Color, RERENDER>,
    pub hovered_background_color: Mutable<Color, RERENDER>,
    pub focused_background_color: Mutable<Color, RERENDER>,
    pub text_color: Mutable<Color, RERENDER>,
    pub ghost_text_color: Mutable<Color, RERENDER>,
    pub focused_text_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub text_offset: MutableRange<f32, RERENDER>,
    pub cursor_offset: MutableRange<f32, RERENDER>,
    pub cursor_width: MutableRange<f32, RERENDER>,
    pub height_constraint: DimensionConstraint,
}

impl Default for InputTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(60)),
            hovered_background_color: Mutable::new(Color::monochrome(80)),
            focused_background_color: Mutable::new(Color::monochrome(100)),
            text_color: Mutable::new(Color::monochrome(200)),
            ghost_text_color: Mutable::new(Color::monochrome(100)),
            focused_text_color: Mutable::new(Color::monochrome(200)),
            border_radius: MutableRange::new(Vector4::from_value(6.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            font_size: MutableRange::new(14.0, 6.0, 50.0),
            text_offset: MutableRange::new(4.0, 2.0, 10.0),
            cursor_offset: MutableRange::new(2.0, 0.0, 10.0),
            cursor_width: MutableRange::new(3.0, 2.0, 30.0),
            height_constraint: dimension!(15),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ChatTheme {
    pub background_color: Mutable<Color, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
}

impl Default for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgba(0, 0, 0, 170)),
            font_size: MutableRange::new(14.0, 6.0, 50.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CursorTheme {
    pub color: Mutable<Color, NO_EVENT>,
}

impl Default for CursorTheme {
    fn default() -> Self {
        Self {
            color: Mutable::new(Color::monochrome(255)),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PrototypeWindow)]
#[window_title("Theme Viewer")]
#[window_class("theme_viewer")]
pub struct Theme {
    #[event_button("reload theme", ReloadTheme)]
    #[event_button("save theme", SaveTheme)]
    pub button: ButtonTheme,
    pub window: WindowTheme,
    pub expandable: ExpandableTheme,
    pub label: LabelTheme,
    pub value: ValueTheme,
    pub close_button: CloseButtonTheme,
    pub overlay: OverlayTheme,
    pub slider: SliderTheme,
    pub input: InputTheme,
    pub chat: ChatTheme,
    pub cursor: CursorTheme,
}

impl Theme {
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
