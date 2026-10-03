use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

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
            login_id1: packet.login_id1,
            login_id2: packet.login_id2,
            sex: packet.sex,
        },
    })?;
    packet_handler.register(|packet: LoginFailedPacket| {
        let (reason, message) = match packet.reason {
            LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed".to_string()),
            LoginFailedReason::AlreadyLoggedIn => (
                UnifiedLoginFailedReason::AlreadyLoggedIn,
                "Someone has already logged in with this id".to_string(),
            ),
            LoginFailedReason::AlreadyOnline => (UnifiedLoginFailedReason::AlreadyOnline, "Already online".to_string()),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let (reason, message) = match packet.reason {
            LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Unregistered id".to_string()),
            LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect password".to_string()),
            LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired".to_string()),
            LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server".to_string()),
            LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team".to_string()),
            LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated".to_string()),
            LoginFailedReason2::LoginProhibitedUntil => (
                UnifiedLoginFailedReason::LoginProhibitedUntil,
                format!("You are prohibited to log in until {}.", packet.date),
            ),
            LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full".to_string()),
            LoginFailedReason2::CompanyAccountLimitReached => (
                UnifiedLoginFailedReason::CompanyAccountLimitReached,
                "Company account limit reached".to_string(),
            ),
            LoginFailedReason2::BannedByDBATeam => (UnifiedLoginFailedReason::BannedByDBATeam, "Banned by DBA team".to_string()),
            LoginFailedReason2::UnconfirmedEmail => (UnifiedLoginFailedReason::UnconfirmedEmail, "Email not confirmed".to_string()),
            LoginFailedReason2::BannedByGMTeam => (UnifiedLoginFailedReason::BannedByGMTeam, "Banned by GM team".to_string()),
            LoginFailedReason2::TemporaryBanForDatabaseWork => (
                UnifiedLoginFailedReason::TemporaryBanForDatabaseWork,
                "Working in DB".to_string(),
            ),
            LoginFailedReason2::SelfLocked => (UnifiedLoginFailedReason::SelfLocked, "Self lock".to_string()),
            LoginFailedReason2::NotPermittedGroup => (UnifiedLoginFailedReason::NotPermittedGroup, "Not Permitted Group".to_string()),
            LoginFailedReason2::AccountIdErased => (
                UnifiedLoginFailedReason::AccountIdErased,
                "This ID has been totally erased".to_string(),
            ),
            LoginFailedReason2::LoginInformationRemains => (
                UnifiedLoginFailedReason::LoginInformationRemains,
                format!("Login information remains at {}", packet.date),
            ),
            LoginFailedReason2::LockedForHackingInvestigation => (
                UnifiedLoginFailedReason::LockedForHackingInvestigation,
                "Account has been locked for a hacking investigation. Please contact the GM Team for more information".to_string(),
            ),
            LoginFailedReason2::TemporaryLockedForBugInvestigation => (
                UnifiedLoginFailedReason::TemporaryLockedForBugInvestigation,
                "This account has been temporarily prohibited from login due to a bug-related investigation".to_string(),
            ),
            LoginFailedReason2::DeletingCharacter => (
                UnifiedLoginFailedReason::DeletingCharacter,
                "This character is being deleted. Login is temporarily unavailable for the time being".to_string(),
            ),
            LoginFailedReason2::DeletingSpouseCharacter => (
                UnifiedLoginFailedReason::DeletingSpouseCharacter,
                "This character is being deleted. Login is temporarily unavailable for the time being".to_string(),
            ),
            LoginFailedReason2::UnknownError => (UnifiedLoginFailedReason::UnknownError, "Unknown error".to_string()),
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
    packet_handler.register(|packet: LoginFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            LoginFailedReason::ServerClosed => "Server closed",
            LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
            LoginFailedReason::AlreadyOnline => "Already online",
        };

        NetworkEvent::CharacterServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(
        |packet: CharacterServerLoginSuccessPacket| NetworkEvent::CharacterServerConnected {
            normal_slot_count: packet.normal_slot_count as usize,
        },
    )?;
    packet_handler.register(|packet: RequestCharacterListSuccessPacket| NetworkEvent::CharacterList {
        characters: packet.character_information,
    })?;
    packet_handler.register_noop::<CharacterListPacket>()?;
    packet_handler.register_noop::<CharacterSlotPagePacket>()?;
    packet_handler.register_noop::<CharacterBanListPacket>()?;
    packet_handler.register_noop::<LoginPincodePacket>()?;
    packet_handler.register_noop::<Packet0b18>()?;
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
        character_information: packet.character_information,
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
    // This is a bit of a workaround for the way that the inventory is
    // sent. There is a single packet to start the inventory list,
    // followed by an arbitary number of item packets, and in the
    // end a sinle packet to mark the list as complete.
    //
    // This variable provides some transient storage shared by all the inventory
    // handlers.
    let inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));

    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Broadcast,
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
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
    packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    packet_handler.register_noop::<MessageTablePacket>()?;
    packet_handler.register(|packet: EntityMessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: DisplayEmotionPacket| NetworkEvent::DisplayEmotion {
        entity_id: packet.entity_id,
        emotion: packet.emotion,
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
    packet_handler.register(|packet: ChangeMapPacket| {
        let ChangeMapPacket { map_name, position } = packet;

        let map_name = map_name.replace(".gat", "");

        NetworkEvent::ChangeMap { map_name, position }
    })?;
    packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityAppear2Packet| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: MovingEntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityDisAppearPacket| NetworkEvent::RemoveEntity {
        entity_id: packet.entity_id,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: GroundItemAppearPacket| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear2Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear3Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear4Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: ItemDisappearPacket| NetworkEvent::RemoveGroundItem {
        entity_id: packet.entity_id,
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
    packet_handler.register_noop::<UpdateAttackRangePacket>()?;
    packet_handler.register_noop::<NewMailStatusPacket>()?;
    packet_handler.register_noop::<AchievementUpdatePacket>()?;
    packet_handler.register_noop::<AchievementListPacket>()?;
    packet_handler.register_noop::<CriticalWeightUpdatePacket>()?;
    packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
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
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyStartPacket| {
            *inventory_items.borrow_mut() = Some(Vec::new());
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: RegularItemListPacket| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item_information| {
                    let RegularItemInformation {
                        index,
                        item_id,
                        item_type,
                        amount,
                        equipped_position,
                        slot,
                        hire_expiration_date,
                        flags,
                    } = item_information;

                    InventoryItem {
                        index,
                        metadata: NoMetadata,
                        item_id,
                        item_type,
                        slot,
                        hire_expiration_date,
                        details: InventoryItemDetails::Regular {
                            amount,
                            equipped_position,
                            flags,
                        },
                    }
                }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: EquippableItemListPacket| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item| {
                    let EquippableItemInformation {
                        index,
                        item_id,
                        item_type,
                        equip_position,
                        equipped_position,
                        slot,
                        hire_expiration_date,
                        bind_on_equip_type,
                        w_item_sprite_number,
                        option_count,
                        option_data,
                        refinement_level,
                        enchantment_level,
                        flags,
                    } = item;

                    InventoryItem {
                        index,
                        metadata: NoMetadata,
                        item_id,
                        item_type,
                        slot,
                        hire_expiration_date,
                        details: InventoryItemDetails::Equippable {
                            equip_position,
                            equipped_position,
                            bind_on_equip_type,
                            w_item_sprite_number,
                            option_count,
                            option_data,
                            refinement_level,
                            enchantment_level,
                            flags,
                        },
                    }
                }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyEndPacket| {
            let items = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");
            NetworkEvent::SetInventory { items }
        }
    })?;
    packet_handler.register_noop::<EquippableSwitchItemListPacket>()?;
    packet_handler.register_noop::<MapTypePacket>()?;
    packet_handler.register(|packet: UpdateSkillTreePacket| {
        let UpdateSkillTreePacket { skill_information } = packet;
        NetworkEvent::SkillTree { skill_information }
    })?;
    packet_handler.register(|packet: UpdateHotkeysPacket| NetworkEvent::SetHotkeyData {
        tab: packet.tab,
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;
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
    packet_handler.register_noop::<UpdatePartyInvitationStatePacket>()?;
    packet_handler.register_noop::<UpdateShowEquipPacket>()?;
    packet_handler.register_noop::<UpdateConfigurationPacket>()?;
    packet_handler.register_noop::<NavigateToMonsterPacket>()?;
    packet_handler.register_noop::<MarkMinimapPositionPacket>()?;
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
    packet_handler.register(|packet: DisplaySpecialEffectPacket| NetworkEvent::SpecialEffect {
        entity_id: packet.entity_id,
        effect_id: packet.effect_id,
    })?;
    packet_handler.register_noop::<DisplaySkillCooldownPacket>()?;
    packet_handler.register(|packet: DisplaySkillEffectAndDamagePacket| NetworkEvent::SkillDamage {
        skill_id: packet.skill_id,
        source_entity_id: packet.source_entity_id,
        destination_entity_id: packet.destination_entity_id,
        start_time: packet.start_time,
        source_motion: packet.source_motion,
        target_motion: packet.target_motion,
        damage: packet.damage,
        skill_level: packet.skill_level,
        hit_count: packet.hit_count,
        action: packet.action,
    })?;
    packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket| NetworkEvent::SkillEffectNoDamage {
        skill_id: packet.skill_id,
        heal_amount: packet.heal_amount,
        destination_entity_id: packet.destination_entity_id,
        source_entity_id: packet.source_entity_id,
        result: packet.result,
    })?;
    packet_handler.register_noop::<DisplayPlayerHealEffect>()?;
    packet_handler.register(|packet: StatusChangePacket| NetworkEvent::StatusChange {
        status_index: packet.index,
        entity_id: packet.entity_id,
        state: packet.state,
        duration_in_milliseconds: packet.duration_in_milliseconds,
        remaining_in_milliseconds: packet.remaining_in_milliseconds,
        values: packet.value,
    })?;
    packet_handler.register_noop::<QuestNotificationPacket1>()?;
    packet_handler.register_noop::<HuntingQuestNotificationPacket>()?;
    packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket>()?;
    packet_handler.register_noop::<QuestRemovedPacket>()?;
    packet_handler.register_noop::<QuestListPacket>()?;
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
    packet_handler.register_noop::<DisplayGainedExperiencePacket>()?;
    packet_handler.register_noop::<DisplayImagePacket>()?;
    packet_handler.register(|packet: StateChangePacket| NetworkEvent::EntityStateChange {
        entity_id: packet.entity_id,
        body_state: packet.body_state,
        health_state: packet.health_state,
        effect_state: packet.effect_state,
        is_pk_mode_on: packet.is_pk_mode_on,
    })?;

    packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
        QuestEffect::None => NetworkEvent::RemoveQuestEffect {
            entity_id: packet.entity_id,
        },
        _ => NetworkEvent::AddQuestEffect { quest_effect: packet },
    })?;
    packet_handler.register(|packet: ItemPickupPacket| {
        let ItemPickupPacket {
            index,
            quantity,
            item_id,
            is_identified,
            is_broken,
            cards,
            equip_position,
            item_type,
            result,
            hire_expiration_date,
            bind_on_equip_type,
            option_data,
            favorite,
            look,
            refinement_level,
            enchantment_level,
        } = packet;

        if result != ItemPickupResult::Success {
            return vec![NetworkEvent::ChatMessage {
                text: "Failed to pick up item.".to_string(),
                color: MessageColor::Error,
            }];
        }

        // TODO: Not sure where to store these, since the *InventoryItem packets are not
        // sending these either. We will certainly use them at some point though.
        let _ = (favorite, look);

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount: quantity,
                equipped_position: equip_position,
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, is_identified != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Equippable {
                equip_position,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type,
                w_item_sprite_number: 0,
                option_count: option_data.len() as u8,
                option_data,
                refinement_level,
                enchantment_level,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, is_identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, is_broken != 0);
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index,
            item_id,
            item_type,
            slot: cards,
            hire_expiration_date,
            details,
        };

        let is_identified = is_identified != 0;

        vec![NetworkEvent::IventoryItemAdded { item }, NetworkEvent::ItemObtained {
            item_id,
            quantity,
            is_identified,
        }]
    })?;
    packet_handler.register(|packet: RemoveItemFromInventoryPacket| NetworkEvent::InventoryItemRemoved {
        reason: packet.remove_reason,
        index: packet.index,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RequestPlayerDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: EntityId(packet.character_id.0),
        name: packet.name,
    })?;
    packet_handler.register(|packet: RequestEntityDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: packet.entity_id,
        name: packet.name,
    })?;
    packet_handler.register(|packet: UpdateEntityHealthPointsPacket| {
        let UpdateEntityHealthPointsPacket {
            entity_id,
            health_points,
            maximum_health_points,
        } = packet;

        NetworkEvent::UpdateEntityHealth {
            entity_id,
            health_points: health_points as usize,
            maximum_health_points: maximum_health_points as usize,
        }
    })?;
    packet_handler.register(|packet: RequestPlayerAttackFailedPacket| {
        let RequestPlayerAttackFailedPacket {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        } = packet;

        NetworkEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        }
    })?;
    packet_handler.register(|packet: DamagePacket1| match packet.damage_type {
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
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.destination_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: DamagePacket3| match packet.damage_type {
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
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.destination_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;
    packet_handler.register(|packet: RequestEquipItemStatusPacket| match packet.result {
        RequestEquipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: packet.equipped_position,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket| match packet.result {
        RequestUnequipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: EquipPosition::NONE,
        }),
        _ => None,
    })?;
    packet_handler.register_noop::<Packet8302>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register_noop::<ConnectionRefusedPacket>()?;
    packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
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
    packet_handler.register(|packet: UseSkillSuccessPacket| NetworkEvent::EntityStartCasting {
        source_entity_id: packet.source_entity,
        destination_entity_id: packet.destination_entity,
        position: packet.position,
        skill_id: packet.skill_id,
        element: packet.element,
        cast_time: packet.delay_time,
        disposable: packet.disposable,
        attack_motion: packet.attack_motion,
    })?;
    packet_handler.register(|packet: CancelSkillCastPacket| NetworkEvent::EntityCancelCasting {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: ToUseSkillSuccessPacket| {
        (packet.flag == 0).then_some(NetworkEvent::SkillUseRejected {
            skill_id: packet.skill_id,
            detail: packet.detail,
            item_id: packet.item_id,
            cause: packet.cause,
        })
    })?;
    packet_handler.register(|packet: NotifySkillUnitPacket| {
        let NotifySkillUnitPacket {
            entity_id,
            creator_id,
            position,
            unit_id,
            range,
            visible,
            skill_level,
            ..
        } = packet;

        NetworkEvent::AddSkillUnit {
            entity_id,
            creator_id,
            unit_id,
            position,
            range,
            visible,
            skill_level,
        }
    })?;
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        NetworkEvent::RemoveSkillUnit { entity_id }
    })?;
    packet_handler.register(|packet: NotifyGroundSkillPacket| NetworkEvent::GroundSkill {
        skill_id: packet.skill_id,
        source_entity_id: packet.entity_id,
        skill_level: packet.level,
        position: packet.position,
        start_time: packet.start_time,
    })?;
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
    packet_handler.register_noop::<PartyInvitePacket>()?;
    packet_handler.register_noop::<StatusChangeSequencePacket>()?;
    packet_handler.register_noop::<ReputationPacket>()?;
    packet_handler.register_noop::<ClanInfoPacket>()?;
    packet_handler.register_noop::<ClanOnlineCountPacket>()?;
    packet_handler.register_noop::<ChangeMapCellPacket>()?;
    packet_handler.register_noop::<OpenMarketPacket>()?;
    packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
    packet_handler.register(|packet: ShopItemListPacket| {
        let items = packet
            .items
            .into_iter()
            .map(|item| ShopItem {
                metadata: NoMetadata,
                item_id: item.item_id,
                item_type: item.item_type,
                price: item.price,
                quantity: ItemQuantity::Infinite,
                weight: 0,
                location: item.location,
            })
            .collect();

        NetworkEvent::OpenShop { items }
    })?;
    packet_handler.register(|packet: BuyShopItemsResultPacket| NetworkEvent::BuyingCompleted { result: packet.result })?;
    packet_handler.register_noop::<ParameterChangePacket>()?;
    packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;
    packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;
    packet_handler.register_noop::<RequestStatUpResponsePacket>()?;
    packet_handler.register_noop::<EquipAmmunitionPacket>()?;
    packet_handler.register_noop::<AmmunitionActionPacket>()?;
    packet_handler.register(|packet: UpdateSkillPacket| {
        let UpdateSkillPacket {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = packet;

        NetworkEvent::UpdateSkill {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable: upgradable != 0,
        }
    })?;
    packet_handler.register(|packet: RemoveSkillPacket| NetworkEvent::RemoveSkill { skill_id: packet.skill_id })?;

    Ok(())
}

#[cfg(test)]
mod skill_packet_tests {
    use ragnarok_bytes::ByteReader;
    use ragnarok_packets::handler::{HandlerResult, NoPacketCallback, PacketHandler};
    use ragnarok_packets::{ClientTick, EffectId, EntityId, ItemId, SkillId, SkillLevel, SkillUseFailureCode, TilePosition, UnitId};

    use super::register_map_server_packets;
    use crate::NetworkEvent;
    use crate::event::NetworkEventList;

    #[test]
    fn maps_cast_start_and_cancel_packets_to_lifecycle_events() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packets = [
            0x1A, 0x0B, // ZC_USESKILL_ACK
            0x04, 0x03, 0x02, 0x01, // source entity
            0x08, 0x07, 0x06, 0x05, // destination entity
            0x12, 0x11, // x
            0x14, 0x13, // y
            0x16, 0x15, // skill id
            0x1A, 0x19, 0x18, 0x17, // element
            0xE8, 0x03, 0x00, 0x00, // 1000 ms cast time
            0x1F, // disposable
            0x23, 0x22, 0x21, 0x20, // attack motion
            0xB9, 0x01, // ZC_DISPEL
            0x04, 0x03, 0x02, 0x01, // entity
        ];
        let mut reader = ByteReader::without_metadata(&packets);

        let HandlerResult::Ok(NetworkEventList(start_events)) = handler.process_one(&mut reader) else {
            panic!("cast start packet was not handled");
        };
        assert!(matches!(start_events.as_slice(), [NetworkEvent::EntityStartCasting {
            source_entity_id: EntityId(0x0102_0304),
            destination_entity_id: EntityId(0x0506_0708),
            position: TilePosition { x: 0x1112, y: 0x1314 },
            skill_id: SkillId(0x1516),
            element: 0x1718_191A,
            cast_time: 1000,
            disposable: 0x1F,
            attack_motion: 0x2021_2223,
        }]));

        let HandlerResult::Ok(NetworkEventList(cancel_events)) = handler.process_one(&mut reader) else {
            panic!("cast cancellation packet was not handled");
        };
        assert!(matches!(cancel_events.as_slice(), [NetworkEvent::EntityCancelCasting {
            entity_id: EntityId(0x0102_0304)
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn preserves_rejection_context_and_ignores_success_acknowledgements() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packets = [
            0x10, 0x01, // ZC_ACK_TOUSESKILL
            0x34, 0x12, // skill id
            0xFE, 0xFF, 0xFF, 0xFF, // signed detail: -2
            0xEF, 0xCD, 0xAB, 0x89, // item id
            0x00, // rejected
            0x47, // required item
            0x10, 0x01, // ZC_ACK_TOUSESKILL
            0x78, 0x56, // skill id
            0x00, 0x00, 0x00, 0x00, // detail
            0x00, 0x00, 0x00, 0x00, // item id
            0x01, // accepted
            0x00, // generic cause
        ];
        let mut reader = ByteReader::without_metadata(&packets);

        let HandlerResult::Ok(NetworkEventList(rejection_events)) = handler.process_one(&mut reader) else {
            panic!("skill rejection packet was not handled");
        };
        assert!(matches!(rejection_events.as_slice(), [NetworkEvent::SkillUseRejected {
            skill_id: SkillId(0x1234),
            detail: -2,
            item_id: ItemId(0x89AB_CDEF),
            cause: SkillUseFailureCode::NEED_ITEM,
        }]));

        let HandlerResult::Ok(NetworkEventList(success_events)) = handler.process_one(&mut reader) else {
            panic!("skill success acknowledgement was not handled");
        };
        assert!(success_events.is_empty());
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn maps_every_signed_skill_damage_field_to_the_event() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packet = [
            0xDE, 0x01, // ZC_NOTIFY_SKILL
            0x34, 0x12, // skill id
            0x04, 0x03, 0x02, 0x01, // source entity
            0x08, 0x07, 0x06, 0x05, // destination entity
            0x0D, 0x0C, 0x0B, 0x0A, // start tick
            0xFE, 0xFF, 0xFF, 0xFF, // source motion: -2
            0xFD, 0xFF, 0xFF, 0xFF, // target motion: -3
            0xD0, 0x8A, 0xFF, 0xFF, // damage sentinel: -30000
            0xFE, 0xFF, // skill level: -2
            0xFC, 0xFF, // hit count: -4
            0x0E, // action: 14
        ];
        let mut reader = ByteReader::without_metadata(&packet);

        let HandlerResult::Ok(NetworkEventList(events)) = handler.process_one(&mut reader) else {
            panic!("skill damage packet was not handled");
        };
        assert!(matches!(events.as_slice(), [NetworkEvent::SkillDamage {
            skill_id: SkillId(0x1234),
            source_entity_id: EntityId(0x0102_0304),
            destination_entity_id: EntityId(0x0506_0708),
            start_time: ClientTick(0x0A0B_0C0D),
            source_motion: -2,
            target_motion: -3,
            damage: -30000,
            skill_level: -2,
            hit_count: -4,
            action: 14,
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn preserves_entity_and_no_damage_skill_effect_context() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packets = [
            0xF3, 0x01, // ZC_NOTIFY_EFFECT
            0x04, 0x03, 0x02, 0x01, // entity
            0x00, 0x00, 0x00, 0x00, // EF_HIT1
            0xCB, 0x09, // ZC_SKILL_NODAMAGE
            0x34, 0x12, // skill id
            0x78, 0x56, 0x34, 0x12, // heal amount / protocol value
            0x08, 0x07, 0x06, 0x05, // destination entity
            0x04, 0x03, 0x02, 0x01, // source entity
            0x9A, // result
        ];
        let mut reader = ByteReader::without_metadata(&packets);

        let HandlerResult::Ok(NetworkEventList(special_effect_events)) = handler.process_one(&mut reader) else {
            panic!("special effect packet was not handled");
        };
        assert!(matches!(special_effect_events.as_slice(), [NetworkEvent::SpecialEffect {
            entity_id: EntityId(0x0102_0304),
            effect_id: EffectId::Hit1,
        }]));

        let HandlerResult::Ok(NetworkEventList(no_damage_events)) = handler.process_one(&mut reader) else {
            panic!("no-damage skill effect packet was not handled");
        };
        assert!(matches!(no_damage_events.as_slice(), [NetworkEvent::SkillEffectNoDamage {
            skill_id: SkillId(0x1234),
            heal_amount: 0x1234_5678,
            destination_entity_id: EntityId(0x0506_0708),
            source_entity_id: EntityId(0x0102_0304),
            result: 0x9A,
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn preserves_ground_skill_and_skill_unit_context() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packets = [
            0x17, 0x01, // ZC_NOTIFY_GROUNDSKILL
            0x34, 0x12, // skill id
            0x04, 0x03, 0x02, 0x01, // source entity
            0x06, 0x00, // skill level
            0x12, 0x11, // x
            0x14, 0x13, // y
            0x0D, 0x0C, 0x0B, 0x0A, // start tick
            0xCA, 0x09, // ZC_SKILL_ENTRY3
            0x17, 0x00, // packet length
            0x08, 0x07, 0x06, 0x05, // unit entity
            0x04, 0x03, 0x02, 0x01, // creator
            0x16, 0x15, // x
            0x18, 0x17, // y
            0xEF, 0xBE, 0xAD, 0xDE, // unknown/new unit id
            0x09, // range
            0x01, // visible
            0x07, // skill level
        ];
        let mut reader = ByteReader::without_metadata(&packets);

        let HandlerResult::Ok(NetworkEventList(ground_events)) = handler.process_one(&mut reader) else {
            panic!("ground skill packet was not handled");
        };
        assert!(matches!(ground_events.as_slice(), [NetworkEvent::GroundSkill {
            skill_id: SkillId(0x1234),
            source_entity_id: EntityId(0x0102_0304),
            skill_level: SkillLevel(6),
            position: TilePosition { x: 0x1112, y: 0x1314 },
            start_time: ClientTick(0x0A0B_0C0D),
        }]));

        let HandlerResult::Ok(NetworkEventList(unit_events)) = handler.process_one(&mut reader) else {
            panic!("skill unit packet was not handled");
        };
        assert!(matches!(unit_events.as_slice(), [NetworkEvent::AddSkillUnit {
            entity_id: EntityId(0x0506_0708),
            creator_id: EntityId(0x0102_0304),
            unit_id: UnitId(0xDEAD_BEEF),
            position: TilePosition { x: 0x1516, y: 0x1718 },
            range: 9,
            visible: 1,
            skill_level: 7,
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn preserves_complete_status_change_context() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packet = [
            0x83, 0x09, // ZC_MSG_STATE_CHANGE3
            0x34, 0x12, // status index
            0x04, 0x03, 0x02, 0x01, // entity
            0x02, // state
            0x78, 0x56, 0x34, 0x12, // duration
            0xEF, 0xCD, 0xAB, 0x09, // remaining
            0x01, 0x00, 0x00, 0x00, // value 1
            0x02, 0x00, 0x00, 0x00, // value 2
            0x03, 0x00, 0x00, 0x00, // value 3
        ];
        let mut reader = ByteReader::without_metadata(&packet);

        let HandlerResult::Ok(NetworkEventList(events)) = handler.process_one(&mut reader) else {
            panic!("status change packet was not handled");
        };
        assert!(matches!(events.as_slice(), [NetworkEvent::StatusChange {
            status_index: 0x1234,
            entity_id: EntityId(0x0102_0304),
            state: 2,
            duration_in_milliseconds: 0x1234_5678,
            remaining_in_milliseconds: 0x09AB_CDEF,
            values: [1, 2, 3],
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }

    #[test]
    fn preserves_complete_entity_state_change_context() {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        register_map_server_packets(&mut handler).unwrap();

        let packet = [
            0x29, 0x02, // ZC_STATE_CHANGE
            0x04, 0x03, 0x02, 0x01, // entity
            0x12, 0x11, // body state
            0x14, 0x13, // health state
            0x78, 0x56, 0x34, 0x12, // effect/option state
            0x01, // PK mode
        ];
        let mut reader = ByteReader::without_metadata(&packet);

        let HandlerResult::Ok(NetworkEventList(events)) = handler.process_one(&mut reader) else {
            panic!("entity state change packet was not handled");
        };
        assert!(matches!(events.as_slice(), [NetworkEvent::EntityStateChange {
            entity_id: EntityId(0x0102_0304),
            body_state: 0x1112,
            health_state: 0x1314,
            effect_state: 0x1234_5678,
            is_pk_mode_on: 1,
        }]));
        assert_eq!(reader.remaining_bytes(), []);
    }
}
