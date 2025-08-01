use cgmath::Vector2;
use korangar_interface::event::{ClickAction, Event, EventQueue};
use korangar_networking::ShopItem;
use ragnarok_packets::{
    AccountId, BuyOrSellOption, CharacterId, CharacterServerInformation, EntityId, HotbarSlot, ShopId, SoldItemInformation, TilePosition,
};
use rust_state::Context;

use crate::interface::resource::Move;
use crate::loaders::ServiceId;
use crate::state::ClientState;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

// TODO: A lot of these are not user events, just a element events.
//
// TODO: Some of these don't need a special event anymore and can just modify
// the state directly.
#[derive(Clone, Debug)]
pub enum UserEvent {
    LogIn {
        service_id: ServiceId,
        username: String,
        password: String,
    },
    SelectServer {
        character_server_information: CharacterServerInformation,
    },
    Respawn,
    LogOut,
    Exit,
    ZoomCamera(f32),
    RotateCamera(f32),
    ResetCameraRotation,
    OpenMenuWindow,
    OpenInventoryWindow,
    OpenEquipmentWindow,
    OpenSkillTreeWindow,
    OpenGraphicsSettingsWindow,
    OpenAudioSettingsWindow,
    OpenFriendListWindow,
    ToggleShowInterface,
    // SetThemeFile {
    //     theme_file: String,
    //     theme_kind: InternalThemeKind,
    // },
    // SaveTheme {
    //     theme_kind: InternalThemeKind,
    // },
    // ReloadTheme {
    //     theme_kind: InternalThemeKind,
    // },
    SelectCharacter {
        slot: usize,
    },
    OpenCharacterCreationWindow {
        slot: usize,
    },
    CreateCharacter {
        slot: usize,
        name: String,
    },
    DeleteCharacter {
        character_id: CharacterId,
    },
    SwitchCharacterSlot {
        origin_slot: usize,
        destination_slot: usize,
    },
    RequestPlayerMove(Vector2<usize>),
    RequestPlayerInteract(EntityId),
    RequestWarpToMap(String, TilePosition),
    SendMessage(String),
    NextDialog(EntityId),
    CloseDialog(EntityId),
    ChooseDialogOption(EntityId, i8),
    MoveResource(Move),
    CastSkill(HotbarSlot),
    StopSkill(HotbarSlot),
    AddFriend {
        character_name: String,
    },
    RemoveFriend {
        account_id: AccountId,
        character_id: CharacterId,
    },
    RejectFriendRequest {
        account_id: AccountId,
        character_id: CharacterId,
    },
    AcceptFriendRequest {
        account_id: AccountId,
        character_id: CharacterId,
    },
    BuyItems {
        items: Vec<ShopItem<u32>>,
    },
    CloseShop,
    BuyOrSell {
        shop_id: ShopId,
        buy_or_sell: BuyOrSellOption,
    },
    SellItems {
        items: Vec<SoldItemInformation>,
    },
    #[cfg(feature = "debug")]
    OpenMarkerDetails(MarkerIdentifier),
    #[cfg(feature = "debug")]
    OpenRenderOptionsWindow,
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    #[cfg(feature = "debug")]
    OpenClientStateInspectorWindow,
    #[cfg(feature = "debug")]
    OpenMapsWindow,
    #[cfg(feature = "debug")]
    OpenCommandsWindow,
    #[cfg(feature = "debug")]
    OpenTimeWindow,
    // TODO: Unify Set* events into one that takes a specific time
    #[cfg(feature = "debug")]
    SetDawn,
    #[cfg(feature = "debug")]
    SetNoon,
    #[cfg(feature = "debug")]
    SetDusk,
    #[cfg(feature = "debug")]
    SetMidnight,
    #[cfg(feature = "debug")]
    OpenThemeInspectorWindow,
    #[cfg(feature = "debug")]
    OpenProfilerWindow,
    #[cfg(feature = "debug")]
    OpenPacketInspectorWindow,
    #[cfg(feature = "debug")]
    CameraLookAround(Vector2<f32>),
    #[cfg(feature = "debug")]
    CameraMoveForward,
    #[cfg(feature = "debug")]
    CameraMoveBackward,
    #[cfg(feature = "debug")]
    CameraMoveLeft,
    #[cfg(feature = "debug")]
    CameraMoveRight,
    #[cfg(feature = "debug")]
    CameraMoveUp,
    #[cfg(feature = "debug")]
    CameraAccelerate,
    #[cfg(feature = "debug")]
    CameraDecelerate,
}

impl From<UserEvent> for Event<ClientState> {
    fn from(custom_event: UserEvent) -> Self {
        Event::Application { custom_event }
    }
}

impl ClickAction<ClientState> for UserEvent {
    fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.clone());
    }
}
