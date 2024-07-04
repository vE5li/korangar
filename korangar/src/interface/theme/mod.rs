#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::PrototypeElement;
use korangar_interface::event::{Nothing, Render, Resolve};
use korangar_interface::layout::{DimensionBound, SizeBound};
use korangar_interface::theme::{
    ButtonTheme, CloseButtonTheme, ExpandableTheme, InputTheme, LabelTheme, SliderTheme, ValueTheme, WindowTheme,
};
use korangar_interface::windows::PrototypeWindow;
use korangar_interface::{dimension_bound, size_bound};
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
mod actions;

#[cfg(feature = "debug")]
use self::actions::ThemeActions;
use super::elements::{Mutable, MutableRange};
use super::layout::{CornerRadius, ScreenPosition, ScreenSize};
use crate::graphics::Color;
use crate::loaders::FontSize;
use crate::threads::Deferred;
use crate::GameState;

/// Themes the user interface can use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InterfaceThemeKind {
    Menu,
    #[default]
    Main,
}

/// Marker trait to specialize the [`ThemeDefault`] trait.
pub trait ThemeKindMarker {}

/// Default theme in the menu.
pub struct DefaultMenu;
impl ThemeKindMarker for DefaultMenu {}

/// Default theme when in game.
pub struct DefaultMain;
impl ThemeKindMarker for DefaultMain {}

/// Default trait that can be specialized over a theme type.
pub trait ThemeDefault<T: ThemeKindMarker> {
    fn default() -> Self;
}

impl ThemeDefault<DefaultMenu> for ButtonTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(150, 70, 255),
            hovered_background_color: Color::rgb_u8(200, 70, 255),
            disabled_background_color: Color::monochrome_u8(70),
            foreground_color: Color::monochrome_u8(200),
            hovered_foreground_color: Color::rgb_u8(220, 170, 215),
            disabled_foreground_color: Color::monochrome_u8(140),
            debug_foreground_color: Color::rgb_u8(230, 140, 230),
            corner_radius: CornerRadius::uniform(26.0),
            icon_offset: ScreenPosition { left: 7.0, top: 2.5 },
            icon_size: ScreenSize::uniform(16.0),
            icon_text_offset: ScreenPosition { left: 40.0, top: 4.0 },
            text_offset: ScreenPosition { left: 15.0, top: 6.0 },
            font_size: FontSize::new(14.0),
            height_bound: dimension_bound!(26),
        }
    }
}

impl ThemeDefault<DefaultMain> for ButtonTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(100),
            hovered_background_color: Color::rgb_u8(140, 120, 140),
            disabled_background_color: Color::monochrome_u8(70),
            foreground_color: Color::monochrome_u8(200),
            hovered_foreground_color: Color::rgb_u8(220, 170, 215),
            disabled_foreground_color: Color::monochrome_u8(140),
            debug_foreground_color: Color::rgb_u8(230, 140, 230),
            corner_radius: CornerRadius::uniform(6.0),
            icon_offset: ScreenPosition { left: 7.0, top: 2.5 },
            icon_size: ScreenSize::uniform(10.0),
            icon_text_offset: ScreenPosition { left: 20.0, top: 1.0 },
            text_offset: ScreenPosition { left: 5.0, top: 1.0 },
            font_size: FontSize::new(14.0),
            height_bound: dimension_bound!(16),
        }
    }
}

impl ThemeDefault<DefaultMenu> for WindowTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(30),
            title_background_color: Color::rgba_u8(70, 60, 70, 0),
            foreground_color: Color::rgb_u8(150, 70, 255),
            corner_radius: CornerRadius::uniform(30.0),
            title_corner_radius: CornerRadius::uniform(6.0),
            border_size: ScreenSize::uniform(30.0),
            text_offset: ScreenPosition { left: 5.0, top: -1.0 },
            gaps: ScreenSize { width: 9.0, height: 19.0 },
            font_size: FontSize::new(20.0),
            title_height: dimension_bound!(30),
            anchor_color: Color::rgba_u8(60, 60, 150, 255),
            closest_anchor_color: Color::rgba_u8(190, 125, 255, 255),
        }
    }
}

impl ThemeDefault<DefaultMain> for WindowTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(40),
            title_background_color: Color::rgb_u8(170, 60, 70),
            foreground_color: Color::monochrome_u8(160),
            corner_radius: CornerRadius::uniform(4.0),
            title_corner_radius: CornerRadius::uniform(6.0),
            border_size: ScreenSize { width: 12.0, height: 6.0 },
            text_offset: ScreenPosition { left: 5.0, top: -1.0 },
            gaps: ScreenSize { width: 4.0, height: 5.0 },
            font_size: FontSize::new(14.0),
            title_height: dimension_bound!(12),
            anchor_color: Color::rgba_u8(150, 100, 100, 255),
            closest_anchor_color: Color::rgba_u8(255, 180, 0, 255),
        }
    }
}

