pub mod theme;

use std::sync::Arc;

use korangar_interface::application::Application;
use korangar_interface::components::button::ButtonTheme;
use korangar_interface::components::collapsable::CollapsableTheme;
use korangar_interface::components::drop_down::DropDownTheme;
use korangar_interface::components::field::FieldTheme;
use korangar_interface::components::state_button::StateButtonTheme;
use korangar_interface::components::text::TextTheme;
use korangar_interface::components::text_box::TextBoxTheme;
use korangar_interface::element::StateElement;
use korangar_interface::layout::tooltip::TooltipTheme;
use korangar_interface::theme::ThemePathGetter;
use korangar_interface::window::{StateWindow, WindowTheme};
use korangar_networking::{MessageColor, SellItem, ShopItem};
use ragnarok_packets::{CharacterId, CharacterServerInformation, Friend};
use rust_state::{Path, RustState, Selector};
use theme::{InterfaceTheme, InterfaceThemePathExt, InterfaceThemeType};

#[cfg(feature = "debug")]
use crate::PacketHistory;
use crate::character_slots::CharacterSlots;
use crate::graphics::Color;
#[cfg(feature = "debug")]
use crate::graphics::RenderOptions;
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::interface::windows::ProfilerWindowState;
use crate::interface::windows::{ChatWindowState, DialogWindowState, FriendListWindowState, LoginWindowState, WindowCache, WindowClass};
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::{ClientInfo, FontLoader, FontSize, GameFileLoader, load_client_info};
use crate::renderer::InterfaceRenderer;
use crate::settings::{GraphicsSettingsCapabilities, LoginSettings};
use crate::state::theme::{GameTheme, ThemeDefault};
use crate::world::{Entity, Map, Player, ResourceMetadata};
use crate::{AudioSettings, GraphicsSettings};

/// A message in the in-game chat.
///
/// The message stores the color separatly rather than baking it into the
/// message so the chat window can use the correct colors when switching themes.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct ChatMessage {
    /// Raw message.
    pub text: String,
    /// Color of the message.
    pub color: MessageColor,
}

/// Internal state of the client. Everything that can be viewed or modified via
/// the user interface should be in here. State that takes care of managing OS
/// or rendering resources should be in [`Client`](super::Client).
#[derive(RustState, StateElement, StateWindow)]
#[cfg_attr(feature = "debug", window_class(WindowClass::ClientStateInspector))]
#[window_title("Client State Inspector")]
#[state_root]
pub struct ClientState {
    /// Saved settings of previous connections and credentials.
    login_settings: LoginSettings,
    /// Saved audio settings.
    audio_settings: AudioSettings,
    /// Saved graphics settings.
    graphics_settings: GraphicsSettings,
    /// Graphics capabilities used in the graphics settings window.
    graphics_settings_capabilities: GraphicsSettingsCapabilities,

    /// The interface theme for the main menu.
    menu_theme: InterfaceTheme,
    /// The interface theme when playing the game.
    playing_theme: InterfaceTheme,
    /// Theme for themeable elements that don't change between the menu
    /// and playing theme.
    game_theme: GameTheme,

    /// Loaded `sclientinfo.xml`.
    client_info: ClientInfo,
    /// Internal state of the login window.
    login_window: LoginWindowState,
    /// Internal state of the chat window.
    chat_window: ChatWindowState,
    /// Internal state of the friend list window.
    friend_list_window: FriendListWindowState,
    /// Internal state of the dialog window.
    dialog_window: DialogWindowState,

    /// The current map.
    // TODO: These are currently pub due to some code in the main render update loop. Ideally these
    // could be private after rewriting that.
    #[hidden_element]
    pub map: Option<Box<Map>>,
    /// All entities on the map.
    pub entities: Vec<Entity>,

    /// List of all received chat messages.
    // TODO: This should be a UniqueVec or something.
    chat_messages: Vec<ChatMessage>,
    /// List of all friends.
    // TODO: This should be a UniqueVec or something.
    friend_list: Vec<Friend>,
    /// List of items offered in the shop.
    // TODO: This should be a UniqueVec or something.
    // TODO: Unhide this
    #[hidden_element]
    shop_items: Vec<ShopItem<ResourceMetadata>>,
    /// List of items that should be sold.
    // TODO: This should be a UniqueVec or something.
    // TODO: Unhide this
    #[hidden_element]
    sell_items: Vec<SellItem<(ResourceMetadata, u16)>>,
    /// The name of the active character. This information is not available
    /// while playing if we don't save it here.
    player_name: String,
    /// Player configured hotbar.
    hotbar: Hotbar,
    /// Player inventory.
    inventory: Inventory,
    /// Player skill tree.
    skill_tree: SkillTree,

