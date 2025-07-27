use std::sync::Arc;

use korangar_interface::application::{Application, RenderLayer};
use korangar_interface::components::button::ButtonTheme;
use korangar_interface::components::collapsable::CollapsableTheme;
use korangar_interface::components::drop_down::DropDownTheme;
use korangar_interface::components::state_button::StateButtonTheme;
use korangar_interface::components::text::TextTheme;
use korangar_interface::components::text_box::TextBoxTheme;
use korangar_interface::element::StateElement;
use korangar_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipTheme;
use korangar_interface::layout::{ClipLayer, ClipLayerId, Icon, Layout};
use korangar_interface::theme::ThemePathGetter;
use korangar_interface::window::{StateWindow, WindowTheme};
use korangar_networking::{MessageColor, SellItem, ShopItem};
use ragnarok_packets::{CharacterId, CharacterInformation, CharacterServerInformation, Friend};
use rust_state::{Path, RustState, Selector};

#[cfg(feature = "debug")]
use crate::PacketHistory;
use crate::character_slots::CharacterSlots;
#[cfg(feature = "debug")]
use crate::graphics::RenderOptions;
use crate::graphics::{Color, Texture};
use crate::input::UserEvent;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::GameTheme;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::{ClientInfo, FontSize, GameFileLoader, Scaling, ServiceId, Sprite, load_client_info};
use crate::renderer::SpriteRenderer;
use crate::settings::{GraphicsSettingsCapabilities, LoginSettings};
use crate::world::{Actions, AnimationState, Entity, Map, Player, ResourceMetadata, SpriteAnimationState};
use crate::{AudioSettings, GraphicsSettings};

// TODO: Move
#[derive(Debug, Clone, RustState, StateElement)]
pub struct ChatMessage {
    pub text: String,
    // TODO: Unhide
    #[hidden_element]
    pub color: MessageColor,
}

pub(super) fn client_state() -> impl Path<ClientState, ClientState> {
    ClientState::path()
}

pub(super) fn this_player() -> impl Path<ClientState, Player, false> {
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

pub(super) fn this_entity() -> impl Path<ClientState, Entity, false> {
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

// TODO: Make all of these private and load them internally
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
    /// The scale of the ueser interface.
    interface_scale: Scaling,

    /// Special render options for debugging the client.
    #[cfg(feature = "debug")]
    render_options: RenderOptions,
    /// Currently selected thread in the profiler.
    #[cfg(feature = "debug")]
    profiler_visible_thread: crate::threads::Enum,
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
            let menu_theme = <InterfaceTheme as ThemeDefault<DefaultMenu>>::default();
            let playing_theme = <InterfaceTheme as ThemeDefault<DefaultPlaying>>::default();
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

            let login_window = LoginWindowState { selected_service };
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

            let chat_window = ChatWindowState {
                current_message: String::new(),
            };
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

        time_phase!("create player resources", {
            let friend_list = Vec::default();
            let shop_items = Vec::default();
            let sell_items = Vec::default();
            let player_name = String::new();
            let hotbar = Hotbar::default();
            let inventory = Inventory::default();
            let skill_tree = SkillTree::default();
        });

        time_phase!("create window resources", {
            let window_size = ScreenSize::default();
            let interface_scale = Scaling::new(1.0);
        });

        #[cfg(feature = "debug")]
        let debug_timer = korangar_debug::logging::Timer::new("creating debug resources");

        #[cfg(feature = "debug")]
        let render_options = RenderOptions::new();

        #[cfg(feature = "debug")]
        let profiler_visible_thread = crate::threads::Enum::Main;

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
            interface_scale,
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            profiler_visible_thread,
            #[cfg(feature = "debug")]
            packet_history,
        }
    }
}

#[derive(RustState, StateElement)]
pub struct LoginWindowState {
    pub selected_service: ServiceId,
}

#[derive(RustState, StateElement)]
pub struct ChatWindowState {
    pub current_message: String,
}

#[derive(RustState, StateElement)]
pub struct DebugButtonTheme {
    foreground_color: Color,
    hovered_background_color: Color,
}

