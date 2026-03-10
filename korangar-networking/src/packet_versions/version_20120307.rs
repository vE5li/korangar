use std::net::IpAddr;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::{
    Broadcast2MessagePacket, BroadcastMessagePacket, BuyItemsResultPacket, BuyOrSellPacket, BuyShopItemsResult, ChangeMapPacket,
    CharacterCreationFailedPacket, CharacterCreationFailedReason, CharacterDeletionFailedPacket, CharacterDeletionFailedReason,
    CharacterDeletionSuccessPacket, CharacterInformation, CharacterListPacket_20100803, CharacterSelectionFailedPacket,
    CharacterSelectionFailedReason, CharacterSelectionSuccessPacket, CloseButtonPacket, CreateCharacterSuccessPacket,
    DamagePacket_20071113, DamageType, DialogMenuPacket, DisconnectResponsePacket, DisconnectResponseStatus, DisplayEmotionPacket,
    DisplayGainedExperiencePacket, DisplayImagePacket, DisplayPlayerHealEffect, DisplaySkillCooldownPacket,
    DisplaySkillEffectAndDamagePacket, DisplaySkillEffectNoDamagePacket_00000000, DisplaySpecialEffectPacket_20060424,
    EntityAppearPacket_20120221, EntityDisappearedPacket, EntityId, EntityMovePacket, EntityStandPacket_20120221, EntityStopMovePacket,
    EquipPosition, EquippableItemFlags, EquippableItemListPacket_20080102, FriendListPacket, FriendOnlineStatusPacket, FriendRequestPacket,
    FriendRequestResult, FriendRequestResultPacket, HotbarTab, HotkeyData, InitialStatsPacket, ItemId, ItemPickupPacket_20071002,
    ItemPickupResult, JobId, LoginBannedPacked, LoginFailedPacket2, LoginFailedReason, LoginFailedReason2, LoginPincodePacket,
    LoginServerLoginSuccessPacket, MapPropertyPacket, MapServerLoginSuccessPacket, MapServerPingPacket, MapServerUnavailablePacket,
    MapTypePacket, MovingEntityAppearPacket_20120221, NextButtonPacket, NotifyFriendRemovedPacket, NotifyGroundSkillPacket,
    NotifySkillUnitPacket_00000000, NpcDialogPacket, OverheadMessagePacket, Packet0b18, Packet8302, ParameterChangePacket,
    PlayerMovePacket, QuestEffect, QuestEffectPacket, RegularItemFlags, RegularItemListPacket_20080102, RemoveItemFromInventoryPacket,
    RequestEntityDetailsSuccessPacket_00000000, RequestEquipItemStatusPacket_20101123, RequestNpcNameSuccessPacket_00000000,
    RequestPlayerAttackFailedPacket, RequestPlayerDetailsSuccessPacket_00000000, RequestStatUpResponsePacket,
    RequestUnequipItemStatusPacket_20110824, RestartResponsePacket, RestartResponseStatus, ResurrectionPacket, SellItemsResultPacket,
    SellListPacket, ServerMessagePacket, ServerTickPacket, ShopItemListPacket_20120307, SkillUnitDisappearPacket,
    SpriteChangePacket_00000000, SpriteChangeType, StatType, StateChangePacket, StatusChangePacket_20090114,
    SwitchCharacterSlotResponsePacket, SwitchCharacterSlotResponseStatus, TilePosition, ToUseSkillSuccessPacket, UpdateAttackRangePacket,
    UpdateConfigurationPacket, UpdateEntityHealthPointsPacket_20100119, UpdateHotkeysPacket_20090617, UpdatePartyInvitationStatePacket,
    UpdateShowEquipPacket, UpdateSkillTreePacket, UpdateStatPacket, UpdateStatPacket1, UpdateStatPacket2, UpdateStatPacket3,
    UseSkillSuccessPacket, VisualEffect, VisualEffectPacket,
};

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::ItemQuantity;
use crate::{
    CharacterServerLoginData, HotkeyState, InventoryItem, InventoryItemDetails, LoginServerLoginData, MessageColor, NetworkEvent,
    NoMetadata, ShopItem, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

pub fn register_login_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginServerLoginSuccessPacket| NetworkEvent::LoginServerConnected {
        character_servers: packet.character_server_information,
        login_data: LoginServerLoginData {
            account_id: packet.account_id,
            login_id1: packet.login_id1 as u32,
            login_id2: packet.login_id2,
            sex: packet.sex,
        },
    })?;
    packet_handler.register(|packet: LoginBannedPacked| {
        let (reason, message) = match packet.reason {
            LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed"),
            LoginFailedReason::AlreadyLoggedIn => (
                UnifiedLoginFailedReason::AlreadyLoggedIn,
                "Someone has already logged in with this id",
            ),
            LoginFailedReason::AlreadyOnline => (UnifiedLoginFailedReason::AlreadyOnline, "Already online"),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let (reason, message) = match packet.reason {
            LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Unregistered id"),
            LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect password"),
            LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired"),
            LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server"),
            LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team"),
            LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated"),
            LoginFailedReason2::LoginProhibitedUntil => (UnifiedLoginFailedReason::LoginProhibitedUntil, "Login prohibited until"),
            LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full"),
            LoginFailedReason2::CompanyAccountLimitReached => (
                UnifiedLoginFailedReason::CompanyAccountLimitReached,
                "Company account limit reached",
            ),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;

    Ok(())
}

pub fn register_character_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginBannedPacked| {
        let reason = packet.reason;
        let message = match reason {
            LoginFailedReason::ServerClosed => "Server closed",
            LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
            LoginFailedReason::AlreadyOnline => "Already online",
        };

        NetworkEvent::CharacterServerConnectionFailed { reason, message }
    })?;

    packet_handler.register(|packet: CharacterListPacket_20100803| {
        NetworkEventList::from(vec![
            NetworkEvent::CharacterServerConnected {
                normal_slot_count: packet.maximum_slot_count as usize,
            },
            NetworkEvent::CharacterList {
                characters: packet
                    .character_information
                    .into_iter()
                    .map(|c| c.into())
                    .collect::<Vec<CharacterInformation>>(),
            },
        ])
    })?;
    packet_handler.register_noop::<LoginPincodePacket>()?;
    packet_handler.register(|packet: CharacterSelectionSuccessPacket| {
        let login_data = CharacterServerLoginData {
            server_ip: IpAddr::V4(packet.map_server_ip.into()),
            server_port: packet.map_server_port,
            character_id: packet.character_id,
        };

        NetworkEvent::CharacterSelected { login_data }
    })?;
    packet_handler.register(|packet: CharacterSelectionFailedPacket| {
        let (reason, message) = match packet.reason {
            CharacterSelectionFailedReason::RejectedFromServer => (
                UnifiedCharacterSelectionFailedReason::RejectedFromServer,
                "Rejected from server",
            ),
        };

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|_: MapServerUnavailablePacket| {
        let reason = UnifiedCharacterSelectionFailedReason::MapServerUnavailable;
        let message = "Map server currently unavailable";

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: CreateCharacterSuccessPacket| NetworkEvent::CharacterCreated {
        character_information: packet.character_information.into(),
    })?;
    packet_handler.register(|packet: CharacterCreationFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterCreationFailedReason::CharacterNameAlreadyUsed => "Character name is already used",
            CharacterCreationFailedReason::NotOldEnough => "You are not old enough to create a character",
            CharacterCreationFailedReason::NotAllowedToUseSlot => "You are not allowed to use this character slot",
            CharacterCreationFailedReason::CharacterCerationFailed => "Character creation failed",
        };

        NetworkEvent::CharacterCreationFailed { reason, message }
    })?;
    packet_handler.register(|_: CharacterDeletionSuccessPacket| NetworkEvent::CharacterDeleted)?;
    packet_handler.register(|packet: CharacterDeletionFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterDeletionFailedReason::NotAllowed => "You are not allowed to delete this character",
            CharacterDeletionFailedReason::CharacterNotFound => "Character was not found",
            CharacterDeletionFailedReason::NotEligible => "Character is not eligible for deletion",
        };
        NetworkEvent::CharacterDeletionFailed { reason, message }
    })?;
    packet_handler.register(|packet: SwitchCharacterSlotResponsePacket| match packet.status {
        SwitchCharacterSlotResponseStatus::Success => NetworkEvent::CharacterSlotSwitched,
        SwitchCharacterSlotResponseStatus::Error => NetworkEvent::CharacterSlotSwitchFailed,
    })?;

    Ok(())
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;

    // Map events
    packet_handler.register(|packet: ChangeMapPacket| {
        let ChangeMapPacket { map_name, position } = packet;
        let map_name = map_name.replace(".gat", "");

        NetworkEvent::ChangeMap { map_name, position }
    })?;
    packet_handler.register_noop::<MapTypePacket>()?;
    packet_handler.register_noop::<MapPropertyPacket>()?;
    // Chat
    packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    // UI
    packet_handler.register(|packet: RestartResponsePacket| match packet.result {
        RestartResponseStatus::Ok => NetworkEvent::LoggedOut,
        RestartResponseStatus::Nothing => NetworkEvent::ChatMessage {
            text: "Failed to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: DisconnectResponsePacket| match packet.result {
        DisconnectResponseStatus::Ok => NetworkEvent::LoggedOut,
        DisconnectResponseStatus::Wait10Seconds => NetworkEvent::ChatMessage {
            text: "Please wait 10 seconds before trying to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: UpdateHotkeysPacket_20090617| NetworkEvent::SetHotkeyData {
        tab: HotbarTab(0),
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;
    // Movement
    packet_handler.register(|packet: PlayerMovePacket| {
        let PlayerMovePacket {
            starting_timestamp,
            from_to,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::PlayerMove {
            origin,
            destination,
            starting_timestamp,
        }
    })?;

    packet_handler.register(|packet: EntityMovePacket| {
        let EntityMovePacket {
            entity_id,
            from_to,
            starting_timestamp,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::EntityMove {
            entity_id,
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register_noop::<EntityStopMovePacket>()?;
    // Stats
    packet_handler.register(|packet: InitialStatsPacket| {
        let InitialStatsPacket {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            ..
        } = packet;

        NetworkEvent::InitialStats {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
        }
    })?;
    packet_handler.register(|packet: UpdateSkillTreePacket| {
        let UpdateSkillTreePacket { skill_information } = packet;
        NetworkEvent::SkillTree { skill_information }
    })?;
    packet_handler.register(|packet: UpdateStatPacket| {
        let UpdateStatPacket { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket1| {
        let UpdateStatPacket1 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket2| {
        let UpdateStatPacket2 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket3| {
        let UpdateStatPacket3 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: SpriteChangePacket_00000000| match packet.sprite_type {
        SpriteChangeType::Base => Some(NetworkEvent::ChangeJob {
            account_id: packet.account_id,
            job_id: JobId(packet.value as u16),
        }),
        SpriteChangeType::Hair => Some(NetworkEvent::ChangeHair {
            account_id: packet.account_id,
            hair_id: packet.value,
        }),
        _ => None,
    })?;

    // Chat
    packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Broadcast,
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: OverheadMessagePacket| {
        // FIX: This should be a different event.
        NetworkEvent::ChatMessage {
            text: packet.message,
            color: MessageColor::Broadcast,
        }
    })?;
    // Entities
    packet_handler.register(|packet: EntityDisappearedPacket| NetworkEvent::RemoveEntity {
        entity_id: packet.entity_id,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntityAppearPacket_20120221| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityStandPacket_20120221| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: MovingEntityAppearPacket_20120221| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register_noop::<DisplayEmotionPacket>()?;
    // Tick
    packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    // Inventory — 20120307 sends items directly without start/end wrappers
    packet_handler.register(|packet: RegularItemListPacket_20080102| {
        let items = packet
            .items
            .into_iter()
            .map(|item| InventoryItem {
                metadata: NoMetadata,
                index: item.index,
                item_id: ItemId(item.item_id as u32),
                item_type: item.item_type,
                slot: item.slot.map(|s| s as u32),
                hire_expiration_date: item.hire_expiration_date,
                details: InventoryItemDetails::Regular {
                    amount: item.amount,
                    equipped_position: EquipPosition::from_bits(item.wear_state as u32).unwrap_or(EquipPosition::NONE),
                    flags: {
                        let mut flags = RegularItemFlags::empty();
                        flags.set(RegularItemFlags::IDENTIFIED, item.is_identified != 0);
                        flags
                    },
                },
            })
            .collect();

        NetworkEvent::SetInventory { items }
    })?;
    packet_handler.register(|packet: EquippableItemListPacket_20080102| {
        let items = packet
            .items
            .into_iter()
            .map(|item| InventoryItem {
                metadata: NoMetadata,
                index: item.index,
                item_id: ItemId(item.item_id as u32),
                item_type: item.item_type,
                slot: item.slot.map(|s| s as u32),
                hire_expiration_date: item.hire_expiration_date,
                details: InventoryItemDetails::Equippable {
                    equip_position: EquipPosition::from_bits(item.equip_position as u32).unwrap_or(EquipPosition::NONE),
                    equipped_position: EquipPosition::from_bits(item.equipped_position as u32).unwrap_or(EquipPosition::NONE),
                    bind_on_equip_type: item.bind_on_equip_type,
                    w_item_sprite_number: item.w_item_sprite_number,
                    option_count: 0,
                    option_data: Default::default(),
                    refinement_level: item.refining_level,
                    enchantment_level: 0,
                    flags: {
                        let mut flags = EquippableItemFlags::empty();
                        flags.set(EquippableItemFlags::IDENTIFIED, item.is_identified != 0);
                        flags.set(EquippableItemFlags::IS_BROKEN, item.is_damaged != 0);
                        flags
                    },
                },
            })
            .collect();

        NetworkEvent::SetInventory { items }
    })?;
    packet_handler.register(|packet: RemoveItemFromInventoryPacket| NetworkEvent::InventoryItemRemoved {
        reason: packet.remove_reason,
        index: packet.index,
        amount: packet.amount,
    })?;
    // Item pickup
    packet_handler.register(|packet: ItemPickupPacket_20071002| {
        if packet.result != ItemPickupResult::Success {
            return None;
        }

        let equip_position = EquipPosition::from_bits(packet.equip_position as u32).unwrap_or(EquipPosition::NONE);

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount: packet.count,
                equipped_position: equip_position,
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, packet.is_identified != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Equippable {
                equip_position,
                equipped_position: EquipPosition::NONE,
                bind_on_equip_type: packet.bind_on_equip_type,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: Default::default(),
                refinement_level: packet.refining_level,
                enchantment_level: 0,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, packet.is_identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, packet.is_broken != 0);
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index: packet.index,
            item_id: ItemId(packet.item_id as u32),
            item_type: packet.item_type,
            slot: packet.cards.map(|s| s as u32),
            hire_expiration_date: packet.hire_expiration_date,
            details,
        };

        Some(NetworkEvent::IventoryItemAdded { item })
    })?;
    // Equip/Unequip
    packet_handler.register(|packet: RequestEquipItemStatusPacket_20101123| {
        if packet.result == 0 {
            Some(NetworkEvent::UpdateEquippedPosition {
                index: packet.inventory_index,
                equipped_position: EquipPosition::from_bits(packet.equipped_position as u32).unwrap_or(EquipPosition::NONE),
            })
        } else {
            None
        }
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket_20110824| {
        if packet.result == 0 {
            Some(NetworkEvent::UpdateEquippedPosition {
                index: packet.inventory_index,
                equipped_position: EquipPosition::NONE,
            })
        } else {
            None
        }
    })?;
    // Skills
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        NetworkEvent::RemoveSkillUnit { entity_id }
    })?;
    packet_handler.register(|packet: NotifySkillUnitPacket_00000000| NetworkEvent::AddSkillUnit {
        entity_id: packet.entity_id,
        unit_id: ragnarok_packets::UnitId::Safetywall, // TODO placeholder — unit_id is u8 in old packet
        position: TilePosition {
            x: packet.x_position,
            y: packet.y_position,
        },
    })?;
    packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket_00000000| NetworkEvent::HealEffect {
        entity_id: packet.destination_entity_id,
        heal_amount: packet.heal_amount as usize,
    })?;
    packet_handler.register_noop::<DisplaySkillCooldownPacket>()?;
    packet_handler.register_noop::<DisplaySkillEffectAndDamagePacket>()?;
    // Quests
    packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
        QuestEffect::None => NetworkEvent::RemoveQuestEffect {
            entity_id: packet.entity_id,
        },
        _ => NetworkEvent::AddQuestEffect { quest_effect: packet },
    })?;
    // Damage
    packet_handler.register(|packet: DamagePacket_20071113| match packet.damage_type {
        DamageType::Damage => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.destination_entity_id,
        }),
        _ => None,
    })?;
    // Entity details
    packet_handler.register(
        |packet: RequestPlayerDetailsSuccessPacket_00000000| NetworkEvent::UpdateEntityDetails {
            entity_id: EntityId(packet.character_id.0),
            name: packet.name,
        },
    )?;
    packet_handler.register(
        |packet: RequestEntityDetailsSuccessPacket_00000000| NetworkEvent::UpdateEntityDetails {
            entity_id: packet.entity_id,
            name: packet.name,
        },
    )?;
    packet_handler.register(
        |packet: RequestNpcNameSuccessPacket_00000000| NetworkEvent::UpdateEntityDetails {
            entity_id: packet.entity_id,
            name: packet.name,
        },
    )?;
    packet_handler.register(
        |packet: UpdateEntityHealthPointsPacket_20100119| NetworkEvent::UpdateEntityHealth {
            entity_id: packet.entity_id,
            health_points: packet.health_points as usize,
            maximum_health_points: packet.maximum_health_points as usize,
        },
    )?;
    // NPC packets
    packet_handler.register(|packet: NextButtonPacket| {
        let NextButtonPacket { npc_id } = packet;

        NetworkEvent::AddNextButton { npc_id }
    })?;
    packet_handler.register(|packet: CloseButtonPacket| {
        let CloseButtonPacket { npc_id } = packet;

        NetworkEvent::AddCloseButton { npc_id }
    })?;
    packet_handler.register(|packet: DialogMenuPacket| {
        let DialogMenuPacket { npc_id, message } = packet;

        let choices = message.split(':').map(String::from).filter(|text| !text.is_empty()).collect();

        NetworkEvent::AddChoiceButtons { choices, npc_id }
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;
    packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
    // Shopping
    packet_handler.register(|packet: ShopItemListPacket_20120307| {
        let items = packet
            .items
            .into_iter()
            .map(|item| ShopItem {
                metadata: NoMetadata,
                item_id: ItemId(item.item_id as u32),
                item_type: item.item_type,
                price: item.price,
                quantity: ItemQuantity::Infinite,
                weight: 0,
                location: 0,
            })
            .collect();

        NetworkEvent::OpenShop { items }
    })?;
    packet_handler.register(|packet: BuyItemsResultPacket| {
        let result = match packet.result {
            ragnarok_packets::BuyItemResult::Successful => BuyShopItemsResult::Success,
            _ => BuyShopItemsResult::Error,
        };
        NetworkEvent::BuyingCompleted { result }
    })?;
    packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;
    packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;
    // Visual effects
    packet_handler.register_noop::<DisplaySpecialEffectPacket_20060424>()?;
    packet_handler.register(|packet: VisualEffectPacket| {
        let VisualEffectPacket { entity_id, effect } = packet;

        let effect_path = match effect {
            VisualEffect::BaseLevelUp => "angel.str",
            VisualEffect::JobLevelUp => "joblvup.str",
            VisualEffect::RefineFailure => "bs_refinefailed.str",
            VisualEffect::RefineSuccess => "bs_refinesuccess.str",
            VisualEffect::GameOver => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::PharmacySuccess => "p_success.str",
            VisualEffect::PharmacyFailure => "p_failed.str",
            VisualEffect::BaseLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::JobLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::BaseLevelUpTaekwon => "help_angel\\help_angel\\help_angel.str",
        };

        NetworkEvent::VisualEffect { effect_path, entity_id }
    })?;
    // Friends
    packet_handler.register(|packet: FriendListPacket| NetworkEvent::SetFriendList {
        friend_list: packet.friend_list,
    })?;
    packet_handler.register_noop::<FriendOnlineStatusPacket>()?;
    packet_handler.register(|packet: FriendRequestPacket| NetworkEvent::FriendRequest {
        requestee: packet.requestee,
    })?;
    packet_handler.register(|packet: FriendRequestResultPacket| {
        let text = match packet.result {
            FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
            FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
            FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
            FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
        };

        let mut events = vec![NetworkEvent::ChatMessage {
            text,
            color: MessageColor::Information,
        }];

        if matches!(packet.result, FriendRequestResult::Accepted) {
            events.push(NetworkEvent::FriendAdded { friend: packet.friend });
        }

        events
    })?;
    packet_handler.register(|packet: NotifyFriendRemovedPacket| NetworkEvent::FriendRemoved {
        account_id: packet.account_id,
        character_id: packet.character_id,
    })?;
    // Status/State
    packet_handler.register_noop::<StatusChangePacket_20090114>()?;
    packet_handler.register_noop::<StateChangePacket>()?;
    // Noops for packets rAthena sends during login and gameplay
    packet_handler.register_noop::<DisplayGainedExperiencePacket>()?;
    packet_handler.register_noop::<UpdateAttackRangePacket>()?;
    packet_handler.register(|packet: ParameterChangePacket| {
        let stat_type = match packet.variable_id {
            1 => StatType::BaseExperience(packet.value as u64),
            2 => StatType::JobExperience(packet.value as u64),
            20 => StatType::Zeny(packet.value),
            22 => StatType::NextBaseExperience(packet.value as u64),
            23 => StatType::NextJobExperience(packet.value as u64),
            _ => return None,
        };
        Some(NetworkEvent::UpdateStat { stat_type })
    })?;
    packet_handler.register_noop::<DisplayPlayerHealEffect>()?;
    packet_handler.register_noop::<DisplayImagePacket>()?;
    packet_handler.register_noop::<RequestStatUpResponsePacket>()?;
    packet_handler.register_noop::<RequestPlayerAttackFailedPacket>()?;
    packet_handler.register_noop::<UseSkillSuccessPacket>()?;
    packet_handler.register_noop::<ToUseSkillSuccessPacket>()?;
    packet_handler.register_noop::<NotifyGroundSkillPacket>()?;
    // Configuration
    packet_handler.register_noop::<UpdatePartyInvitationStatePacket>()?;
    packet_handler.register_noop::<UpdateShowEquipPacket>()?;
    packet_handler.register_noop::<UpdateConfigurationPacket>()?;
    // Unknown
    packet_handler.register_noop::<Packet8302>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    Ok(())
}