    /// List of all available character servers.
    // TODO: This should be a UniqueVec or something.
    character_servers: Vec<CharacterServerInformation>,
    /// List of all the slots and characters on the current character server.
    character_slots: CharacterSlots,
    /// Id of the character that is currently being deleted. The server does not
    /// send back information about which character was deleted
    /// successfully, so we store it instead.
    currently_deleting: Option<CharacterId>,
    /// Slot of the character currently being moved.
    switch_request: Option<usize>,
    /// Name of the character being created currently.
    create_character_name: String,

    /// Size of the Korangar window.
    window_size: ScreenSize,

    /// Special render options for debugging the client.
    #[cfg(feature = "debug")]
    render_options: RenderOptions,
    /// Internal state of the profiler window.
    #[cfg(feature = "debug")]
    profiler_window: ProfilerWindowState,
    /// List of packets sent and received for the packet inspector. Also
    /// contains information about which packets to display in the
    /// inspector.
    #[cfg(feature = "debug")]
    packet_history: PacketHistory,
}

impl ClientState {
    pub fn new(
        game_file_loader: &GameFileLoader,
        map: Box<Map>,
        graphics_settings: GraphicsSettings,
        graphics_settings_capabilities: GraphicsSettingsCapabilities,
        #[cfg(feature = "debug")] packet_history: PacketHistory,
    ) -> Self {
        time_phase!("load settings", {
            let mut login_settings = LoginSettings::new();
            let audio_settings = AudioSettings::new();
        });

        time_phase!("load themes", {
            let menu_theme = InterfaceTheme::default_menu();
            let playing_theme = InterfaceTheme::default_playing();
            let game_theme = GameTheme::default();
        });

        time_phase!("create login window state", {
            let client_info = load_client_info(&game_file_loader);

            let selected_service = login_settings
                .recent_service_id
                // Make sure that the recent service id is still valid.
                .and_then(|service_id| {
                    client_info
                        .services
                        .iter()
                        .any(|service| service.service_id() == service_id)
                        .then_some(service_id)
                })
                // If there is no recent service id or it was no longer valid, select the first
                // service instead.
                .or_else(|| Some(client_info.services.get(0)?.service_id()))
                .expect("There are no services available. Check your sclientinfo.yaml.");

            // Make sure that every service has a service settings entry. Without a service
            // settings entry the login window will panic.
            for service in &client_info.services {
                login_settings.service_settings.entry(service.service_id()).or_default();
            }

            let login_window = LoginWindowState::new(selected_service);
        });

        time_phase!("create window state", {
            let welcome_string = format!(
                "Welcome to ^ff8800Korangar^000000 version ^ff8800{}^000000!",
                env!("CARGO_PKG_VERSION")
            );
            let chat_messages = vec![ChatMessage {
                text: welcome_string,
                color: MessageColor::Server,
            }];

            let chat_window = ChatWindowState::default();
        });

        time_phase!("create character server resources", {
            // TODO: This could be in a single struct.
            let character_servers = Vec::new();
            // TODO: This could be in a single struct.
            let character_slots = CharacterSlots::default();
            // TODO: This could be in a single struct.
            let currently_deleting = None;
            // TODO: This could be in a single struct.
            let switch_request = None;
            let create_character_name = String::new();
        });

        time_phase!("create friend list state", {
            let friend_list = Vec::default();
            let friend_list_window = FriendListWindowState::default();
        });

        time_phase!("create player resources", {
            let dialog_window = DialogWindowState::default();

            let shop_items = Vec::default();
            let sell_items = Vec::default();
            let player_name = String::new();
            let hotbar = Hotbar::default();
            let inventory = Inventory::default();
            let skill_tree = SkillTree::default();
        });

        time_phase!("create window resources", {
            let window_size = ScreenSize::default();
        });

        #[cfg(feature = "debug")]
        let debug_timer = korangar_debug::logging::Timer::new("creating debug resources");

        #[cfg(feature = "debug")]
        let render_options = RenderOptions::new();

        #[cfg(feature = "debug")]
        let profiler_window = ProfilerWindowState::default();

        #[cfg(feature = "debug")]
        debug_timer.stop();

        ClientState {
            login_settings,
            audio_settings,
            graphics_settings,
            graphics_settings_capabilities,
            menu_theme,
            playing_theme,
            game_theme,
            client_info,
            login_window,
            chat_window,
            friend_list_window,
            dialog_window,
            map: Some(map),
            entities: Vec::new(),
            chat_messages,
            friend_list,
            shop_items,
            sell_items,
            player_name,
            hotbar,
            inventory,
            skill_tree,
            character_servers,
            character_slots,
            currently_deleting,
            switch_request,
            create_character_name,
            window_size,
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            profiler_window,
            #[cfg(feature = "debug")]
            packet_history,
        }
    }
}

