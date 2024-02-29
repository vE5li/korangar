use procedural::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::interface::*;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThemeSelector<const KIND: ThemeKind>(pub String);

impl<const KIND: ThemeKind> ThemeSelector<KIND> {
    pub fn get_file(&self) -> &str {
        &self.0
    }

    pub fn set_file(&mut self, file: String) {
        self.0 = file;
    }
}

impl<const KIND: ThemeKind> PrototypeElement for ThemeSelector<KIND> {
    fn to_element(&self, display: String) -> ElementCell {
        let state = TrackedState::new(self.0.clone());

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
            Text::default().with_text(display).with_width(dimension!(50%)).wrap(),
            PickList::default()
                .with_options(themes)
                .with_selected(state.clone())
                .with_event(Box::new(move || {
                    vec![ClickAction::Event(UserEvent::SetThemeFile {
                        theme_file: state.get(),
                        theme_kind: KIND,
                    })]
                }))
                .with_width(dimension!(!))
                .wrap(),
        ];

        Container::new(elements).wrap()
    }
}

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct InterfaceSettings {
    #[name("Scaling")]
    pub scaling: MutableRange<f32, Resolve>,
    #[name("Main theme")]
    pub main_theme: ThemeSelector<{ ThemeKind::Main }>,
    #[name("Menu theme")]
    pub menu_theme: ThemeSelector<{ ThemeKind::Menu }>,
    #[name("Game theme")]
    pub game_theme: ThemeSelector<{ ThemeKind::Game }>,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        let scaling = MutableRange::new(1.0, 0.5, 2.5);
        let main_theme = ThemeSelector("client/themes/main.ron".to_string());
        let menu_theme = ThemeSelector("client/themes/menu.ron".to_string());
        let game_theme = ThemeSelector("client/themes/game.ron".to_string());

        Self {
            scaling,
            main_theme,
            menu_theme,
            game_theme,
        }
    }
}

impl InterfaceSettings {
    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load interface settings from {}filename{}", MAGENTA, NONE);

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading interface settings from {}filename{}", MAGENTA, NONE);

        std::fs::read_to_string("client/interface_settings.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}filename{}", MAGENTA, NONE);

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write("client/interface_settings.ron", data).expect("unable to write file");
    }
}

impl Drop for InterfaceSettings {
    fn drop(&mut self) {
        self.save();
    }
}
