use cgmath::Vector2;
use korangar_interface::event::ClickAction;
use korangar_interface::ElementEvent;

use super::HotbarSlot;
use crate::interface::application::{InterfaceSettings, InternalThemeKind};
use crate::interface::resource::Move;
use crate::loaders::ServiceId;
use crate::network::{AccountId, CharacterId, CharacterServerInformation, EntityId};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Clone, Debug)]
// TODO: A lot of these are not user events, just a element events
pub enum UserEvent {
    LogIn {
        service_id: ServiceId,
        username: String,
        password: String,
    },
    SelectServer(CharacterServerInformation),
    LogOut,
    Exit,
    CameraZoom(f32),
    CameraRotate(f32),
    OpenMenuWindow,
    OpenInventoryWindow,
    OpenEquipmentWindow,
    OpenSkillTreeWindow,
    OpenGraphicsSettingsWindow,
    OpenAudioSettingsWindow,
    OpenFriendsWindow,
    ToggleShowInterface,
    SetThemeFile {
        theme_file: String,
        theme_kind: InternalThemeKind,
    },
    SaveTheme {
        theme_kind: InternalThemeKind,
    },
    ReloadTheme {
        theme_kind: InternalThemeKind,
    },
    SelectCharacter(usize),
    OpenCharacterCreationWindow(usize),
    CreateCharacter(usize, String),
    DeleteCharacter(CharacterId),
    RequestSwitchCharacterSlot(usize),
    CancelSwitchCharacterSlot,
    SwitchCharacterSlot(usize),
    RequestPlayerMove(Vector2<usize>),
    RequestPlayerInteract(EntityId),
    RequestWarpToMap(String, Vector2<usize>),
    SendMessage(String),
    NextDialog(EntityId),
    CloseDialog(EntityId),
    ChooseDialogOption(EntityId, i8),
    MoveResource(Move),
    CastSkill(HotbarSlot),
    StopSkill(HotbarSlot),
    AddFriend(String),
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
    #[cfg(feature = "debug")]
    OpenMarkerDetails(MarkerIdentifier),
    #[cfg(feature = "debug")]
    OpenRenderSettingsWindow,
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    #[cfg(feature = "debug")]
    OpenMapsWindow,
    #[cfg(feature = "debug")]
    OpenCommandsWindow,
    #[cfg(feature = "debug")]
    OpenTimeWindow,
    #[cfg(feature = "debug")]
    SetDawn,
    #[cfg(feature = "debug")]
    SetNoon,
    #[cfg(feature = "debug")]
    SetDusk,
    #[cfg(feature = "debug")]
    SetMidnight,
    #[cfg(feature = "debug")]
    OpenThemeViewerWindow,
    #[cfg(feature = "debug")]
    OpenProfilerWindow,
    #[cfg(feature = "debug")]
    OpenPacketWindow,
    #[cfg(feature = "debug")]
    ClearPacketHistory,
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

impl ElementEvent<InterfaceSettings> for UserEvent {
    fn trigger(&mut self) -> Vec<ClickAction<InterfaceSettings>> {
        vec![ClickAction::Custom(self.clone())]
    }
}
