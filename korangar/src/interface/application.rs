use std::marker::ConstParamTy;

#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_interface::application::{Application, ScalingTrait};
use korangar_interface::dimension_bound;
use korangar_interface::elements::{Container, ElementCell, ElementWrap, PickList, PrototypeElement, Text};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainTrackedState, TrackedStateClone};
use ron::ser::PrettyConfig;
use rust_state::{PathId, PathUuid, RawSelector};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::layout::{CornerRadius, PartialScreenSize, ScreenClip, ScreenPosition, ScreenSize};
use super::resource::{Move, PartialMove};
use super::theme::InterfaceThemeKind;
use super::windows::WindowCache;
use crate::graphics::{Color, InterfaceRenderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::loaders::{FontLoader, FontSize, Scaling};
use crate::{GameState, GameStateFocusedElementPath, GameStateHoveredElementPath, GameStateMouseModePath, GameStateScalePath};

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

/* #[derive(Serialize, Deserialize)]
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

impl<const KIND: InternalThemeKind> PrototypeElement<GameState> for ThemeSelector<KIND> {
    fn to_element(&self, display: String) -> ElementCell<GameState> {
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
} */

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
    const FILE_NAME: &'static str = "client/interface_settings.ron";

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load interface settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading interface settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
    }
}

/*#[derive(PrototypeElement)]
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
    pub fn load_or_default() -> Self {
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
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn set_theme_file(&mut self, theme_file: String, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.menu_theme.set_file(theme_file),
            InternalThemeKind::Main => self.main_theme.set_file(theme_file),
            InternalThemeKind::Game => self.game_theme.set_file(theme_file),
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn save_theme(&self, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.themes.menu.save(self.menu_theme.get_file()),
            InternalThemeKind::Main => self.themes.main.save(self.main_theme.get_file()),
            InternalThemeKind::Game => self.themes.game.save(self.game_theme.get_file()),
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn reload_theme(&mut self, kind: InternalThemeKind) {
        match kind {
            InternalThemeKind::Menu => self.themes.menu.reload::<DefaultMenu>(self.menu_theme.get_file()),
            InternalThemeKind::Main => self.themes.main.reload::<DefaultMain>(self.main_theme.get_file()),
            InternalThemeKind::Game => self.themes.game.reload(self.game_theme.get_file()),
        }
    }
}*/

#[derive(Clone, Copy)]
pub struct ThemeSelector2 {
    kind: InterfaceThemeKind,
}

impl From<InterfaceThemeKind> for ThemeSelector2 {
    fn from(kind: InterfaceThemeKind) -> Self {
        ThemeSelector2 { kind }
    }
}

macro_rules! impl_theme_selector {
    ($selector:ty, $subtype:ty, $field:ident) => {
        impl<'a> Selector<'a, GameState, $subtype> for $selector {
            fn get(&self, state: &'a GameState) -> Option<&'a $subtype> {
                match self.kind {
                    InterfaceThemeKind::Main => Some(&state.main_theme.$field),
                    InterfaceThemeKind::Menu => Some(&state.menu_theme.$field),
                }
            }

            fn get_mut(&self, state: &'a mut GameState) -> Option<&'a mut $subtype> {
                match self.kind {
                    InterfaceThemeKind::Main => Some(&mut state.main_theme.$field),
                    InterfaceThemeKind::Menu => Some(&mut state.menu_theme.$field),
                }
            }

            fn get_path_id(&self) -> rust_state::PathId {
                match self.kind {
                    InterfaceThemeKind::Main => PathId::new(vec![PathUuid(100001)]),
                    InterfaceThemeKind::Menu => PathId::new(vec![PathUuid(100002)]),
                }
            }
        }
    };
}

impl_theme_selector!(ThemeSelector2, korangar_interface::theme::ButtonTheme<GameState>, button);
impl_theme_selector!(ThemeSelector2, korangar_interface::theme::WindowTheme<GameState>, window);
impl_theme_selector!(
    ThemeSelector2,
    korangar_interface::theme::ExpandableTheme<GameState>,
    expandable
);
impl_theme_selector!(ThemeSelector2, korangar_interface::theme::LabelTheme<GameState>, label);
impl_theme_selector!(ThemeSelector2, korangar_interface::theme::ValueTheme<GameState>, value);
impl_theme_selector!(
    ThemeSelector2,
    korangar_interface::theme::CloseButtonTheme<GameState>,
    close_button
);
impl_theme_selector!(ThemeSelector2, korangar_interface::theme::SliderTheme<GameState>, slider);
impl_theme_selector!(ThemeSelector2, korangar_interface::theme::InputTheme<GameState>, input);
impl_theme_selector!(ThemeSelector2, super::theme::ProfilerTheme, profiler);
impl_theme_selector!(ThemeSelector2, super::theme::ChatTheme, chat);

impl Application for GameState {
    type Cache = WindowCache;
    type Clip = ScreenClip;
    type Color = Color;
    type CornerRadius = CornerRadius;
    type CustomEvent = UserEvent;
    type DropResource = PartialMove;
    type DropResult = Move;
    type FocusedElementSelector = GameStateFocusedElementPath;
    type FontLoader = std::rc::Rc<std::cell::RefCell<FontLoader>>;
    type FontSize = FontSize;
    type HoveredElementSelector = GameStateHoveredElementPath;
    type MouseInputMode = MouseInputMode;
    type MouseModeSelector = GameStateMouseModePath;
    type PartialSize = PartialScreenSize;
    type Position = ScreenPosition;
    type Renderer = InterfaceRenderer;
    type ScaleSelector = GameStateScalePath;
    type Scaling = Scaling;
    type Size = ScreenSize;
    type ThemeKind = InterfaceThemeKind;
    type ThemeSelector = ThemeSelector2;
}

/*impl Drop for GameState {
    fn drop(&mut self) {
        InterfaceSettingsStorage {
            menu_theme: self.menu_theme.get_file().to_owned(),
            main_theme: self.main_theme.get_file().to_owned(),
            game_theme: self.game_theme.get_file().to_owned(),
            scaling: self.scaling.get(),
        }
        .save();
    }
}*/