impl ThemeDefault<DefaultMenu> for ExpandableTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(60),
            second_background_color: Color::monochrome_u8(45),
            foreground_color: Color::monochrome_u8(170),
            hovered_foreground_color: Color::rgb_u8(190, 145, 185),
            corner_radius: CornerRadius::uniform(6.0),
            border_size: ScreenSize::uniform(5.0),
            element_offset: ScreenPosition { left: 7.0, top: -2.0 },
            icon_offset: ScreenPosition { left: 6.0, top: 5.0 },
            icon_size: ScreenSize::uniform(6.0),
            text_offset: ScreenPosition { left: 14.0, top: 1.5 },
            gaps: ScreenSize::uniform(6.0),
            font_size: FontSize::new(14.0),
        }
    }
}

impl ThemeDefault<DefaultMain> for ExpandableTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(60),
            second_background_color: Color::monochrome_u8(45),
            foreground_color: Color::monochrome_u8(170),
            hovered_foreground_color: Color::rgb_u8(190, 145, 185),
            corner_radius: CornerRadius::uniform(6.0),
            border_size: ScreenSize::uniform(5.0),
            element_offset: ScreenPosition { left: 7.0, top: -2.0 },
            icon_offset: ScreenPosition { left: 6.0, top: 5.0 },
            icon_size: ScreenSize::uniform(6.0),
            text_offset: ScreenPosition { left: 14.0, top: 1.5 },
            gaps: ScreenSize::uniform(6.0),
            font_size: FontSize::new(14.0),
        }
    }
}

impl ThemeDefault<DefaultMenu> for LabelTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(130),
            foreground_color: Color::monochrome_u8(255),
            corner_radius: CornerRadius::uniform(6.0),
            text_offset: ScreenPosition { left: 5.0, top: 0.0 },
            font_size: FontSize::new(14.0),
            size_bound: size_bound!(120 > 50% < 300, 0),
        }
    }
}

impl ThemeDefault<DefaultMain> for LabelTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(130),
            foreground_color: Color::monochrome_u8(255),
            corner_radius: CornerRadius::uniform(6.0),
            text_offset: ScreenPosition { left: 5.0, top: 0.0 },
            font_size: FontSize::new(14.0),
            size_bound: size_bound!(120 > 50% < 300, 0),
        }
    }
}

impl ThemeDefault<DefaultMenu> for ValueTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(100, 100, 100),
            hovered_background_color: Color::rgb_u8(130, 100, 120),
            foreground_color: Color::rgb_u8(220, 220, 220),
            corner_radius: CornerRadius::uniform(6.0),
            text_offset: ScreenPosition { left: 5.0, top: 0.0 },
            font_size: FontSize::new(14.0),
            size_bound: size_bound!(60 > !, 14),
        }
    }
}

impl ThemeDefault<DefaultMain> for ValueTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(100, 100, 100),
            hovered_background_color: Color::rgb_u8(130, 100, 120),
            foreground_color: Color::rgb_u8(220, 220, 220),
            corner_radius: CornerRadius::uniform(6.0),
            text_offset: ScreenPosition { left: 5.0, top: 0.0 },
            font_size: FontSize::new(14.0),
            size_bound: size_bound!(60 > !, 14),
        }
    }
}

impl ThemeDefault<DefaultMenu> for CloseButtonTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(200, 100, 100),
            hovered_background_color: Color::rgb_u8(200, 140, 100),
            foreground_color: Color::rgb_u8(220, 220, 220),
            corner_radius: CornerRadius::uniform(26.0),
            text_offset: ScreenPosition { left: 8.35, top: 2.55 },
            font_size: FontSize::new(20.0),
            size_bound: size_bound!(26, 26),
        }
    }
}

impl ThemeDefault<DefaultMain> for CloseButtonTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(200, 100, 100),
            hovered_background_color: Color::rgb_u8(200, 140, 100),
            foreground_color: Color::rgb_u8(220, 220, 220),
            corner_radius: CornerRadius::uniform(1.0),
            text_offset: ScreenPosition { left: 9.0, top: 0.0 },
            font_size: FontSize::new(12.0),
            size_bound: size_bound!(25, 12),
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct OverlayTheme {
    pub foreground_color: Color,
    pub text_offset: ScreenPosition,
    pub font_size: FontSize,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            foreground_color: Color::monochrome_u8(220),
            text_offset: ScreenPosition { left: 20.0, top: 10.0 },
            font_size: FontSize::new(18.0),
        }
    }
}

impl ThemeDefault<DefaultMenu> for SliderTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(140, 80, 100),
            rail_color: Color::rgb_u8(150, 130, 150),
            knob_color: Color::rgb_u8(100, 180, 180),
            size_bound: size_bound!(100%, 18),
        }
    }
}

