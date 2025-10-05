use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path, RustState};

use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::{InterfaceTheme, InterfaceThemeType, WorldTheme};

const MAXIMUM_NAME_LENGTH: usize = 40;

#[derive(Default, RustState, StateElement)]
pub struct ThemeInspectorWindowState {
    menu_theme_name: String,
    in_game_theme_name: String,
    world_theme_name: String,
}

pub struct ThemeInspectorWindow<A, B, C, D> {
    window_state_path: A,
    menu_theme_path: B,
    in_game_theme_path: C,
    world_theme_path: D,
}

impl<A, B, C, D> ThemeInspectorWindow<A, B, C, D> {
    pub fn new(window_state_path: A, menu_theme_path: B, in_game_theme_path: C, world_theme_path: D) -> Self {
        Self {
            window_state_path,
            menu_theme_path,
            in_game_theme_path,
            world_theme_path,
        }
    }
}

impl<A, B, C, D> CustomWindow<ClientState> for ThemeInspectorWindow<A, B, C, D>
where
    A: Path<ClientState, ThemeInspectorWindowState>,
    B: Path<ClientState, InterfaceTheme>,
    C: Path<ClientState, InterfaceTheme>,
    D: Path<ClientState, WorldTheme>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ThemeInspector)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct DummyTextBox;

        window! {
            title: "Theme inspector",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: scroll_view! {
                children: (
                    collapsible! {
                        text: "Controls",
                        children: (
                            collapsible! {
                                text: "Menu theme",
                                children: (
                                    text_box! {
                                        ghost_text: "Menu theme name",
                                        state: self.window_state_path.menu_theme_name(),
                                        input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.menu_theme_name(), Event::Unfocus),
                                        focus_id: DummyTextBox,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "Load",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name = state.get(&self.window_state_path.menu_theme_name()).clone();

                                                    state.update_value_with(self.menu_theme_path, move |theme| {
                                                        *theme = InterfaceTheme::load(InterfaceThemeType::Menu, &name);
                                                    });
                                                },
                                            },
                                            button! {
                                                text: "Save",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name_path = self.window_state_path.menu_theme_name();
                                                    let theme_name = state.get(&name_path);

                                                    state.get(&self.menu_theme_path).save(InterfaceThemeType::Menu, theme_name);
                                                },
                                            },
                                        ),
                                    },
                                ),
                            },
                            collapsible! {
                                text: "In-game theme",
                                children: (
                                    text_box! {
                                        ghost_text: "In-game theme name",
                                        state: self.window_state_path.in_game_theme_name(),
                                        input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.in_game_theme_name(), Event::Unfocus),
                                        focus_id: DummyTextBox,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "Load",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name = state.get(&self.window_state_path.in_game_theme_name()).clone();

                                                    state.update_value_with(self.in_game_theme_path, move |theme| {
                                                        *theme = InterfaceTheme::load(InterfaceThemeType::InGame, &name);
                                                    });
                                                },
                                            },
                                            button! {
                                                text: "Save",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name_path = self.window_state_path.in_game_theme_name();
                                                    let theme_name = state.get(&name_path);

                                                    state.get(&self.in_game_theme_path).save(InterfaceThemeType::InGame, theme_name);
                                                },
                                            },
                                        ),
                                    },
                                ),
                            },
                            collapsible! {
                                text: "World theme",
                                children: (
                                    text_box! {
                                        ghost_text: "World theme name",
                                        state: self.window_state_path.world_theme_name(),
                                        input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.world_theme_name(), Event::Unfocus),
                                        focus_id: DummyTextBox,
                                    },
                                    split! {
                                        gaps: theme().window().gaps(),
                                        children: (
                                            button! {
                                                text: "Load",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name = state.get(&self.window_state_path.world_theme_name()).clone();

                                                    state.update_value_with(self.world_theme_path, move |theme| {
                                                        *theme = WorldTheme::load(&name);
                                                    });
                                                },
                                            },
                                            button! {
                                                text: "Save",
                                                event: move |state: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                                    let name_path = self.window_state_path.world_theme_name();
                                                    let theme_name = state.get(&name_path);

                                                    state.get(&self.world_theme_path).save(theme_name);
                                                },
                                            },
                                        ),
                                    },
                                ),
                            },
                        ),
                    },
                    InterfaceTheme::to_element(self.menu_theme_path, "Menu theme".to_owned()),
                    InterfaceTheme::to_element(self.in_game_theme_path, "In-game theme".to_owned()),
                    WorldTheme::to_element(self.world_theme_path, "World theme".to_owned())
                ),
            },
        }
    }
}
