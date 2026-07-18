use std::time::Instant;

use ragnarok_packets::*;

use crate::hotkey::HotkeyState;
use crate::items::ShopItem;
use crate::{
    CharacterServerLoginData, EntityData, InventoryItem, LoginServerLoginData, MessageColor, NoMetadata,
    UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
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
    AccountId {
        account_id: AccountId,
    },
    CharacterList {
        characters: Vec<CharacterInformation>,
    },
    CharacterSelected {
        login_data: CharacterServerLoginData,
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
    /// Initial player status.
    InitialStats {
        strength_stat_points_cost: u8,
        agility_stat_points_cost: u8,
        vitality_stat_points_cost: u8,
        intelligence_stat_points_cost: u8,
        dexterity_stat_points_cost: u8,
        luck_stat_points_cost: u8,
    },
    /// Resurrect a player.
    ResurrectPlayer {
        entity_id: EntityId,
    },
    /// Make a player stand up.
    PlayerStandUp {
        entity_id: EntityId,
    },
    /// Show an emotion above an entity.
    DisplayEmotion {
        entity_id: EntityId,
        emotion: u8,
    },
    /// Add an entity to the list of entities that the client is aware of.
    AddEntity {
        entity_data: EntityData,
    },
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id.
    RemoveEntity {
        entity_id: EntityId,
        reason: DisappearanceReason,
    },
    /// Add an item to the ground.
    AddGroundItem {
        entity_id: EntityId,
        item_id: ItemId,
        is_identified: bool,
        quantity: u16,
        position: TilePosition,
        x_offset: u8,
        y_offset: u8,
    },
    /// Remove an item from the ground.
    RemoveGroundItem {
        entity_id: EntityId,
    },
    /// The player is pathing to a new position.
    PlayerMove {
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// An Entity nearby is pathing to a new position.
    EntityMove {
        entity_id: EntityId,
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// Player was moved to a new position on a different map or the current map
    ChangeMap {
        map_name: String,
        position: TilePosition,
    },
    /// Update the client side to keep server and client synchronized.
    UpdateClientTick {
        client_tick: ClientTick,
        received_at: Instant,
    },
    /// New chat message for the client.
    ChatMessage {
        text: String,
        color: MessageColor,
    },
    /// A skill use was rejected by the map server.
    SkillUseRejected {
        skill_id: SkillId,
        detail: i32,
        item_id: ItemId,
        cause: SkillUseFailureCode,
    },
    CharacterSlotSwitched,
    CharacterSlotSwitchFailed,
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails {
        entity_id: EntityId,
        name: String,
    },
    UpdateEntityHealth {
        entity_id: EntityId,
        health_points: usize,
        maximum_health_points: usize,
    },
    DamageEffect {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        /// Damage amount. [`None`] on miss, [`Some`] otherwise.
        damage_amount: Option<usize>,
        attack_duration: u32,
        is_critical: bool,
    },
    /// An entity started casting a skill.
    ///
    /// All server-provided routing data is retained so consumers can attach
    /// cast visuals to the source, destination, or ground position without
    /// having to infer it from an outgoing request.
    EntityStartCasting {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        position: TilePosition,
        skill_id: SkillId,
        element: u32,
        /// Cast duration in milliseconds.
        cast_time: u32,
        disposable: u8,
        attack_motion: u32,
    },
    /// The server cancelled an entity's active skill cast.
    EntityCancelCasting {
        entity_id: EntityId,
    },
    /// A skill hit resolved.
    ///
    /// Signed protocol fields are retained because the server uses negative
    /// values as display controls and sentinels. This stays separate from
    /// [`NetworkEvent::DamageEffect`] so skill hits cannot advance normal
    /// auto-attacks or replace sprite animations.
    SkillDamage {
        skill_id: SkillId,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        start_time: ClientTick,
        source_motion: i32,
        target_motion: i32,
        damage: i32,
        skill_level: i16,
        hit_count: i16,
        action: i8,
    },
    EntityPickUpItem {
        entity_id: EntityId,
        item_entity_id: EntityId,
    },
    /// A skill effect that resolved without a damage packet.
    ///
    /// Despite the protocol field name, `heal_amount` is also used by
    /// non-healing skills, so the complete packet context is preserved.
    SkillEffectNoDamage {
        skill_id: SkillId,
        heal_amount: u32,
        destination_entity_id: EntityId,
        source_entity_id: EntityId,
        result: u8,
    },
    /// An effect requested for a specific entity by its protocol effect id.
    SpecialEffect {
        entity_id: EntityId,
        effect_id: EffectId,
    },
    /// A non-damaging skill effect positioned on the ground.
    GroundSkill {
        skill_id: SkillId,
        source_entity_id: EntityId,
        skill_level: SkillLevel,
        position: TilePosition,
        start_time: ClientTick,
    },
    /// A status effect started, changed, or ended on an entity.
    StatusChange {
        status_index: u16,
        entity_id: EntityId,
        state: u8,
        duration_in_milliseconds: u32,
        remaining_in_milliseconds: u32,
        values: [u32; 3],
    },
    /// The complete option-state masks currently active on an entity.
    ///
    /// Effects such as Sight are represented by bits in `effect_state`, so
    /// consumers need both set and cleared masks to manage their lifecycle.
    EntityStateChange {
        entity_id: EntityId,
        body_state: u16,
        health_state: u16,
        effect_state: u32,
        is_pk_mode_on: u8,
    },
    UpdateStat {
        stat_type: StatType,
    },
    OpenDialog {
        text: String,
        npc_id: EntityId,
    },
    AddNextButton {
        npc_id: EntityId,
    },
    AddCloseButton {
        npc_id: EntityId,
    },
    AddChoiceButtons {
        choices: Vec<String>,
        npc_id: EntityId,
    },
    AddQuestEffect {
        quest_effect: QuestEffectPacket,
    },
    RemoveQuestEffect {
        entity_id: EntityId,
    },
    SetInventory {
        items: Vec<InventoryItem<NoMetadata>>,
    },
    IventoryItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    ItemObtained {
        item_id: ItemId,
        quantity: u16,
        is_identified: bool,
    },
    SkillTree {
        skill_information: Vec<SkillInformation>,
    },
    UpdateEquippedPosition {
        index: InventoryIndex,
        equipped_position: EquipPosition,
    },
    ChangeJob {
        account_id: AccountId,
        job_id: JobId,
    },
    ChangeHair {
        account_id: AccountId,
        hair_id: u32,
    },
    LoggedOut,
    FriendRequest {
        requestee: Friend,
    },
    VisualEffect {
        effect_path: &'static str,
        entity_id: EntityId,
    },
    AddSkillUnit {
        entity_id: EntityId,
        creator_id: EntityId,
        unit_id: UnitId,
        position: TilePosition,
        range: u8,
        visible: u8,
        skill_level: u8,
    },
    RemoveSkillUnit {
        entity_id: EntityId,
    },
    SetFriendList {
        friend_list: Vec<Friend>,
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
    OpenShop {
        items: Vec<ShopItem<NoMetadata>>,
    },
    AskBuyOrSell {
        shop_id: ShopId,
    },
    BuyingCompleted {
        result: BuyShopItemsResult,
    },
    SellItemList {
        items: Vec<SellItemInformation>,
    },
    SellingCompleted {
        result: SellItemsResult,
    },
    InventoryItemRemoved {
        reason: RemoveItemReason,
        index: InventoryIndex,
        amount: u16,
    },
    AttackFailed {
        target_entity_id: EntityId,
        target_position: TilePosition,
        player_position: TilePosition,
        attack_range: AttackRange,
    },
    UpdateSkill {
        skill_id: SkillId,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    },
    /// Delete a skill from the skill tree.
    RemoveSkill {
        skill_id: SkillId,
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