/// Static used to create a path without arguments that points the the current
/// theme when creating the layout for a given window.
static mut CURRENT_THEME: InterfaceThemeType = InterfaceThemeType::Game;

/// Path resolving to the selected theme for the window.
#[derive(Clone, Copy)]
pub struct ThemePath;

impl Path<ClientState, InterfaceTheme> for ThemePath {
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        match unsafe { CURRENT_THEME } {
            InterfaceThemeType::Menu => Some(&state.menu_theme),
            InterfaceThemeType::Game => Some(&state.playing_theme),
        }
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InterfaceTheme> {
        match unsafe { CURRENT_THEME } {
            InterfaceThemeType::Menu => Some(&mut state.menu_theme),
            InterfaceThemeType::Game => Some(&mut state.playing_theme),
        }
    }
}

impl Selector<ClientState, InterfaceTheme> for ThemePath {
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        self.follow(state)
    }
}

/// Glue between our [`InterfaceTheme`] and [`korangar_interface`].
#[derive(Clone, Copy)]
pub struct ClientThemeGetter;

impl ThemePathGetter<ClientState> for ClientThemeGetter {
    fn new() -> Self {
        Self
    }

    fn window(self) -> impl Path<ClientState, WindowTheme<ClientState>> {
        ThemePath.window()
    }

    fn text(self) -> impl Path<ClientState, TextTheme<ClientState>> {
        ThemePath.text()
    }

    fn button(self) -> impl Path<ClientState, ButtonTheme<ClientState>> {
        ThemePath.button()
    }

    fn state_button(self) -> impl Path<ClientState, StateButtonTheme<ClientState>> {
        ThemePath.state_button()
    }

    fn text_box(self) -> impl Path<ClientState, TextBoxTheme<ClientState>> {
        ThemePath.text_box()
    }

    fn collapsable(self) -> impl Path<ClientState, CollapsableTheme<ClientState>> {
        ThemePath.collapsable()
    }

    fn drop_down(self) -> impl Path<ClientState, DropDownTheme<ClientState>> {
        ThemePath.drop_down()
    }

    fn field(self) -> impl Path<ClientState, FieldTheme<ClientState>> {
        ThemePath.field()
    }

    fn tooltip(self) -> impl Path<ClientState, TooltipTheme<ClientState>> {
        ThemePath.tooltip()
    }
}

impl Application for ClientState {
    type Cache = WindowCache;
    type Clip = ScreenClip;
    type Color = Color;
    type CornerRadius = CornerRadius;
    type CustomEvent = InputEvent;
    type CustomMouseMode = MouseInputMode;
    type FontSize = FontSize;
    type Position = ScreenPosition;
    type Renderer = InterfaceRenderer;
    type Size = ScreenSize;
    type TextLayouter = Arc<FontLoader>;
    type ThemeGetter = ClientThemeGetter;
    type ThemeType = InterfaceThemeType;
    type WindowClass = WindowClass;

    fn set_current_theme_type(theme: InterfaceThemeType) {
        unsafe {
            CURRENT_THEME = theme;
        }
    }
}

/// Path to the [`ClientState`] root.
pub fn client_state() -> impl Path<ClientState, ClientState> {
    ClientState::path()
}

/// Path to the current theme, same as [`korangar_interface::theme::theme`] but
/// more strongly typed so we can use the additional fields.
pub fn client_theme() -> impl Path<ClientState, InterfaceTheme> {
    ThemePath
}

/// Path to the player as [`Player`].
pub fn this_player() -> impl Path<ClientState, Player, false> {
    #[derive(Clone, Copy)]
    struct CustomPath;

    impl Selector<ClientState, Player, false> for CustomPath {
        fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a Player> {
            self.follow(state)
        }
    }

    impl Path<ClientState, Player, false> for CustomPath {
        fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a Player> {
            // TODO: Select our player better.
            match state.entities.first()? {
                Entity::Player(player) => Some(player),
                _ => unreachable!(),
            }
        }

        fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut Player> {
            // TODO: Select our player better.
            match state.entities.first_mut()? {
                Entity::Player(player) => Some(player),
                _ => unreachable!(),
            }
        }
    }

    CustomPath
}

/// Path to the player as [`Entity`].
pub fn this_entity() -> impl Path<ClientState, Entity, false> {
    #[derive(Clone, Copy)]
    struct CustomPath;

    impl Selector<ClientState, Entity, false> for CustomPath {
        fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a Entity> {
            self.follow(state)
        }
    }

    impl Path<ClientState, Entity, false> for CustomPath {
        fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a Entity> {
            // TODO: Select our player better.
            state.entities.get(0)
        }

        fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut Entity> {
            // TODO: Select our player better.
            state.entities.get_mut(0)
        }
    }

    CustomPath
}