impl ThemeDefault<DefaultMain> for SliderTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(140, 80, 100),
            rail_color: Color::rgb_u8(150, 130, 150),
            knob_color: Color::rgb_u8(100, 180, 180),
            size_bound: size_bound!(100%, 18),
        }
    }
}

impl ThemeDefault<DefaultMenu> for InputTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(45),
            hovered_background_color: Color::rgb_u8(70, 60, 80),
            focused_background_color: Color::monochrome_u8(100),
            text_color: Color::monochrome_u8(200),
            ghost_text_color: Color::monochrome_u8(100),
            focused_text_color: Color::monochrome_u8(200),
            corner_radius: CornerRadius::uniform(26.0),
            font_size: FontSize::new(15.0),
            text_offset: ScreenPosition { left: 15.0, top: 6.0 },
            cursor_offset: 2.0,
            cursor_width: 3.0,
            height_bound: dimension_bound!(26),
        }
    }
}

impl ThemeDefault<DefaultMain> for InputTheme<GameState> {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(60),
            hovered_background_color: Color::monochrome_u8(80),
            focused_background_color: Color::monochrome_u8(100),
            text_color: Color::monochrome_u8(200),
            ghost_text_color: Color::monochrome_u8(100),
            focused_text_color: Color::monochrome_u8(200),
            corner_radius: CornerRadius::uniform(6.0),
            font_size: FontSize::new(14.0),
            text_offset: ScreenPosition { left: 4.0, top: 0.0 },
            cursor_offset: 2.0,
            cursor_width: 3.0,
            height_bound: dimension_bound!(15),
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct ChatTheme {
    pub background_color: Color,
    pub font_size: FontSize,
    pub broadcast_color: Color,
    pub server_color: Color,
    pub error_color: Color,
    pub information_color: Color,
}

impl ThemeDefault<DefaultMenu> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Color::rgba_u8(0, 0, 0, 170),
            font_size: FontSize::new(14.0),
            broadcast_color: Color::rgb_u8(210, 210, 210),
            server_color: Color::rgb_u8(255, 255, 210),
            error_color: Color::rgb_u8(255, 150, 150),
            information_color: Color::rgb_u8(200, 255, 200),
        }
    }
}

impl ThemeDefault<DefaultMain> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Color::rgba_u8(0, 0, 0, 170),
            font_size: FontSize::new(14.0),
            broadcast_color: Color::rgb_u8(210, 210, 210),
            server_color: Color::rgb_u8(255, 255, 210),
            error_color: Color::rgb_u8(255, 150, 150),
            information_color: Color::rgb_u8(200, 255, 200),
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct CursorTheme {
    pub color: Color,
}

impl Default for CursorTheme {
    fn default() -> Self {
        Self {
            color: Color::monochrome_u8(255),
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct ProfilerTheme {
    pub background_color: Color,
    pub corner_radius: CornerRadius,
    pub line_color: Color,
    pub line_width: f32,
    pub bar_height: f32,
    pub bar_gap: ScreenSize,
    pub bar_corner_radius: CornerRadius,
    pub bar_text_color: Color,
    pub bar_text_size: f32,
    pub bar_text_offset: ScreenPosition,
    pub distance_text_size: f32,
    pub distance_text_offset: f32,
}

impl ThemeDefault<DefaultMenu> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(55),
            corner_radius: CornerRadius::uniform(2.0),
            line_color: Color::rgb_u8(80, 90, 80),
            line_width: 2.0,
            bar_height: 15.0,
            bar_gap: ScreenSize { width: 1.0, height: 5.0 },
            bar_corner_radius: CornerRadius::default(),
            bar_text_color: Color::monochrome_u8(0),
            bar_text_size: 14.0,
            bar_text_offset: ScreenPosition { left: 7.0, top: 0.0 },
            distance_text_size: 12.0,
            distance_text_offset: 20.0,
        }
    }
}

impl ThemeDefault<DefaultMain> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(55),
            corner_radius: CornerRadius::uniform(2.0),
            line_color: Color::rgb_u8(80, 90, 80),
            line_width: 2.0,
            bar_height: 15.0,
            bar_gap: ScreenSize { width: 1.0, height: 5.0 },
            bar_corner_radius: CornerRadius::default(),
            bar_text_color: Color::monochrome_u8(0),
            bar_text_size: 14.0,
            bar_text_offset: ScreenPosition { left: 7.0, top: 0.0 },
            distance_text_size: 12.0,
            distance_text_offset: 20.0,
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct StatusBarTheme {
    pub background_color: Color,
    pub player_health_color: Color,
    pub enemy_health_color: Color,
    pub spell_point_color: Color,
    pub activity_point_color: Color,
    pub player_bar_width: f32,
    pub enemy_bar_width: f32,
    pub health_height: f32,
    pub enemy_health_height: f32,
    pub spell_point_height: f32,
    pub activity_point_height: f32,
    pub border_size: ScreenSize,
    pub gap: f32,
}

