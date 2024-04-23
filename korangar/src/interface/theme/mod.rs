#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::PrototypeElement;
use korangar_interface::event::{Nothing, Render, Resolve};
use korangar_interface::layout::{DimensionBound, SizeBound};
use korangar_interface::windows::PrototypeWindow;
use korangar_interface::{dimension_bound, size_bound};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
mod actions;

#[cfg(feature = "debug")]
use self::actions::ThemeActions;
use super::application::InterfaceSettings;
use super::elements::{Mutable, MutableRange};
use super::layout::{CornerRadius, ScreenPosition, ScreenSize};
use crate::graphics::Color;
use crate::loaders::FontSize;

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

// TODO: Make all theme fileds private. Use the traits to access fields
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
    pub font_size: MutableRange<FontSize, Render>,
    pub height_bound: DimensionBound,
}

impl ThemeDefault<DefaultMenu> for ButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(150, 70, 255)),
            hovered_background_color: Mutable::new(Color::rgb_u8(200, 70, 255)),
            disabled_background_color: Mutable::new(Color::monochrome_u8(70)),
            foreground_color: Mutable::new(Color::monochrome_u8(200)),
            hovered_foreground_color: Mutable::new(Color::rgb_u8(220, 170, 215)),
            disabled_foreground_color: Mutable::new(Color::monochrome_u8(140)),
            debug_foreground_color: Mutable::new(Color::rgb_u8(230, 140, 230)),
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
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            height_bound: dimension_bound!(26),
        }
    }
}

impl ThemeDefault<DefaultMain> for ButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(100)),
            hovered_background_color: Mutable::new(Color::rgb_u8(140, 120, 140)),
            disabled_background_color: Mutable::new(Color::monochrome_u8(70)),
            foreground_color: Mutable::new(Color::monochrome_u8(200)),
            hovered_foreground_color: Mutable::new(Color::rgb_u8(220, 170, 215)),
            disabled_foreground_color: Mutable::new(Color::monochrome_u8(140)),
            debug_foreground_color: Mutable::new(Color::rgb_u8(230, 140, 230)),
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
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            height_bound: dimension_bound!(16),
        }
    }
}

impl korangar_interface::theme::ButtonTheme<InterfaceSettings> for ButtonTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn hovered_background_color(&self) -> Color {
        self.hovered_background_color.get()
    }

    fn disabled_background_color(&self) -> Color {
        self.disabled_background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn hovered_foreground_color(&self) -> Color {
        self.hovered_foreground_color.get()
    }

    fn disabled_foreground_color(&self) -> Color {
        self.disabled_foreground_color.get()
    }

    fn debug_foreground_color(&self) -> Color {
        self.debug_foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn icon_offset(&self) -> ScreenPosition {
        self.icon_offset.get()
    }

    fn icon_size(&self) -> ScreenSize {
        self.icon_size.get()
    }

    fn icon_text_offset(&self) -> ScreenPosition {
        self.icon_text_offset.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn height_bound(&self) -> korangar_interface::layout::DimensionBound {
        self.height_bound
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
    pub font_size: MutableRange<FontSize, Render>,
    pub title_height: DimensionBound,
}

impl ThemeDefault<DefaultMenu> for WindowTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(30)),
            title_background_color: Mutable::new(Color::rgba_u8(70, 60, 70, 0)),
            foreground_color: Mutable::new(Color::rgb_u8(150, 70, 255)),
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
            font_size: MutableRange::new(FontSize::new(20.0), FontSize::new(6.0), FontSize::new(30.0)),
            title_height: dimension_bound!(30),
        }
    }
}

impl ThemeDefault<DefaultMain> for WindowTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(40)),
            title_background_color: Mutable::new(Color::rgb_u8(170, 60, 70)),
            foreground_color: Mutable::new(Color::monochrome_u8(160)),
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
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            title_height: dimension_bound!(12),
        }
    }
}

impl korangar_interface::theme::WindowTheme<InterfaceSettings> for WindowTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn title_background_color(&self) -> Color {
        self.title_background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn title_corner_radius(&self) -> CornerRadius {
        self.title_corner_radius.get()
    }

