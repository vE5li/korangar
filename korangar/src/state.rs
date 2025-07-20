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
use korangar_interface::layout::{ClipLayer, ClipLayerId, Layout};
use korangar_interface::theme::ThemePathGetter;
use korangar_interface::window::{StateWindow, WindowTheme};
use korangar_networking::{SellItem, ShopItem};
use ragnarok_packets::{CharacterId, CharacterInformation, CharacterServerInformation, Friend};
use rust_state::{Path, RustState, Selector};

use crate::character_slots::CharacterSlots;
#[cfg(feature = "debug")]
use crate::graphics::RenderSettings;
use crate::graphics::{Color, Texture};
use crate::input::UserEvent;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::theme::GameTheme;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::{ClientInfo, FontSize, Scaling, ServiceId, Sprite};
use crate::renderer::SpriteRenderer;
use crate::settings::{GraphicsSettingsCapabilities, LoginSettings};
use crate::world::{Actions, AnimationState, Entity, Map, Player, ResourceMetadata, SpriteAnimationState};
use crate::{AudioSettings, ChatMessage, GraphicsSettings, PacketState};

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
#[cfg_attr(feature = "debug", window_class(WindowClass::ClientState))]
#[state_root]
pub struct ClientState {
    pub menu_theme: ClientTheme,
    pub game_theme: ClientTheme,
    pub game_theme_2: GameTheme,
    pub interface_scale: Scaling,

    pub client_info: ClientInfo,
    pub login_window: LoginWindowState,
    pub login_settings: LoginSettings,
    // TODO: This should be a UniqueVec or something.
    pub character_servers: Vec<CharacterServerInformation>,

    // TODO: This should be a UniqueVec or something.
    // TODO: Unhide this element
    #[hidden_element]
    pub chat_messages: Vec<ChatMessage>,

    // TODO: This should be a UniqueVec or something.
    pub friend_list: Vec<Friend>,
    // saved_login_data: Option<LoginServerLoginData>,
    // saved_character_server: Option<CharacterServerInformation>,
    pub character_slots: CharacterSlots,
    // pub saved_characters: Vec<CharacterInformation>,
    // TODO: This should be a UniqueVec or something.
    #[hidden_element]
    pub shop_items: Vec<ShopItem<ResourceMetadata>>,
    // TODO: This should be a UniqueVec or something.
    #[hidden_element]
    pub sell_items: Vec<SellItem<(ResourceMetadata, u16)>>,
    pub currently_deleting: Option<CharacterId>,
    pub player_name: String,
    pub switch_request: Option<usize>,

    #[hidden_element]
    pub map: Option<Box<Map>>,
    pub entities: Vec<Entity>,

    #[hidden_element]
    pub hotbar: Hotbar,
    #[hidden_element]
    pub player_inventory: Inventory,
    #[hidden_element]
    pub player_skill_tree: SkillTree,

    pub window_size: ScreenSize,
    pub audio_settings: AudioSettings,
    pub graphics_settings: GraphicsSettings,
    pub graphics_settings_capabilities: GraphicsSettingsCapabilities,
    #[cfg(feature = "debug")]
    pub render_settings: RenderSettings,
    #[cfg(feature = "debug")]
    pub profiler_visible_thread: crate::threads::Enum,
    #[cfg(feature = "debug")]
    pub packet_state: PacketState,

    pub create_character_name: String,
}

#[derive(RustState, StateElement)]
pub struct LoginWindowState {
    pub username: String,
    pub password: String,
    pub remember_username: bool,
    pub remember_password: bool,
    pub selected_service: ServiceId,
}

#[derive(RustState, StateElement)]
pub struct DebugButtonTheme {
    foreground_color: Color,
}

#[derive(RustState, StateElement, StateWindow)]
pub struct ClientTheme {
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
}

/// Marker trait to specialize the [`ThemeDefault`] trait.
pub trait ThemeKindMarker {}

/// Default theme in the menu.
pub struct DefaultMenu;
impl ThemeKindMarker for DefaultMenu {}

/// Default theme when in game.
pub struct DefaultGame;
impl ThemeKindMarker for DefaultGame {}

