use ragnarok_packets::*;

use crate::hotkey::HotkeyState;
use crate::{
    CharacterServerLoginData, EntityData, InventoryItem, LoginServerLoginData, MessageColor, UnifiedCharacterSelectionFailedReason,
    UnifiedLoginFailedReason,
};

/// An event triggered by one of the Ragnarok Online servers.
#[derive(Debug)]
pub enum NetworkEvent {
    LoginServerConnected {
        character_servers: Vec<CharacterServerInformation>,
        login_data: LoginServerLoginData,
    },
    LoginServerConnectionFailed {
        reason: UnifiedLoginFailedReason,
        message: &'static str,
    },
    LoginServerDisconnected {
        reason: DisconnectReason,
    },
    CharacterServerConnected {
        normal_slot_count: usize,
    },
    CharacterServerConnectionFailed {
        reason: LoginFailedReason,
        message: &'static str,
    },
    CharacterServerDisconnected {
        reason: DisconnectReason,
    },
    AccountId(AccountId),
    CharacterList {
        characters: Vec<CharacterInformation>,
    },
    CharacterSelected {
        login_data: CharacterServerLoginData,
        map_name: String,
    },
    CharacterSelectionFailed {
        reason: UnifiedCharacterSelectionFailedReason,
        message: &'static str,
    },
    CharacterCreated {
        character_information: CharacterInformation,
    },
    CharacterCreationFailed {
        reason: CharacterCreationFailedReason,
        message: &'static str,
    },
    CharacterDeleted,
    CharacterDeletionFailed {
        reason: CharacterDeletionFailedReason,
        message: &'static str,
    },
    MapServerDisconnected {
        reason: DisconnectReason,
    },
    /// Add an entity to the list of entities that the client is aware of.
    AddEntity(EntityData),
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id.
    RemoveEntity(EntityId),
    /// The player is pathing to a new position.
    PlayerMove(WorldPosition, WorldPosition, ClientTick),
    /// An Entity nearby is pathing to a new position.
    EntityMove(EntityId, WorldPosition, WorldPosition, ClientTick),
    /// Player was moved to a new position on a different map or the current map
    ChangeMap(String, TilePosition),
    /// Update the client side [`tick
    /// counter`](crate::system::GameTimer::base_client_tick) to keep server and
    /// client synchronized.
    UpdateClientTick(ClientTick),
    /// New chat message for the client.
    ChatMessage {
        text: String,
        color: MessageColor,
    },
    CharacterSlotSwitched,
    CharacterSlotSwitchFailed,
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails(EntityId, String),
    UpdateEntityHealth(EntityId, usize, usize),
    DamageEffect(EntityId, usize),
    HealEffect(EntityId, usize),
    UpdateStatus(StatusType),
    OpenDialog(String, EntityId),
    AddNextButton,
    AddCloseButton,
    AddChoiceButtons(Vec<String>),
    AddQuestEffect(QuestEffectPacket),
    RemoveQuestEffect(EntityId),
    SetInventory {
        items: Vec<InventoryItem>,
    },
    AddIventoryItem(ItemIndex, ItemId, EquipPosition, EquipPosition),
    SkillTree(Vec<SkillInformation>),
    UpdateEquippedPosition {
        index: ItemIndex,
        equipped_position: EquipPosition,
    },
    ChangeJob(AccountId, u32),
    SetPlayerPosition(WorldPosition),
    LoggedOut,
    FriendRequest {
        requestee: Friend,
    },
    VisualEffect(&'static str, EntityId),
    AddSkillUnit(EntityId, UnitId, TilePosition),
    RemoveSkillUnit(EntityId),
    SetFriendList {
        friends: Vec<Friend>,
    },
    FriendAdded {
        friend: Friend,
    },
    FriendRemoved {
        account_id: AccountId,
        character_id: CharacterId,
    },
    SetHotkeyData {
        tab: HotbarTab,
        hotkeys: Vec<HotkeyState>,
    },
}

/// New-type so we can implement some `From` traits. This will help when
/// registering the packet handlers.
#[derive(Default)]
pub(crate) struct NetworkEventList(pub Vec<NetworkEvent>);

pub(crate) struct NoNetworkEvents;

impl From<NetworkEvent> for NetworkEventList {
    fn from(event: NetworkEvent) -> Self {
        Self(vec![event])
    }
}

impl From<Vec<NetworkEvent>> for NetworkEventList {
    fn from(events: Vec<NetworkEvent>) -> Self {
        Self(events)
    }
}

impl From<Option<NetworkEvent>> for NetworkEventList {
    fn from(event: Option<NetworkEvent>) -> Self {
        match event {
            Some(event) => Self(vec![event]),
            None => Self(Vec::new()),
        }
    }
}

impl From<(NetworkEvent, NetworkEvent)> for NetworkEventList {
    fn from(events: (NetworkEvent, NetworkEvent)) -> Self {
        Self(vec![events.0, events.1])
    }
}

impl From<NoNetworkEvents> for NetworkEventList {
    fn from(_: NoNetworkEvents) -> Self {
        Self(Vec::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    ClosedByClient,
    ConnectionError,
}

pub(crate) trait DisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent;
}

pub(crate) struct LoginServerDisconnectedEvent;
pub(crate) struct CharacterServerDisconnectedEvent;
pub(crate) struct MapServerDisconnectedEvent;

impl DisconnectedEvent for LoginServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::LoginServerDisconnected { reason }
    }
}

impl DisconnectedEvent for CharacterServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::CharacterServerDisconnected { reason }
    }
}

impl DisconnectedEvent for MapServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::MapServerDisconnected { reason }
    }
}