    fn border_size(&self) -> ScreenSize {
        self.border_size.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn gaps(&self) -> ScreenSize {
        self.gaps.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn title_height(&self) -> korangar_interface::layout::DimensionBound {
        self.title_height
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
    pub font_size: MutableRange<FontSize, Render>,
}

impl ThemeDefault<DefaultMenu> for ExpandableTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(60)),
            second_background_color: Mutable::new(Color::monochrome_u8(45)),
            foreground_color: Mutable::new(Color::monochrome_u8(170)),
            hovered_foreground_color: Mutable::new(Color::rgb_u8(190, 145, 185)),
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
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
        }
    }
}

impl ThemeDefault<DefaultMain> for ExpandableTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(60)),
            second_background_color: Mutable::new(Color::monochrome_u8(45)),
            foreground_color: Mutable::new(Color::monochrome_u8(170)),
            hovered_foreground_color: Mutable::new(Color::rgb_u8(190, 145, 185)),
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
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
        }
    }
}

impl korangar_interface::theme::ExpandableTheme<InterfaceSettings> for ExpandableTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn second_background_color(&self) -> Color {
        self.second_background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn hovered_foreground_color(&self) -> Color {
        self.hovered_foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn border_size(&self) -> ScreenSize {
        self.border_size.get()
    }

    fn element_offset(&self) -> ScreenPosition {
        self.element_offset.get()
    }

    fn icon_offset(&self) -> ScreenPosition {
        self.icon_offset.get()
    }

    fn icon_size(&self) -> ScreenSize {
        self.icon_size.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn gaps(&self) -> ScreenSize {
        self.gaps.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct LabelTheme {
    pub background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<FontSize, Render>,
    pub size_bound: SizeBound,
}

impl ThemeDefault<DefaultMenu> for LabelTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(130)),
            foreground_color: Mutable::new(Color::monochrome_u8(255)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(120 > 50% < 300, 0),
        }
    }
}

impl ThemeDefault<DefaultMain> for LabelTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(130)),
            foreground_color: Mutable::new(Color::monochrome_u8(255)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(120 > 50% < 300, 0),
        }
    }
}

impl korangar_interface::theme::LabelTheme<InterfaceSettings> for LabelTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn size_bound(&self) -> korangar_interface::layout::SizeBound {
        self.size_bound
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ValueTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<FontSize, Render>,
    pub size_bound: SizeBound,
}

impl ThemeDefault<DefaultMenu> for ValueTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(100, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb_u8(130, 100, 120)),
            foreground_color: Mutable::new(Color::rgb_u8(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(60 > !, 14),
        }
    }
}

impl ThemeDefault<DefaultMain> for ValueTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(100, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb_u8(130, 100, 120)),
            foreground_color: Mutable::new(Color::rgb_u8(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 5.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(60 > !, 14),
        }
    }
}

impl korangar_interface::theme::ValueTheme<InterfaceSettings> for ValueTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn hovered_background_color(&self) -> Color {
        self.hovered_background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn size_bound(&self) -> korangar_interface::layout::SizeBound {
        self.size_bound
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CloseButtonTheme {
    pub background_color: Mutable<Color, Render>,
    pub hovered_background_color: Mutable<Color, Render>,
    pub foreground_color: Mutable<Color, Render>,
    pub corner_radius: MutableRange<CornerRadius, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub font_size: MutableRange<FontSize, Render>,
    pub size_bound: SizeBound,
}

impl ThemeDefault<DefaultMenu> for CloseButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(200, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb_u8(200, 140, 100)),
            foreground_color: Mutable::new(Color::rgb_u8(220, 220, 220)),
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
            font_size: MutableRange::new(FontSize::new(20.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(26, 26),
        }
    }
}

impl ThemeDefault<DefaultMain> for CloseButtonTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(200, 100, 100)),
            hovered_background_color: Mutable::new(Color::rgb_u8(200, 140, 100)),
            foreground_color: Mutable::new(Color::rgb_u8(220, 220, 220)),
            corner_radius: MutableRange::new(CornerRadius::uniform(1.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 9.0, top: 0.0 },
                ScreenPosition::uniform(-10.0),
                ScreenPosition::uniform(20.0),
            ),
            font_size: MutableRange::new(FontSize::new(12.0), FontSize::new(6.0), FontSize::new(30.0)),
            size_bound: size_bound!(25, 12),
        }
    }
}