#[derive(RustState, StateElement)]
pub struct ChatTheme {
    window_color: Color,
    text_box_background_color: Color,
}

#[derive(RustState, StateElement, StateWindow)]
#[window_title("Theme Inspector")]
pub struct InterfaceTheme {
    #[hidden_element]
    pub window: WindowTheme<ClientState>,
    #[hidden_element]
    pub text: TextTheme<ClientState>,
    #[hidden_element]
    pub button: ButtonTheme<ClientState>,
    #[hidden_element]
    pub state_button: StateButtonTheme<ClientState>,
    #[hidden_element]
    pub text_box: TextBoxTheme<ClientState>,
    #[hidden_element]
    pub collapsable: CollapsableTheme<ClientState>,
    #[hidden_element]
    pub drop_down: DropDownTheme<ClientState>,
    #[hidden_element]
    pub tooltip: TooltipTheme<ClientState>,
    pub debug_button: DebugButtonTheme,
    pub chat: ChatTheme,
}

/// Marker trait to specialize the [`ThemeDefault`] trait.
pub trait ThemeKindMarker {}

/// Default theme in the menu.
pub struct DefaultMenu;
impl ThemeKindMarker for DefaultMenu {}

/// Default theme when in game.
pub struct DefaultPlaying;
impl ThemeKindMarker for DefaultPlaying {}

pub trait ThemeDefault<T: ThemeKindMarker> {
    fn default() -> Self;
}