impl Default for StatusBarTheme {
    fn default() -> Self {
        Self {
            background_color: Color::monochrome_u8(40),
            player_health_color: Color::rgb_u8(67, 163, 83),
            enemy_health_color: Color::rgb_u8(206, 49, 116),
            spell_point_color: Color::rgb_u8(0, 129, 163),
            activity_point_color: Color::rgb_u8(218, 145, 81),
            player_bar_width: 85.0,
            enemy_bar_width: 60.0,
            health_height: 8.0,
            enemy_health_height: 6.0,
            spell_point_height: 4.0,
            activity_point_height: 4.0,
            border_size: ScreenSize { width: 2.0, height: 1.0 },
            gap: 1.0,
        }
    }
}

#[derive(RustState, Serialize, Deserialize)]
pub struct IndicatorTheme {
    pub walking: Color,
}

impl Default for IndicatorTheme {
    fn default() -> Self {
        Self {
            walking: Color::rgba_u8(0, 255, 170, 170),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct InterfaceTheme {
    pub button: ButtonTheme<GameState>,
    pub window: WindowTheme<GameState>,
    pub expandable: ExpandableTheme<GameState>,
    pub label: LabelTheme<GameState>,
    pub value: ValueTheme<GameState>,
    pub close_button: CloseButtonTheme<GameState>,
    pub slider: SliderTheme<GameState>,
    pub input: InputTheme<GameState>,
    pub profiler: ProfilerTheme,
    pub chat: ChatTheme,
}

impl<T: ThemeKindMarker> ThemeDefault<T> for InterfaceTheme
where
    ButtonTheme<GameState>: ThemeDefault<T>,
    WindowTheme<GameState>: ThemeDefault<T>,
    ExpandableTheme<GameState>: ThemeDefault<T>,
    LabelTheme<GameState>: ThemeDefault<T>,
    ValueTheme<GameState>: ThemeDefault<T>,
    CloseButtonTheme<GameState>: ThemeDefault<T>,
    SliderTheme<GameState>: ThemeDefault<T>,
    InputTheme<GameState>: ThemeDefault<T>,
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

#[derive(RustState, Serialize, Deserialize, Default)]
pub struct GameTheme {
    pub overlay: OverlayTheme,
    pub status_bar: StatusBarTheme,
    pub indicator: IndicatorTheme,
    pub cursor: CursorTheme,
}

/* #[derive(PrototypeWindow)]
#[window_title("Theme Viewer")]
#[window_class("theme_viewer")]
pub struct Themes {
    #[cfg(feature = "debug")]
    #[name("Actions")]
    theme_actions: ThemeActions,
    #[name("Menu")]
    pub menu: InterfaceTheme,
    #[name("Main")]
    pub main: InterfaceTheme,
    #[name("Game")]
    pub game: GameTheme,
} */

/* impl Themes {
    pub fn new(menu: InterfaceTheme, main: InterfaceTheme, game: GameTheme) -> Self {
        Self {
            #[cfg(feature = "debug")]
            theme_actions: Default::default(),
            menu,
            main,
            game,
        }
    }
}

impl InterfaceTheme {
    pub fn new<T: ThemeKindMarker>(theme_file: &str) -> Self
    where
        Self: ThemeDefault<T>,
    {
        Self::load(theme_file).unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}", theme_file.magenta());

            ThemeDefault::<T>::default()
        })
    }

    fn load(theme_file: &str) -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading theme from {}", theme_file.magenta());

        std::fs::read_to_string(theme_file).ok().and_then(|data| ron::from_str(&data).ok())
    }

    pub fn reload<T: ThemeKindMarker>(&mut self, theme_file: &str)
    where
        Self: ThemeDefault<T>,
    {
        *self = Self::new::<T>(theme_file);
    }

    pub fn save(&self, theme_file: &str) {
        #[cfg(feature = "debug")]
        print_debug!("saving theme to {}", theme_file.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(theme_file, data).expect("unable to write file");
    }
}

impl GameTheme {
    pub fn new(theme_file: &str) -> Self {
        Self::load(theme_file).unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load theme from file {}", theme_file.magenta());

            Default::default()
        })
    }

    fn load(theme_file: &str) -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading theme from {}", theme_file.magenta());

        std::fs::read_to_string(theme_file).ok().and_then(|data| ron::from_str(&data).ok())
    }

    pub fn reload(&mut self, theme_file: &str) {
        *self = Self::new(theme_file);
    }

    pub fn save(&self, theme_file: &str) {
        #[cfg(feature = "debug")]
        print_debug!("saving theme to {}", theme_file.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(theme_file, data).expect("unable to write file");
    }
} */