impl korangar_interface::theme::CloseButtonTheme<InterfaceSettings> for CloseButtonTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn hovered_background_color(&self) -> Color {
        self.hovered_background_color.get()
    }

    fn foreground_color(&self) -> Color {
        self.foreground_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn size_bound(&self) -> korangar_interface::layout::SizeBound {
        self.size_bound
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct OverlayTheme {
    pub foreground_color: Mutable<Color, Nothing>,
    pub text_offset: MutableRange<ScreenPosition, Nothing>,
    pub font_size: MutableRange<FontSize, Nothing>,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            foreground_color: Mutable::new(Color::monochrome_u8(220)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 20.0, top: 10.0 },
                ScreenPosition::default(),
                ScreenPosition { left: 1000.0, top: 500.0 },
            ),
            font_size: MutableRange::new(FontSize::new(18.0), FontSize::new(6.0), FontSize::new(50.0)),
        }
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct SliderTheme {
    pub background_color: Mutable<Color, Render>,
    pub rail_color: Mutable<Color, Render>,
    pub knob_color: Mutable<Color, Render>,
    pub size_bound: SizeBound,
}

impl ThemeDefault<DefaultMenu> for SliderTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(140, 80, 100)),
            rail_color: Mutable::new(Color::rgb_u8(150, 130, 150)),
            knob_color: Mutable::new(Color::rgb_u8(100, 180, 180)),
            size_bound: size_bound!(100%, 18),
        }
    }
}

impl ThemeDefault<DefaultMain> for SliderTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgb_u8(140, 80, 100)),
            rail_color: Mutable::new(Color::rgb_u8(150, 130, 150)),
            knob_color: Mutable::new(Color::rgb_u8(100, 180, 180)),
            size_bound: size_bound!(100%, 18),
        }
    }
}

impl korangar_interface::theme::SliderTheme<InterfaceSettings> for SliderTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn rail_color(&self) -> Color {
        self.rail_color.get()
    }

    fn knob_color(&self) -> Color {
        self.knob_color.get()
    }

    fn size_bound(&self) -> korangar_interface::layout::SizeBound {
        self.size_bound.clone()
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
    pub font_size: MutableRange<FontSize, Render>,
    pub text_offset: MutableRange<ScreenPosition, Render>,
    pub cursor_offset: MutableRange<f32, Render>,
    pub cursor_width: MutableRange<f32, Render>,
    pub height_bound: DimensionBound,
}

impl ThemeDefault<DefaultMenu> for InputTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(45)),
            hovered_background_color: Mutable::new(Color::rgb_u8(70, 60, 80)),
            focused_background_color: Mutable::new(Color::monochrome_u8(100)),
            text_color: Mutable::new(Color::monochrome_u8(200)),
            ghost_text_color: Mutable::new(Color::monochrome_u8(100)),
            focused_text_color: Mutable::new(Color::monochrome_u8(200)),
            corner_radius: MutableRange::new(
                CornerRadius::uniform(26.0),
                CornerRadius::default(),
                CornerRadius::uniform(30.0),
            ),
            font_size: MutableRange::new(FontSize::new(15.0), FontSize::new(6.0), FontSize::new(50.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 15.0, top: 6.0 },
                ScreenPosition::default(),
                ScreenPosition::uniform(50.0),
            ),
            cursor_offset: MutableRange::new(2.0, 0.0, 10.0),
            cursor_width: MutableRange::new(3.0, 2.0, 30.0),
            height_bound: dimension_bound!(26),
        }
    }
}

impl ThemeDefault<DefaultMain> for InputTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(60)),
            hovered_background_color: Mutable::new(Color::monochrome_u8(80)),
            focused_background_color: Mutable::new(Color::monochrome_u8(100)),
            text_color: Mutable::new(Color::monochrome_u8(200)),
            ghost_text_color: Mutable::new(Color::monochrome_u8(100)),
            focused_text_color: Mutable::new(Color::monochrome_u8(200)),
            corner_radius: MutableRange::new(CornerRadius::uniform(6.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(50.0)),
            text_offset: MutableRange::new(
                ScreenPosition { left: 4.0, top: 0.0 },
                ScreenPosition::default(),
                ScreenPosition::uniform(50.0),
            ),
            cursor_offset: MutableRange::new(2.0, 0.0, 10.0),
            cursor_width: MutableRange::new(3.0, 2.0, 30.0),
            height_bound: dimension_bound!(15),
        }
    }
}