impl ThemeDefault<DefaultMenu> for InterfaceTheme {
    fn default() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(200, 150, 150),
                hovered_title_color: Color::rgb_u8(250, 200, 200),
                background_color: Color::monochrome_u8(30),
                gaps: 25.0,
                border: 30.0,
                corner_radius: CornerRadius::uniform(50.0),
                close_button_size: ScreenSize { width: 45.0, height: 35.0 },
                close_button_corner_radius: CornerRadius::uniform(25.0),
                minimum_width: 400.0,
                maximum_width: 600.0,
                minimum_height: 80.0,
                maximum_height: 700.0,
                title_height: 45.0,
                title_gap: 20.0,
                font_size: FontSize(20.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                anchor_color: Color::rgb_u8(130, 105, 160),
                closest_anchor_color: Color::rgb_u8(255, 175, 30),
            },
            text: TextTheme {
                color: Color::monochrome_u8(220),
                height: 15.0,
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            },
            button: ButtonTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                height: 30.0,
                corner_radius: CornerRadius::uniform(30.0),
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            state_button: StateButtonTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                checkbox_color: Color::rgb_u8(255, 100, 100),
                height: 24.0,
                corner_radius: CornerRadius::uniform(24.0),
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Left { offset: 50.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(60),
                foreground_color: Color::monochrome_u8(180),
                hovered_background_color: Color::monochrome_u8(90),
                hovered_foreground_color: Color::monochrome_u8(220),
                focused_background_color: Color::monochrome_u8(120),
                focused_foreground_color: Color::monochrome_u8(255),
                hide_icon_color: Color::rgb_u8(200, 180, 180),
                hovered_hide_icon_color: Color::rgb_u8(250, 200, 200),
                height: 30.0,
                corner_radius: CornerRadius::uniform(30.0),
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            collapsable: CollapsableTheme {
                background_color: Color::monochrome_u8(45),
                secondary_background_color: Color::monochrome_u8(30),
                foreground_color: Color::monochrome_u8(200),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_radius: CornerRadius::uniform(20.0),
                icon_color: Color::monochrome_u8(170),
                icon_size: 15.0,
                gaps: 5.0,
                border: 10.0,
                title_height: 30.0,
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Left { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(65),
                item_foreground_color: Color::monochrome_u8(180),
                item_hovered_background_color: Color::monochrome_u8(105),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_height: 30.0,
                item_corner_radius: CornerRadius::uniform(30.0),
                item_font_size: FontSize(16.0),
                item_text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                item_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                list_corner_radius: CornerRadius::uniform(30.0),
                list_background_color: Color::monochrome_u8(40),
                list_gaps: 8.0,
                list_border: 5.0,
                list_maximum_height: 700.0,
                button_background_color: Color::monochrome_u8(80),
                button_foreground_color: Color::monochrome_u8(180),
                button_hovered_background_color: Color::monochrome_u8(120),
                button_hovered_foreground_color: Color::monochrome_u8(220),
                button_height: 30.0,
                button_corner_radius: CornerRadius::uniform(30.0),
                button_font_size: FontSize(16.0),
                button_text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                button_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            tooltip: TooltipTheme {
                background_color: Color::rgba_u8(15, 15, 15, 200),
                foreground_color: Color::monochrome_u8(235),
                font_size: FontSize(16.0),
                corner_radius: CornerRadius::uniform(8.0),
                border: 8.0,
                gap: 4.0,
                mouse_offset: 20.0,
            },
            debug_button: DebugButtonTheme {
                foreground_color: Color::rgb_u8(255, 167, 89),
                hovered_background_color: Color::rgb_u8(225, 199, 115),
            },
            chat: ChatTheme {
                window_color: Color::TRANSPARENT,
                text_box_background_color: Color::TRANSPARENT,
            },
        }
    }
}

impl ThemeDefault<DefaultPlaying> for InterfaceTheme {
    fn default() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(185, 155, 155),
                hovered_title_color: Color::rgb_u8(240, 155, 155),
                background_color: Color::monochrome_u8(50),
                gaps: 8.0,
                border: 15.0,
                corner_radius: CornerRadius::uniform(14.0),
                close_button_size: ScreenSize { width: 40.0, height: 18.0 },
                close_button_corner_radius: CornerRadius::uniform(12.0),
                minimum_width: 400.0,
                maximum_width: 600.0,
                minimum_height: 80.0,
                maximum_height: 700.0,
                title_height: 25.0,
                title_gap: 2.0,
                font_size: FontSize(15.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                anchor_color: Color::rgb_u8(140, 105, 130),
                closest_anchor_color: Color::rgb_u8(255, 175, 30),
            },
            text: TextTheme {
                color: Color::monochrome_u8(220),
                height: 12.0,
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            },
            button: ButtonTheme {
                background_color: Color::monochrome_u8(120),
                foreground_color: Color::monochrome_u8(220),
                hovered_background_color: Color::monochrome_u8(150),
                hovered_foreground_color: Color::monochrome_u8(250),
                height: 20.0,
                corner_radius: CornerRadius::uniform(10.0),
                font_size: FontSize(14.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            state_button: StateButtonTheme {
                background_color: Color::monochrome_u8(120),
                foreground_color: Color::monochrome_u8(220),
                hovered_background_color: Color::monochrome_u8(150),
                hovered_foreground_color: Color::monochrome_u8(250),
                checkbox_color: Color::rgb_u8(255, 100, 100),
                height: 20.0,
                corner_radius: CornerRadius::uniform(10.0),
                font_size: FontSize(14.0),
                text_alignment: HorizontalAlignment::Left { offset: 30.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(40),
                foreground_color: Color::monochrome_u8(220),
                hovered_background_color: Color::monochrome_u8(60),
                hovered_foreground_color: Color::monochrome_u8(250),
                focused_background_color: Color::monochrome_u8(110),
                focused_foreground_color: Color::monochrome_u8(255),
                hide_icon_color: Color::monochrome_u8(180),
                hovered_hide_icon_color: Color::rgb_u8(250, 200, 200),
                height: 20.0,
                corner_radius: CornerRadius::uniform(10.0),
                font_size: FontSize(14.0),
                text_alignment: HorizontalAlignment::Left { offset: 15.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            collapsable: CollapsableTheme {
                background_color: Color::monochrome_u8(75),
                secondary_background_color: Color::monochrome_u8(55),
                foreground_color: Color::monochrome_u8(170),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_radius: CornerRadius::uniform(10.0),
                icon_color: Color::monochrome_u8(120),
                icon_size: 10.0,
                gaps: 4.0,
                border: 5.0,
                title_height: 20.0,
                font_size: FontSize(14.0),
                text_alignment: HorizontalAlignment::Left { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(80),
                item_foreground_color: Color::monochrome_u8(180),
                item_hovered_background_color: Color::monochrome_u8(120),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_height: 20.0,
                item_corner_radius: CornerRadius::uniform(10.0),
                item_font_size: FontSize(14.0),
                item_text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                item_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                list_corner_radius: CornerRadius::uniform(8.0),
                list_background_color: Color::monochrome_u8(40),
                list_gaps: 4.0,
                list_border: 4.0,
                list_maximum_height: 500.0,
                button_background_color: Color::monochrome_u8(120),
                button_foreground_color: Color::monochrome_u8(220),
                button_hovered_background_color: Color::monochrome_u8(150),
                button_hovered_foreground_color: Color::monochrome_u8(250),
                button_height: 20.0,
                button_corner_radius: CornerRadius::uniform(10.0),
                button_font_size: FontSize(14.0),
                button_text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                button_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            tooltip: TooltipTheme {
                background_color: Color::rgba_u8(15, 15, 15, 200),
                foreground_color: Color::monochrome_u8(235),
                font_size: FontSize(14.0),
                corner_radius: CornerRadius::uniform(5.0),
                border: 4.0,
                gap: 3.0,
                mouse_offset: 16.0,
            },
            debug_button: DebugButtonTheme {
                foreground_color: Color::rgb_u8(255, 167, 89),
                hovered_background_color: Color::rgb_u8(225, 199, 115),
            },
            chat: ChatTheme {
                window_color: Color::rgba_u8(0, 0, 0, 200),
                text_box_background_color: Color::rgba_u8(0, 0, 0, 150),
            },
        }
    }
}

static mut CURRENT_THEME: ClientThemeType = ClientThemeType::Game;

impl Application for ClientState {
    type Cache = WindowCache;
    type Clip = ScreenClip;
    type Color = Color;
    type CornerRadius = CornerRadius;
    type Event = UserEvent;
    type FontSize = FontSize;
    type Position = ScreenPosition;
    type Renderer = crate::renderer::InterfaceRenderer;
    type Size = ScreenSize;
    type ThemeGetter = ClientThemeGetter;
    type ThemeType = ClientThemeType;
    type WindowClass = WindowClass;

    // fn get_scaling_path() -> impl Path<Self, Scaling> {
    //     client_state().interface_scale()
    // }

    fn set_current_theme_type(theme: ClientThemeType) {
        unsafe {
            CURRENT_THEME = theme;
        }
    }
}

// impl Drop for ClientState {
//     fn drop(&mut self) {
//         InterfaceSettingsStorage {
//             fonts: self.fonts.to_owned(),
//             menu_theme: self.menu_theme.get_file().to_owned(),
//             main_theme: self.main_theme.get_file().to_owned(),
//             game_theme: self.game_theme.get_file().to_owned(),
//             scaling: self.scaling.get(),
//         }
//         .save();
//     }
// }

#[derive(Default, Debug, Clone, Copy)]
pub enum ClientThemeType {
    Menu,
    #[default]
    Game,
}

#[derive(Clone, Copy)]
pub struct ThemePath;

impl Path<ClientState, InterfaceTheme> for ThemePath {
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        match unsafe { CURRENT_THEME } {
            ClientThemeType::Menu => Some(&state.menu_theme),
            ClientThemeType::Game => Some(&state.playing_theme),
        }
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InterfaceTheme> {
        match unsafe { CURRENT_THEME } {
            ClientThemeType::Menu => Some(&mut state.menu_theme),
            ClientThemeType::Game => Some(&mut state.playing_theme),
        }
    }
}

impl Selector<ClientState, InterfaceTheme> for ThemePath {
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        self.follow(state)
    }
}

pub fn client_theme() -> impl Path<ClientState, InterfaceTheme> {
    ThemePath
}

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

    fn tooltip(self) -> impl Path<ClientState, TooltipTheme<ClientState>> {
        ThemePath.tooltip()
    }
}