pub trait ThemeDefault<T: ThemeKindMarker> {
    fn default() -> Self;
}

impl ThemeDefault<DefaultMenu> for ClientTheme {
    fn default() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(200, 150, 150),
                hovered_title_color: Color::rgb_u8(250, 200, 200),
                background_color: Color::monochrome_u8(30),
                gaps: 15.0,
                border: 20.0,
                corner_radius: CornerRadius::uniform(20.0),
                minimum_width: 400.0,
                maximum_width: 600.0,
                minimum_height: 80.0,
                maximum_height: 700.0,
                title_height: 45.0,
                font_size: FontSize(20.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                anchor_color: Color::rgb_u8(130, 105, 160),
                closest_anchor_color: Color::rgb_u8(255, 175, 30),
            },
            text: TextTheme {
                color: Color::monochrome_u8(220),
                height: 15.0,
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            },
            button: ButtonTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                height: 30.0,
                corner_radius: CornerRadius::uniform(20.0),
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
                height: 30.0,
                corner_radius: CornerRadius::uniform(20.0),
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                height: 30.0,
                corner_radius: CornerRadius::uniform(20.0),
                font_size: FontSize(20.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            collapsable: CollapsableTheme {
                background_color: Color::monochrome_u8(45),
                secondary_background_color: Color::monochrome_u8(30),
                foreground_color: Color::monochrome_u8(200),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_radius: CornerRadius::uniform(20.0),
                gaps: 5.0,
                border: 10.0,
                title_height: 30.0,
                font_size: FontSize(16.0),
                text_alignment: HorizontalAlignment::Left { offset: 20.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(65),
                item_foreground_color: Color::monochrome_u8(180),
                item_hovered_background_color: Color::monochrome_u8(105),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_height: 30.0,
                item_corner_radius: CornerRadius::uniform(20.0),
                item_font_size: FontSize(16.0),
                item_text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                item_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                list_corner_radius: CornerRadius::uniform(20.0),
                list_background_color: Color::monochrome_u8(40),
                list_gaps: 8.0,
                list_border: 5.0,
                list_maximum_height: 700.0,
                button_background_color: Color::monochrome_u8(80),
                button_foreground_color: Color::monochrome_u8(180),
                button_hovered_background_color: Color::monochrome_u8(120),
                button_hovered_foreground_color: Color::monochrome_u8(220),
                button_height: 30.0,
                button_corner_radius: CornerRadius::uniform(20.0),
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
                foreground_color: Color::rgb_u8(255, 100, 255),
            },
        }
    }
}

