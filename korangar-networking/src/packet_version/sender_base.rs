use ragnarok_packets::{
    AccountId, Action, AddFriendPacket, BuyOrSellOption, BuyShopItemInformation, BuyShopItemsPacket, CharacterId,
    CharacterServerLoginPacket, CharacterServerPacket, ChooseDialogOptionPacket, ClientTick, CloseDialogPacket, CloseShopPacket,
    CreateCharacterPacket, DeleteCharacterPacket, EndUseSkillPacket, EntityId, EquipPosition, FriendRequestResponse,
    FriendRequestResponsePacket, GlobalMessagePacket, HotbarSlot, HotbarTab, HotkeyData, InventoryIndex, LoginServerLoginPacket,
    LoginServerPacket, MapLoadedPacket, MapServerLoginPacket, MapServerPacket, NextDialogPacket, RemoveFriendPacket, RequestActionPacket,
    RequestCharacterListPacket, RequestDetailsPacket, RequestEquipItemPacket, RequestPlayerMovePacket, RequestServerTickPacket,
    RequestUnequipItemPacket, RequestWarpToMapPacket, RestartPacket, RestartType, SelectBuyOrSellPacket, SelectCharacterPacket,
    SellItemsPacket, SetHotkeyData2Packet, Sex, ShopId, SkillId, SkillLevel, SoldItemInformation, StartDialogPacket, StartUseSkillPacket,
    SwitchCharacterSlotPacket, TilePosition, UseSkillAtIdPacket, UseSkillOnGroundPacket, WorldPosition,
};

use crate::{CharPacketFactory, LoginPacketFactory, LoginServerLoginData, MapPacketFactory};

#[derive(Clone)]
pub struct BaseLoginPacketFactory;

#[derive(Clone)]
pub struct BaseCharPacketFactory;

#[derive(Clone)]
pub struct BaseMapPacketFactory;

impl LoginPacketFactory for BaseLoginPacketFactory {
    fn login_server_login(&self, username: impl Into<String>, password: impl Into<String>) -> impl LoginServerPacket {
        LoginServerLoginPacket::new(username.into(), password.into())
    }
}

impl CharPacketFactory for BaseCharPacketFactory {
    fn char_server_login(&self, login_data: &LoginServerLoginData) -> impl CharacterServerPacket {
        CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        )
    }

    fn request_character_list(&self) -> impl CharacterServerPacket {
        RequestCharacterListPacket::default()
    }

    fn select_character(&self, character_slot: usize) -> impl CharacterServerPacket {
        SelectCharacterPacket::new(character_slot as u8)
    }

    fn create_character(
        &self,
        slot: usize,
        name: String,
        hair_color: usize,
        hair_style: usize,
        start_job: usize,
        sex: Sex,
    ) -> impl CharacterServerPacket {
        CreateCharacterPacket::new(name, slot as u8, hair_color as u16, hair_style as u16, start_job as u16, sex)
    }

    fn delete_character(&self, character_id: CharacterId, _email: String) -> impl CharacterServerPacket {
        let email = "a@a.com".to_string();
        DeleteCharacterPacket::new(character_id, email)
    }

    fn switch_character_slot(&self, origin_slot: usize, destination_slot: usize) -> impl CharacterServerPacket {
        SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16)
    }
}

impl MapPacketFactory for BaseMapPacketFactory {
    fn map_server_login(
        &self,
        account_id: AccountId,
        character_id: CharacterId,
        login_id1: u32,
        client_tick: ClientTick,
        sex: Sex,
    ) -> impl MapServerPacket {
        MapServerLoginPacket::new(account_id, character_id, login_id1, client_tick, sex)
    }

    fn map_loaded(&self) -> impl MapServerPacket {
        MapLoadedPacket::default()
    }

    fn request_client_tick(&self, client_tick: ClientTick) -> impl MapServerPacket {
        RequestServerTickPacket::new(client_tick)
    }

    fn respawn(&self) -> impl MapServerPacket {
        RestartPacket::new(RestartType::Respawn)
    }

    fn log_out(&self) -> impl MapServerPacket {
        RestartPacket::new(RestartType::Disconnect)
    }

