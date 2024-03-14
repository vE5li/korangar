use std::marker::ConstParamTy;

use korangar_interface::application::{Application, ScalingTrait};
use korangar_interface::elements::{Container, ElementCell, ElementWrap, PickList, PrototypeElement, Text};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainTrackedState, TrackedStateClone};
use korangar_interface::windows::PrototypeWindow;
use korangar_procedural::{dimension_bound, profile, PrototypeElement};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::elements::MutableRange;
use super::layout::{CornerRadius, PartialScreenSize, ScreenClip, ScreenPosition, ScreenSize};
use super::resource::{Move, PartialMove};
use super::theme::{DefaultMain, DefaultMenu, GameTheme, InterfaceTheme, InterfaceThemeKind, Themes};
use super::windows::WindowCache;
use crate::graphics::{Color, InterfaceRenderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::loaders::{FontLoader, FontSize, Scaling};

impl korangar_interface::application::ColorTrait for Color {
    fn is_transparent(&self) -> bool {
        const TRANSPARENCY_THRESHOLD: f32 = 0.999;
        self.alpha < TRANSPARENCY_THRESHOLD
    }
}

impl korangar_interface::application::SizeTrait for ScreenSize {
    fn new(width: f32, height: f32) -> Self {
        ScreenSize { width, height }
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn height(&self) -> f32 {
        self.height
    }
}

impl korangar_interface::application::PositionTrait for ScreenPosition {
    fn new(left: f32, top: f32) -> Self {
        ScreenPosition { left, top }
    }

    fn left(&self) -> f32 {
        self.left
    }

    fn top(&self) -> f32 {
        self.top
    }
}

impl korangar_interface::application::ClipTrait for ScreenClip {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, right, top, bottom }
    }

    fn left(&self) -> f32 {
        self.left
    }

    fn right(&self) -> f32 {
        self.right
    }

    fn top(&self) -> f32 {
        self.top
    }

    fn bottom(&self) -> f32 {
        self.bottom
    }
}

impl korangar_interface::application::CornerRadiusTrait for CornerRadius {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    fn top_left(&self) -> f32 {
        self.top_left
    }

    fn top_right(&self) -> f32 {
        self.top_right
    }

    fn bottom_right(&self) -> f32 {
        self.bottom_right
    }

    fn bottom_left(&self) -> f32 {
        self.bottom_left
    }
}

impl korangar_interface::application::PartialSizeTrait for PartialScreenSize {
    fn new(width: f32, height: Option<f32>) -> Self {
        Self { width, height }
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn height(&self) -> Option<f32> {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalThemeKind {
    Main,
    Menu,
    Game,
}

impl ConstParamTy for InternalThemeKind {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThemeSelector<const KIND: InternalThemeKind>(pub String);

impl<const KIND: InternalThemeKind> ThemeSelector<KIND> {
    pub fn get_file(&self) -> &str {
        &self.0
    }

    pub fn set_file(&mut self, file: String) {
        self.0 = file;
    }
}

impl<const KIND: InternalThemeKind> PrototypeElement<InterfaceSettings> for ThemeSelector<KIND> {
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        let state = PlainTrackedState::new(self.0.clone());

        let themes = WalkDir::new("client/themes/")
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|path| {
                let name = path.path().file_name()?.to_str()?.strip_suffix(".ron")?.to_owned();
                let file_path = format!("client/themes/{}.ron", name);
                Some((name, file_path))
            })
            .collect();

        let elements = vec![
            Text::default().with_text(display).with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(themes)
                .with_selected(state.clone())
                .with_event(move || {
                    vec![ClickAction::Custom(UserEvent::SetThemeFile {
                        theme_file: state.cloned(),
                        theme_kind: KIND,
                    })]
                })
                .with_width(dimension_bound!(!))
                .wrap(),
        ];

        Container::new(elements).wrap()
    }
}

#[derive(Serialize, Deserialize)]
struct InterfaceSettingsStorage {
    menu_theme: String,
    main_theme: String,
    game_theme: String,
    scaling: Scaling,
}

impl Default for InterfaceSettingsStorage {
    fn default() -> Self {
        let main_theme = "client/themes/main.ron".to_string();
        let menu_theme = "client/themes/menu.ron".to_string();
        let game_theme = "client/themes/game.ron".to_string();
        let scaling = Scaling::new(1.0);

        Self {
            main_theme,
            menu_theme,
            game_theme,
            scaling,
        }
    }
}

impl InterfaceSettingsStorage {
    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            crate::debug::print_debug!(
                "failed to load interface settings from {}filename{}",
                crate::debug::MAGENTA,
                crate::debug::NONE
            );

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        crate::debug::print_debug!(
            "loading interface settings from {}filename{}",
            crate::debug::MAGENTA,
            crate::debug::NONE
        );

        std::fs::read_to_string("client/interface_settings.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        crate::debug::print_debug!(
            "saving interface settings to {}filename{}",
            crate::debug::MAGENTA,
            crate::debug::NONE
        );

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write("client/interface_settings.ron", data).expect("unable to write file");
    }
}

#[derive(PrototypeElement)]
pub struct InterfaceSettings {
    #[name("Main theme")]
    pub main_theme: ThemeSelector<{ InternalThemeKind::Main }>,
    #[name("Menu theme")]
    pub menu_theme: ThemeSelector<{ InternalThemeKind::Menu }>,
    #[name("Game theme")]
    pub game_theme: ThemeSelector<{ InternalThemeKind::Game }>,
    scaling: MutableRange<Scaling, korangar_interface::event::Resolve>,
    #[hidden_element]
    themes: Themes,
}

impl InterfaceSettings {
    pub fn new() -> Self {
        let InterfaceSettingsStorage {
            menu_theme,
            main_theme,
            game_theme,
            scaling,
        } = InterfaceSettingsStorage::load_or_default();

        let themes = Themes::new(
            InterfaceTheme::new::<super::theme::DefaultMenu>(&menu_theme),
            InterfaceTheme::new::<super::theme::DefaultMain>(&main_theme),
            GameTheme::new(&menu_theme),
        );

        Self {
            main_theme: ThemeSelector(main_theme),
            menu_theme: ThemeSelector(menu_theme),
            game_theme: ThemeSelector(game_theme),
            scaling: MutableRange::new(scaling, Scaling::new(0.5), Scaling::new(2.5)),
            themes,
        }
    }