impl korangar_interface::theme::InputTheme<InterfaceSettings> for InputTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn hovered_background_color(&self) -> Color {
        self.hovered_background_color.get()
    }

    fn focused_background_color(&self) -> Color {
        self.focused_background_color.get()
    }

    fn text_color(&self) -> Color {
        self.text_color.get()
    }

    fn ghost_text_color(&self) -> Color {
        self.ghost_text_color.get()
    }

    fn focused_text_color(&self) -> Color {
        self.focused_text_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn text_offset(&self) -> ScreenPosition {
        self.text_offset.get()
    }

    fn cursor_offset(&self) -> f32 {
        self.cursor_offset.get()
    }

    fn cursor_width(&self) -> f32 {
        self.cursor_width.get()
    }

    fn height_bound(&self) -> korangar_interface::layout::DimensionBound {
        self.height_bound
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct ChatTheme {
    pub background_color: Mutable<Color, Render>,
    pub font_size: MutableRange<FontSize, Render>,
    pub broadcast_color: Mutable<Color, Render>,
    pub server_color: Mutable<Color, Render>,
    pub error_color: Mutable<Color, Render>,
    pub information_color: Mutable<Color, Render>,
}

impl ThemeDefault<DefaultMenu> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgba_u8(0, 0, 0, 170)),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(50.0)),
            broadcast_color: Mutable::new(Color::rgb_u8(210, 210, 210)),
            server_color: Mutable::new(Color::rgb_u8(255, 255, 210)),
            error_color: Mutable::new(Color::rgb_u8(255, 150, 150)),
            information_color: Mutable::new(Color::rgb_u8(200, 255, 200)),
        }
    }
}

impl ThemeDefault<DefaultMain> for ChatTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::rgba_u8(0, 0, 0, 170)),
            font_size: MutableRange::new(FontSize::new(14.0), FontSize::new(6.0), FontSize::new(50.0)),
            broadcast_color: Mutable::new(Color::rgb_u8(210, 210, 210)),
            server_color: Mutable::new(Color::rgb_u8(255, 255, 210)),
            error_color: Mutable::new(Color::rgb_u8(255, 150, 150)),
            information_color: Mutable::new(Color::rgb_u8(200, 255, 200)),
        }
    }
}