impl ThemeDefault<DefaultGame> for ClientTheme {
    fn default() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(200, 150, 150),
                hovered_title_color: Color::rgb_u8(250, 200, 200),
                background_color: Color::monochrome_u8(50),
                gaps: 8.0,
                border: 15.0,
                corner_radius: CornerRadius::uniform(14.0),
                minimum_width: 400.0,
                maximum_width: 600.0,
                minimum_height: 80.0,
                maximum_height: 700.0,
                title_height: 25.0,
                font_size: FontSize(14.0),
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
                font_size: FontSize(16.0),
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
                font_size: FontSize(15.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(120),
                foreground_color: Color::monochrome_u8(220),
                hovered_background_color: Color::monochrome_u8(150),
                hovered_foreground_color: Color::monochrome_u8(250),
                height: 20.0,
                corner_radius: CornerRadius::uniform(10.0),
                font_size: FontSize(15.0),
                text_alignment: HorizontalAlignment::Center { offset: 0.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            collapsable: CollapsableTheme {
                background_color: Color::monochrome_u8(75),
                secondary_background_color: Color::monochrome_u8(55),
                foreground_color: Color::monochrome_u8(180),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_radius: CornerRadius::uniform(10.0),
                gaps: 3.0,
                border: 5.0,
                title_height: 20.0,
                font_size: FontSize(14.0),
                text_alignment: HorizontalAlignment::Left { offset: 15.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(80),
                item_foreground_color: Color::monochrome_u8(180),
                item_hovered_background_color: Color::monochrome_u8(120),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_height: 20.0,
                item_corner_radius: CornerRadius::uniform(10.0),
                item_font_size: FontSize(16.0),
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
                button_font_size: FontSize(16.0),
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
                foreground_color: Color::rgb_u8(255, 100, 255),
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

impl Path<ClientState, ClientTheme> for ThemePath {
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a ClientTheme> {
        match unsafe { CURRENT_THEME } {
            ClientThemeType::Menu => Some(&state.menu_theme),
            ClientThemeType::Game => Some(&state.game_theme),
        }
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut ClientTheme> {
        match unsafe { CURRENT_THEME } {
            ClientThemeType::Menu => Some(&mut state.menu_theme),
            ClientThemeType::Game => Some(&mut state.game_theme),
        }
    }
}

impl Selector<ClientState, ClientTheme> for ThemePath {
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a ClientTheme> {
        self.follow(state)
    }
}

pub fn client_theme() -> impl Path<ClientState, ClientTheme> {
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

struct TextureInstruction {
    texture: Arc<Texture>,
    clip_layer: ClipLayerId,
    area: Area,
    color: Color,
    smooth: bool,
}

struct SpriteInstruction<'a> {
    actions: &'a Actions,
    sprite: &'a Sprite,
    animation_state: &'a SpriteAnimationState,
    clip_layer: ClipLayerId,
    area: Area,
    color: Color,
    smooth: bool,
}

pub enum CustomInstruction<'a> {
    Texture(TextureInstruction),
    Sprite(SpriteInstruction<'a>),
}

impl RenderLayer<ClientState> for crate::renderer::InterfaceRenderer {
    type CustomInstruction<'a> = CustomInstruction<'a>;

    fn render_rectangle(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, corner_radius: CornerRadius, color: Color) {
        self.render_rectangle(position, size, clip, corner_radius, color);
    }

    fn render_checkbox(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, state: bool) {
        self.render_checkbox(position, size, clip, color, state);
    }

    fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        self.get_text_dimensions(text, font_size, available_width)
    }

    fn render_text(&self, text: &str, position: ScreenPosition, clip: ScreenClip, color: Color, font_size: FontSize) {
        self.render_text(text, position, clip, color, font_size);
    }

    fn render_expand_arrow(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, expanded: bool) {
        todo!()
    }

    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clip_layers: &[ClipLayer<ClientState>]) {
        match instruction {
            CustomInstruction::Sprite(SpriteInstruction {
                actions,
                sprite,
                animation_state,
                clip_layer,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition {
                    left: area.x + area.width / 2.0,
                    top: area.y + area.height / 2.0,
                };
                let screen_clip = clip_layers[clip_layer.0].get();

                actions.render_sprite(self, sprite, animation_state, position, 0, color, 1.0);
            }
            CustomInstruction::Texture(TextureInstruction {
                texture,
                clip_layer,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition { left: area.x, top: area.y };
                let size = ScreenSize {
                    width: area.width,
                    height: area.height,
                };
                let screen_clip = clip_layers[clip_layer.0].get();

                self.render_sprite(texture, position, size, screen_clip, color, smooth);
            }
        }
    }
}

pub trait LayoutExt<'a> {
    fn add_texture(&mut self, texture: Arc<Texture>, area: Area, color: Color, smooth: bool);

    fn add_sprite(
        &mut self,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        area: Area,
        color: Color,
        smooth: bool,
    );
}

impl<'a> LayoutExt<'a> for Layout<'a, ClientState> {
    fn add_texture(&mut self, texture: Arc<Texture>, area: Area, color: Color, smooth: bool) {
        let clip_layer = self.get_active_clip_layer();

        self.add_custom_instruction(CustomInstruction::Texture(TextureInstruction {
            texture,
            clip_layer,
            area,
            color,
            smooth,
        }));
    }

    fn add_sprite(
        &mut self,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        area: Area,
        color: Color,
        smooth: bool,
    ) {
        let clip_layer = self.get_active_clip_layer();

        self.add_custom_instruction(CustomInstruction::Sprite(SpriteInstruction {
            actions,
            sprite,
            animation_state,
            clip_layer,
            area,
            color,
            smooth,
        }));
    }
}
