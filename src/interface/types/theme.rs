use cgmath::Zero;
use serde::{ Serialize, Deserialize };

#[cfg(feature = "debug")]
use debug::*;
use types::maths::*;
use graphics::Color;
use interface::types::{ MutableRange, SizeConstraint };

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct ButtonTheme  {
    pub background_color: Color,
    pub hovered_background_color: Color,
    pub foreground_color: Color,
    pub debug_foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub icon_offset: MutableRange<Vector2<f32>>,
    pub icon_size: MutableRange<Vector2<f32>>,
    pub icon_text_offset: MutableRange<Vector2<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
    pub size_constraint: SizeConstraint,
    pub menu_size_constraint: SizeConstraint,
}

impl Default for ButtonTheme {

    fn default() -> Self {
        Self {
            background_color: Color::monochrome(100),
            hovered_background_color: Color::rgb(140, 120, 140),
            foreground_color: Color::monochrome(200),
            debug_foreground_color: Color::rgb(230, 140, 230),
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

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct WindowTheme {
    pub background_color: Color,
    pub title_background_color: Color,
    pub foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub title_border_radius: MutableRange<Vector4<f32>>,
    pub border_size: MutableRange<Vector2<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub gaps: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
    pub title_size_constraint: SizeConstraint,
}

impl Default for WindowTheme {

    fn default() -> Self {
        Self {
            background_color: Color::monochrome(40),
            title_background_color: Color::rgb(70, 60, 70),
            foreground_color: Color::monochrome(160),
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

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct ExpandableTheme {
    pub background_color: Color,
    pub second_background_color: Color,
    pub foreground_color: Color,
    pub hovered_foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub border_size: MutableRange<Vector2<f32>>,
    pub element_offset: MutableRange<Vector2<f32>>,
    pub icon_offset: MutableRange<Vector2<f32>>,
    pub icon_size: MutableRange<Vector2<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub gaps: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
}

impl Default for ExpandableTheme {

    fn default() -> Self {
        Self {
            background_color: Color::monochrome(60),
            second_background_color: Color::monochrome(45),
            foreground_color: Color::monochrome(170),
            hovered_foreground_color: Color::rgb(190, 145, 185),
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

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct LabelTheme {
    pub background_color: Color,
    pub foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
    pub size_constraint: SizeConstraint,
}

impl Default for LabelTheme {

    fn default() -> Self {
        Self {
            background_color: Color::monochrome(130),
            foreground_color: Color::monochrome(255),
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(5.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(150.0 > 0.0 < 70.0%, 14.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct ValueTheme {
    pub background_color: Color,
    pub hovered_background_color: Color,
    pub foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
    pub size_constraint: SizeConstraint,
}

impl Default for ValueTheme {

    fn default() -> Self {
        Self {
            background_color: Color::rgb(100, 100, 100),
            hovered_background_color: Color::rgb(130, 100, 120),
            foreground_color: Color::rgb(220, 220, 220),
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(5.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
            size_constraint: constraint!(60.0 > !, 14.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct CloseButtonTheme {
    pub background_color: Color,
    pub hovered_background_color: Color,
    pub foreground_color: Color,
    pub border_radius: MutableRange<Vector4<f32>>,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
    pub size_constraint: SizeConstraint,
}

impl Default for CloseButtonTheme {

    fn default() -> Self {
        Self {
            background_color: Color::rgb(200, 100, 100),
            hovered_background_color: Color::rgb(200, 140, 100),
            foreground_color: Color::rgb(220, 220, 220),
            border_radius: MutableRange::new(vector4!(6.0), vector4!(0.0), vector4!(30.0)),
            text_offset: MutableRange::new(vector2!(6.0, 0.0), vector2!(-10.0), vector2!(20.0)),
            font_size: MutableRange::new(12.0, 6.0, 30.0),
            size_constraint: constraint!(25.0, 12.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct OverlayTheme {
    pub foreground_color: Color,
    pub text_offset: MutableRange<Vector2<f32>>,
    pub font_size: MutableRange<f32>,
}

impl Default for OverlayTheme {

    fn default() -> Self {
        Self {
            foreground_color: Color::monochrome(220),
            text_offset: MutableRange::new(Vector2::new(20.0, 10.0), Vector2::zero(), Vector2::new(1000.0, 500.0)),
            font_size: MutableRange::new(18.0, 6.0, 50.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeMutableElement)]
pub struct SliderTheme {
    pub background_color: Color,
    pub rail_color: Color,
    pub knob_color: Color,
    pub size_constraint: SizeConstraint,
}

impl Default for SliderTheme {

    fn default() -> Self {
        Self {
            background_color: Color::rgb(140, 80, 100),
            rail_color: Color::rgb(150, 130, 150),
            knob_color: Color::rgb(100, 180, 180),
            size_constraint: constraint!(100.0%, 18.0),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PrototypeMutableWindow)]
pub struct Theme {
    #[window_title("theme viewer")]
    #[window_class("theme_viewer")]
    #[event_button("reload theme", ReloadTheme)]
    _dummy: usize,
    #[event_button("save theme", SaveTheme)]
    _dummy1: usize,
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
            print_debug!("failed to load theme from file {}{}{}", magenta(), theme_file, none());

            Default::default()
        })
    }

    fn load(theme_file: &str) -> Option<Self> {

        #[cfg(feature = "debug")]
        print_debug!("loading theme from {}{}{}", magenta(), theme_file, none());

        std::fs::read_to_string(theme_file)
            .ok()
            .map(|data| serde_json::from_str(&data).ok())
            .flatten()
    }

    pub fn reload(&mut self, theme_file: &str) -> bool {

        let Some(theme) = Self::load(theme_file) else {

            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}{}{}", magenta(), theme_file, none());

            return false;
        };

        *self = theme;
        true
    }
        
    pub fn save(&self, theme_file: &str) {

        #[cfg(feature = "debug")]
        print_debug!("saving theme to {}{}{}", magenta(), theme_file, none());

        let data = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(theme_file, data).expect("unable to write file");
    }
}
