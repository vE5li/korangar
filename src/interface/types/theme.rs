use cgmath::Zero;
use ron::ser::PrettyConfig;
use serde::{ Serialize, Deserialize };

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::maths::*;
use crate::graphics::Color;
use crate::interface::types::*;

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ButtonTheme  {
    pub background_color: Mutable<Color, RERENDER>,
    pub hovered_background_color: Mutable<Color, RERENDER>,
    pub foreground_color: Mutable<Color, RERENDER>,
    pub debug_foreground_color: Mutable<Color, RERENDER>,
    pub border_radius: MutableRange<Vector4<f32>, RERENDER>,
    pub icon_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub icon_size: MutableRange<Vector2<f32>, RERENDER>,
    pub icon_text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub text_offset: MutableRange<Vector2<f32>, RERENDER>,
    pub font_size: MutableRange<f32, RERENDER>,
    pub size_constraint: SizeConstraint,
    pub menu_size_constraint: SizeConstraint,
}

impl Default for ButtonTheme {

    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(100)),
            hovered_background_color: Mutable::new(Color::rgb(140, 120, 140)),
            foreground_color: Mutable::new(Color::monochrome(200)),
            debug_foreground_color: Mutable::new(Color::rgb(230, 140, 230)),
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            icon_offset: MutableRange::new(Vector2::new(7.0, 2.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_size: MutableRange::new(Vector2::new(10.0, 10.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_text_offset: MutableRange::new(Vector2::new(20.0, 0.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, 0.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(40.0 > 100.0%, 16.0),
            menu_size_constraint: constraint!(40.0 > 100.0%, 20.0),
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
    pub title_size_constraint: SizeConstraint,
}

impl Default for WindowTheme {

    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(40)),
            title_background_color: Mutable::new(Color::rgb(70, 60, 70)),
            foreground_color: Mutable::new(Color::monochrome(160)),
            border_radius: MutableRange::new(vector4!(4.0), vector4!(0.0), vector4!(30.0)),
            title_border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            border_size: MutableRange::new(Vector2::new(12.0, 6.0), Vector2::zero(), Vector2::new(30.0, 30.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, -1.0), Vector2::zero(), Vector2::new(50.0, 30.0)),
            gaps: MutableRange::new(Vector2::new(6.0, 8.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            title_size_constraint: constraint!(80.0%, 12.0),
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
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
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
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(5.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(150.0 > 0.0 < 70.0%, 14.0),
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
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(5.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(60.0 > !, 14.0),
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
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(6.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(12.0, 6.0, 30.0),
            size_constraint: constraint!(25.0, 12.0),
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
            size_constraint: constraint!(100.0%, 18.0),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PrototypeWindow)]
#[window_title("theme viewer")]
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

        std::fs::read_to_string(theme_file)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
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