    // TODO: Remove
    pub fn get_scaling_factor(&self) -> f32 {
        self.scaling.get().get_factor()
    }

    pub fn theme_window(&self) -> &dyn PrototypeWindow<InterfaceSettings> {
        &self.themes
    }

    pub fn get_game_theme(&self) -> &GameTheme {
        &self.themes.game
    }
}

impl InterfaceSettings {
    #[profile]
    pub fn set_theme_file(&mut self, theme_file: String, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.menu_theme.set_file(theme_file),
            InternalThemeKind::Main => self.main_theme.set_file(theme_file),
            InternalThemeKind::Game => self.game_theme.set_file(theme_file),
        }
    }

    #[profile]
    pub fn save_theme(&self, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.themes.menu.save(self.menu_theme.get_file()),
            InternalThemeKind::Main => self.themes.main.save(self.main_theme.get_file()),
            InternalThemeKind::Game => self.themes.game.save(self.game_theme.get_file()),
        }
    }

    #[profile]
    pub fn reload_theme(&mut self, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.themes.menu.reload::<DefaultMenu>(self.menu_theme.get_file()),
            InternalThemeKind::Main => self.themes.main.reload::<DefaultMain>(self.main_theme.get_file()),
            InternalThemeKind::Game => self.themes.game.reload(self.game_theme.get_file()),
        }
    }
}

impl Application for InterfaceSettings {
    type Cache = WindowCache;
    type Clip = ScreenClip;
    type Color = Color;
    type CornerRadius = CornerRadius;
    type CustomEvent = UserEvent;
    type DropResource = PartialMove;
    type DropResult = Move;
    type FontLoader = std::rc::Rc<std::cell::RefCell<FontLoader>>;
    type FontSize = FontSize;
    type MouseInputMode = MouseInputMode;
    type PartialSize = PartialScreenSize;
    type Position = ScreenPosition;
    type Renderer = InterfaceRenderer;
    type Scaling = Scaling;
    type Size = ScreenSize;
    type Theme = InterfaceTheme;
    type ThemeKind = InterfaceThemeKind;

    fn get_scaling(&self) -> Self::Scaling {
        self.scaling.get()
    }

    fn get_theme(&self, kind: &InterfaceThemeKind) -> &InterfaceTheme {
        match kind {
            InterfaceThemeKind::Menu => &self.themes.menu,
            InterfaceThemeKind::Main => &self.themes.main,
        }
    }
}

impl Drop for InterfaceSettings {
    fn drop(&mut self) {
        InterfaceSettingsStorage {
            menu_theme: self.menu_theme.get_file().to_owned(),
            main_theme: self.main_theme.get_file().to_owned(),
            game_theme: self.game_theme.get_file().to_owned(),
            scaling: self.scaling.get(),
        }
        .save();
    }
}