    fn player_move(&self, position: WorldPosition) -> impl MapServerPacket {
        RequestPlayerMovePacket::new(position)
    }

    fn warp_to_map(&self, map_name: String, position: TilePosition) -> impl MapServerPacket {
        RequestWarpToMapPacket::new(map_name, position)
    }

    fn entity_details(&self, entity_id: EntityId) -> impl MapServerPacket {
        RequestDetailsPacket::new(entity_id)
    }

    fn player_attack(&self, entity_id: EntityId) -> impl MapServerPacket {
        RequestActionPacket::new(entity_id, Action::Attack)
    }

    fn send_chat_message(&self, player_name: &str, text: &str) -> impl MapServerPacket {
        let message = format!("{} : {}", player_name, text);
        GlobalMessagePacket::new(message)
    }

    fn start_dialog(&self, npc_id: EntityId) -> impl MapServerPacket {
        StartDialogPacket::new(npc_id)
    }

    fn next_dialog(&self, npc_id: EntityId) -> impl MapServerPacket {
        NextDialogPacket::new(npc_id)
    }

    fn close_dialog(&self, npc_id: EntityId) -> impl MapServerPacket {
        CloseDialogPacket::new(npc_id)
    }

    fn choose_dialog_option(&self, npc_id: EntityId, option: i8) -> impl MapServerPacket {
        ChooseDialogOptionPacket::new(npc_id, option)
    }

    fn request_item_equip(&self, item_index: InventoryIndex, equip_position: EquipPosition) -> impl MapServerPacket {
        RequestEquipItemPacket::new(item_index, equip_position)
    }

    fn request_item_unequip(&self, item_index: InventoryIndex) -> impl MapServerPacket {
        RequestUnequipItemPacket::new(item_index)
    }

    fn cast_skill(&self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> impl MapServerPacket {
        UseSkillAtIdPacket::new(skill_level, skill_id, entity_id)
    }

    fn cast_ground_skill(&self, skill_id: SkillId, skill_level: SkillLevel, target_position: TilePosition) -> impl MapServerPacket {
        UseSkillOnGroundPacket::new(skill_level, skill_id, target_position)
    }

    fn cast_channeling_skill(&self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> impl MapServerPacket {
        StartUseSkillPacket::new(skill_id, skill_level, entity_id)
    }

    fn stop_channeling_skill(&self, skill_id: SkillId) -> impl MapServerPacket {
        EndUseSkillPacket::new(skill_id)
    }

    fn add_friend(&self, name: String) -> impl MapServerPacket {
        AddFriendPacket::new(name)
    }

    fn remove_friend(&self, account_id: AccountId, character_id: CharacterId) -> impl MapServerPacket {
        RemoveFriendPacket::new(account_id, character_id)
    }

    fn reject_friend_request(&self, account_id: AccountId, character_id: CharacterId) -> impl MapServerPacket {
        FriendRequestResponsePacket::new(account_id, character_id, FriendRequestResponse::Reject)
    }

    fn accept_friend_request(&self, account_id: AccountId, character_id: CharacterId) -> impl MapServerPacket {
        FriendRequestResponsePacket::new(account_id, character_id, FriendRequestResponse::Accept)
    }

    fn set_hotkey_data(&self, tab: HotbarTab, index: HotbarSlot, hotkey_data: HotkeyData) -> impl MapServerPacket {
        SetHotkeyData2Packet::new(tab, index, hotkey_data)
    }

    fn select_buy_or_sell(&self, shop_id: ShopId, buy_or_sell: BuyOrSellOption) -> impl MapServerPacket {
        SelectBuyOrSellPacket::new(shop_id, buy_or_sell)
    }

    fn purchase_items(&self, items: Vec<BuyShopItemInformation>) -> impl MapServerPacket {
        BuyShopItemsPacket::new(items)
    }

    fn close_shop(&self) -> impl MapServerPacket {
        CloseShopPacket::new()
    }

    fn sell_items(&self, items: Vec<SoldItemInformation>) -> impl MapServerPacket {
        SellItemsPacket { items }
    }
}
