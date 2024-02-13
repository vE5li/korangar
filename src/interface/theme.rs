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
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub disabled_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub hovered_foreground_color: Mutable<Color, Render>,
    pub disabled_foreground_color: Mutable<Color, Render>,
    pub debug_foreground_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub icon_offset: MutableRange<Vector2<f32>, Render>,
    pub icon_size: MutableRange<Vector2<f32>, Render>,
    pub icon_text_offset: MutableRange<Vector2<f32>, Render>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub font_size: MutableRange<f32, Render>,
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
            icon_offset: MutableRange::new(Vector2::new(7.0, 2.5), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_size: MutableRange::new(Vector2::new(10.0, 10.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            icon_text_offset: MutableRange::new(Vector2::new(20.0, 1.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
            text_offset: MutableRange::new(Vector2::new(5.0, 1.0), Vector2::zero(), Vector2::new(100.0, 20.0)),
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
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub title_border_radius: MutableRange<Vector4<f32>, Render>,
    pub border_size: MutableRange<Vector2<f32>, Resolve>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub gaps: MutableRange<Vector2<f32>, Resolve>,
    pub font_size: MutableRange<f32, Render>,
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
    pub background_color: Mutable<Color, Render>,
    pub second_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub hovered_foreground_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub border_size: MutableRange<Vector2<f32>, Resolve>,
    pub element_offset: MutableRange<Vector2<f32>, Resolve>,
    pub icon_offset: MutableRange<Vector2<f32>, Render>,
    pub icon_size: MutableRange<Vector2<f32>, Render>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub gaps: MutableRange<Vector2<f32>, Resolve>,
    pub font_size: MutableRange<f32, Render>,
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
            text_offset: MutableRange::new(Vector2::new(14.0, 1.5), Vector2::zero(), Vector2::new(50.0, 20.0)),
            gaps: MutableRange::new(Vector2::new(6.0, 6.0), Vector2::zero(), Vector2::new(20.0, 20.0)),
            font_size: MutableRange::new(14.0, 6.0, 30.0),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct LabelTheme {
    pub background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub font_size: MutableRange<f32, Render>,
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
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub font_size: MutableRange<f32, Render>,
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
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub text_offset: MutableRange<Vector2<f32>, Render>,
    pub font_size: MutableRange<f32, Render>,
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
    pub foreground_color: Mutable<Color, Nothing>,
    pub text_offset: MutableRange<Vector2<f32>, Nothing>,
    pub font_size: MutableRange<f32, Nothing>,
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
    pub background_color: Mutable<Color, Render>,
    pub rail_color: Mutable<Color, Render>,
    pub knob_color: Mutable<Color, Render>,
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
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub focused_background_color: Mutable<Color, Render>,
    pub text_color: Mutable<Color, Render>,
    pub ghost_text_color: Mutable<Color, Render>,
    pub focused_text_color: Mutable<Color, Render>,
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub font_size: MutableRange<f32, Render>,
    pub text_offset: MutableRange<f32, Render>,
    pub cursor_offset: MutableRange<f32, Render>,
    pub cursor_width: MutableRange<f32, Render>,
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
    pub background_color: Mutable<Color, Render>,
    pub font_size: MutableRange<f32, Render>,
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
    pub border_radius: MutableRange<Vector4<f32>, Render>,
    pub line_color: Mutable<Color, Render>,
    pub line_width: MutableRange<f32, Render>,
    pub bar_height: MutableRange<f32, Render>,
    pub bar_gap: MutableRange<Vector2<f32>, Render>,
    pub bar_border_radius: MutableRange<Vector4<f32>, Render>,
    pub bar_text_color: Mutable<Color, Render>,
    pub bar_text_size: MutableRange<f32, Render>,
    pub bar_text_offset: MutableRange<Vector2<f32>, Render>,
    pub distance_text_size: MutableRange<f32, Render>,
    pub distance_text_offset: MutableRange<f32, Render>,
}

impl Default for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome(55)),
            border_radius: MutableRange::new(Vector4::from_value(2.0), Vector4::from_value(0.0), Vector4::from_value(30.0)),
            line_color: Mutable::new(Color::rgb(80, 90, 80)),
            line_width: MutableRange::new(2.0, 0.5, 4.0),
            bar_height: MutableRange::new(15.0, 5.0, 30.0),
            bar_gap: MutableRange::new(Vector2::new(1.0, 5.0), Vector2::from_value(0.0), Vector2::new(10.0, 20.0)),
            bar_border_radius: MutableRange::new(Vector4::from_value(0.0), Vector4::from_value(0.0), Vector4::from_value(15.0)),
            bar_text_color: Mutable::new(Color::monochrome(0)),
            bar_text_size: MutableRange::new(14.0, 6.0, 50.0),
            bar_text_offset: MutableRange::new(Vector2::new(7.0, 0.0), Vector2::new(0.0, -10.0), Vector2::new(40.0, 10.0)),
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
    pub border_size: MutableRange<Vector2<f32>, Render>,
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
            border_size: MutableRange::new(Vector2::new(2.0, 1.0), Vector2::from_value(0.0), Vector2::from_value(20.0)),
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
        let name_action = Box::new(move || Some(ClickAction::FocusNext(FocusMode::FocusNext)));

        let load_action = {
            let theme_name = theme_name.clone();

            Box::new(move || {
                let mut taken_name = String::new();
                let mut theme_name = theme_name.borrow_mut();
                std::mem::swap(&mut taken_name, &mut theme_name);

                let file_name = format!("client/themes/{}.ron", taken_name);

                Some(ClickAction::Event(UserEvent::SetThemeFile(file_name)))
            })
        };

        let elements = vec![
            InputField::<40>::new(theme_name, "Theme name", name_action, dimension!(75%)).wrap(),
            Button::default()
                .with_text("Load")
                .with_event(load_action)
                .with_width(dimension!(!))
                .wrap(),
            Button::default()
                .with_text("Save theme")
                .with_event(UserEvent::SaveTheme)
                .with_width(dimension!(50%))
                .wrap(),
            Button::default()
                .with_text("Reload theme")
                .with_event(UserEvent::ReloadTheme)
                .with_width(dimension!(!))
                .wrap(),
        ];

        Expandable::new(display, elements, false).wrap()
    }
}

#[derive(Serialize, Deserialize, Default, PrototypeWindow)]
#[window_title("Theme Viewer")]
#[window_class("theme_viewer")]
pub struct Theme {
    #[serde(skip)]
    pub theme_selector: ThemeSelector,
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
    pub profiler: ProfilerTheme,
    pub status_bar: StatusBarTheme,
    pub indicator: IndicatorTheme,
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