impl korangar_interface::theme::ChatTheme<InterfaceSettings> for ChatTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn font_size(&self) -> FontSize {
        self.font_size.get()
    }

    fn broadcast_color(&self) -> Color {
        self.broadcast_color.get()
    }

    fn server_color(&self) -> Color {
        self.server_color.get()
    }

    fn error_color(&self) -> Color {
        self.error_color.get()
    }

    fn information_color(&self) -> Color {
        self.information_color.get()
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct CursorTheme {
    pub color: Mutable<Color, Nothing>,
}

impl Default for CursorTheme {
    fn default() -> Self {
        Self {
            color: Mutable::new(Color::monochrome_u8(255)),
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

impl ThemeDefault<DefaultMenu> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(55)),
            corner_radius: MutableRange::new(CornerRadius::uniform(2.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            line_color: Mutable::new(Color::rgb_u8(80, 90, 80)),
            line_width: MutableRange::new(2.0, 0.5, 4.0),
            bar_height: MutableRange::new(15.0, 5.0, 30.0),
            bar_gap: MutableRange::new(ScreenSize { width: 1.0, height: 5.0 }, ScreenSize::default(), ScreenSize {
                width: 10.0,
                height: 20.0,
            }),
            bar_corner_radius: MutableRange::new(CornerRadius::default(), CornerRadius::default(), CornerRadius::uniform(15.0)),
            bar_text_color: Mutable::new(Color::monochrome_u8(0)),
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

impl ThemeDefault<DefaultMain> for ProfilerTheme {
    fn default() -> Self {
        Self {
            background_color: Mutable::new(Color::monochrome_u8(55)),
            corner_radius: MutableRange::new(CornerRadius::uniform(2.0), CornerRadius::default(), CornerRadius::uniform(30.0)),
            line_color: Mutable::new(Color::rgb_u8(80, 90, 80)),
            line_width: MutableRange::new(2.0, 0.5, 4.0),
            bar_height: MutableRange::new(15.0, 5.0, 30.0),
            bar_gap: MutableRange::new(ScreenSize { width: 1.0, height: 5.0 }, ScreenSize::default(), ScreenSize {
                width: 10.0,
                height: 20.0,
            }),
            bar_corner_radius: MutableRange::new(CornerRadius::default(), CornerRadius::default(), CornerRadius::uniform(15.0)),
            bar_text_color: Mutable::new(Color::monochrome_u8(0)),
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

impl korangar_interface::theme::ProfilerTheme<InterfaceSettings> for ProfilerTheme {
    fn background_color(&self) -> Color {
        self.background_color.get()
    }

    fn corner_radius(&self) -> CornerRadius {
        self.corner_radius.get()
    }

    fn line_color(&self) -> Color {
        self.line_color.get()
    }

    fn line_width(&self) -> f32 {
        self.line_width.get()
    }

    fn bar_height(&self) -> f32 {
        self.bar_height.get()
    }

    fn bar_gap(&self) -> ScreenSize {
        self.bar_gap.get()
    }

    fn bar_corner_radius(&self) -> CornerRadius {
        self.bar_corner_radius.get()
    }

    fn bar_text_color(&self) -> Color {
        self.bar_text_color.get()
    }

    fn bar_text_size(&self) -> f32 {
        self.bar_text_size.get()
    }

    fn bar_text_offset(&self) -> ScreenPosition {
        self.bar_text_offset.get()
    }

    fn distance_text_size(&self) -> f32 {
        self.distance_text_size.get()
    }

    fn distance_text_offset(&self) -> f32 {
        self.distance_text_offset.get()
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
            background_color: Mutable::new(Color::monochrome_u8(40)),
            player_health_color: Mutable::new(Color::rgb_u8(67, 163, 83)),
            enemy_health_color: Mutable::new(Color::rgb_u8(206, 49, 116)),
            spell_point_color: Mutable::new(Color::rgb_u8(0, 129, 163)),
            activity_point_color: Mutable::new(Color::rgb_u8(218, 145, 81)),
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
            walking: Mutable::new(Color::rgba_u8(0, 255, 170, 170)),
        }
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

impl<T: ThemeKindMarker> ThemeDefault<T> for InterfaceTheme
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

impl korangar_interface::theme::InterfaceTheme for InterfaceTheme {
    type Button = ButtonTheme;
    type Chat = ChatTheme;
    type CloseButton = CloseButtonTheme;
    type Expandable = ExpandableTheme;
    type Input = InputTheme;
    type Label = LabelTheme;
    type Profiler = ProfilerTheme;
    type Settings = InterfaceSettings;
    type Slider = SliderTheme;
    type Value = ValueTheme;
    type Window = WindowTheme;

    fn button(&self) -> &Self::Button {
        &self.button
    }

    fn window(&self) -> &Self::Window {
        &self.window
    }

    fn expandable(&self) -> &Self::Expandable {
        &self.expandable
    }

    fn label(&self) -> &Self::Label {
        &self.label
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn close_button(&self) -> &Self::CloseButton {
        &self.close_button
    }

    fn slider(&self) -> &Self::Slider {
        &self.slider
    }

    fn input(&self) -> &Self::Input {
        &self.input
    }

    fn profiler(&self) -> &Self::Profiler {
        &self.profiler
    }

    fn chat(&self) -> &Self::Chat {
        &self.chat
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
    #[cfg(feature = "debug")]
    #[name("Actions")]
    theme_actions: ThemeActions,
    #[name("Menu")]
    pub menu: InterfaceTheme,
    #[name("Main")]
    pub main: InterfaceTheme,
    #[name("Game")]
    pub game: GameTheme,
}

impl Themes {
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
}
