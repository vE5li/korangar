use std::cell::RefCell;
use std::rc::Rc;

use ragnarok_packets::handler::{PacketCallback, PacketHandler};
use ragnarok_packets::{
    AccountId, BuyOrSellOption, BuyShopItemInformation, CharacterId, CharacterServerPacket, ClientTick, EmptyCharServerPacket,
    EmptyLoginServerPacket, EmptyMapServerPacket, EntityId, EquipPosition, HotbarSlot, HotbarTab, HotkeyData, InventoryIndex,
    LoginServerPacket, MapServerPacket, Sex, ShopId, SkillId, SkillLevel, SoldItemInformation, TilePosition, WorldPosition,
};

use crate::event::NetworkEventList;
use crate::{InventoryItem, LoginServerLoginData, NoMetadata};

pub mod receiver_base;
pub mod sender_base;

pub struct MapPacketHandlerState {
    pub inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>>,
}

pub trait LoginPacketHandlerRegister: Send + Copy + 'static {
    fn register_version_specific_handler<Output, Meta, Callback>(&self, _packet_handler: &mut PacketHandler<Output, Meta, Callback>)
    where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: PacketCallback + Send,
    {
        // noop
    }
}
pub trait CharPacketHandlerRegister: Send + Copy + 'static {
    fn register_version_specific_handler<Output, Meta, Callback>(&self, _packet_handler: &mut PacketHandler<Output, Meta, Callback>)
    where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: PacketCallback + Send,
    {
        // noop
    }
}
pub trait MapPacketHandlerRegister: Send + Copy + 'static {
    fn register_version_specific_handler<Output, Meta, Callback>(
        &self,
        _packet_handler: &mut PacketHandler<Output, Meta, Callback>,
        _state: &MapPacketHandlerState,
    ) where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: PacketCallback + Send,
    {
        // noop
    }
}

pub trait LoginPacketFactory: Clone + Send + 'static {
    fn login_server_login(&self, _username: impl Into<String>, _password: impl Into<String>) -> impl LoginServerPacket {
        EmptyLoginServerPacket {}
    }
}

pub trait CharPacketFactory: Clone + Send + 'static {
    fn char_server_login(&self, _login_data: &LoginServerLoginData) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }
    fn request_character_list(&self) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }

    fn select_character(&self, _character_slot: usize) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }

    fn create_character(
        &self,
        _slot: usize,
        _name: String,
        _hair_color: usize,
        _hair_style: usize,
        _start_job: usize,
        _sex: Sex,
    ) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }

    fn delete_character(&self, _character_id: CharacterId, _email: String) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }

    fn switch_character_slot(&self, _origin_slot: usize, _destination_slot: usize) -> impl CharacterServerPacket {
        EmptyCharServerPacket {}
    }
}

pub trait MapPacketFactory: Clone + Send + 'static {
    fn map_server_login(
        &self,
        _account_id: AccountId,
        _character_id: CharacterId,
        _login_id1: u32,
        _client_tick: ClientTick,
        _sex: Sex,
    ) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }
    fn map_loaded(&self) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }
    fn request_client_tick(&self, _client_tick: ClientTick) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }
    fn respawn(&self) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn log_out(&self) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn player_move(&self, _position: WorldPosition) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn warp_to_map(&self, _map_name: String, _position: TilePosition) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn entity_details(&self, _entity_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn player_attack(&self, _entity_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn send_chat_message(&self, _player_name: &str, _text: &str) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn start_dialog(&self, _npc_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn next_dialog(&self, _npc_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn close_dialog(&self, _npc_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn choose_dialog_option(&self, _npc_id: EntityId, _option: i8) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn request_item_equip(&self, _item_index: InventoryIndex, _equip_position: EquipPosition) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn request_item_unequip(&self, _item_index: InventoryIndex) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn cast_skill(&self, _skill_id: SkillId, _skill_level: SkillLevel, _entity_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn cast_ground_skill(&self, _skill_id: SkillId, _skill_level: SkillLevel, _target_position: TilePosition) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn cast_channeling_skill(&self, _skill_id: SkillId, _skill_level: SkillLevel, _entity_id: EntityId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn stop_channeling_skill(&self, _skill_id: SkillId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn add_friend(&self, _name: String) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn remove_friend(&self, _account_id: AccountId, _character_id: CharacterId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn reject_friend_request(&self, _account_id: AccountId, _character_id: CharacterId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn accept_friend_request(&self, _account_id: AccountId, _character_id: CharacterId) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn set_hotkey_data(&self, _tab: HotbarTab, _index: HotbarSlot, _hotkey_data: HotkeyData) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn select_buy_or_sell(&self, _shop_id: ShopId, _buy_or_sell: BuyOrSellOption) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn purchase_items(&self, _items: Vec<BuyShopItemInformation>) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn close_shop(&self) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }

    fn sell_items(&self, _items: Vec<SoldItemInformation>) -> impl MapServerPacket {
        EmptyMapServerPacket {}
    }
}
