use std::any::TypeId;

use etherparse::{SlicedPacket, TransportSlice};
use korangar_debug::logging::symbols::ARROW;
use korangar_debug::logging::{Colorize, Colorized};
use ragnarok_bytes::{ByteReader, ConversionError};
use ragnarok_packets::handler::{HandlerResult, PacketCallback, PacketHandler};
use ragnarok_packets::*;

// Adjust this to change which packets will print their content!
const EXPAND_BEHAVIOUR: ExpandBehaviour = ExpandBehaviour::Blacklist(&[
    TypeId::of::<LoginServerKeepalivePacket>(),
    TypeId::of::<CharacterServerKeepalivePacket>(),
    TypeId::of::<RequestServerTickPacket>(),
    TypeId::of::<ServerTickPacket>(),
    TypeId::of::<MapServerPingPacket>(),
]);

#[allow(dead_code)]
enum ExpandBehaviour {
    None,
    All,
    Whitelist(&'static [TypeId]),
    Blacklist(&'static [TypeId]),
}

#[derive(Clone)]
enum ServerType {
    Login,
    Character,
    Map,
}

impl ServerType {
    pub fn convert(&self) -> Colorized<'static, &'static str> {
        match self {
            ServerType::Login => "Login".green(),
            ServerType::Character => "Character".yellow(),
            ServerType::Map => "Map".cyan(),
        }
    }
}

#[derive(Clone)]
enum Direction {
    Incoming,
    Outgoing,
}

impl Direction {
    pub fn convert(&self) -> Colorized<'static, &'static str> {
        match self {
            Direction::Incoming => "Incoming".green(),
            Direction::Outgoing => "Outgoing".red(),
        }
    }
}

#[derive(Clone)]
struct PrintCallback {
    server_type: ServerType,
    direction: Direction,
}

impl PrintCallback {
    pub fn new(server_type: ServerType, direction: Direction) -> Self {
        Self { server_type, direction }
    }
}

impl PacketCallback for PrintCallback {
    fn incoming_packet<Packet>(&self, _packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
    }

    fn unknown_packet(&self, bytes: Vec<u8>) {
        if bytes.len() > 2 {
            let header = format!("0x{:0>4x}", bytes[0] as u16 | (bytes[1] as u16) << 8);

            println!(
                "Unknown {} packet on {} server with header {}: {:?}",
                self.direction.convert(),
                self.server_type.convert(),
                header.red(),
                &bytes[2..]
            );
        } else {
            println!(
                "Trailing {} bytes on {} server: {:?}",
                self.direction.convert(),
                self.server_type.convert(),
                bytes
            );
        }
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        if bytes.len() > 2 {
            let header = format!("0x{:0>4x}", bytes[0] as u16 | (bytes[1] as u16) << 8);

            println!(
                "Error {} packet on {} server with header {} and error {:?}: {:?}",
                self.direction.convert(),
                self.server_type.convert(),
                header.red(),
                error.red(),
                &bytes[2..]
            );
        } else {
            println!(
                "Trailing {} bytes on {} server with error {:?}: {:?}",
                self.direction.convert(),
                self.server_type.convert(),
                error.red(),
                bytes
            );
        }
    }
}

fn handler<P: Packet + 'static>(server_type: ServerType, direction: Direction) -> impl Fn(P) {
    move |packet: P| {
        let header = format!("0x{:0>4x}", P::HEADER.0);

        println!(
            "{} packet on {} server: {} ({})",
            direction.convert(),
            server_type.convert(),
            std::any::type_name::<P>().cyan(),
            header.green()
        );

        match EXPAND_BEHAVIOUR {
            ExpandBehaviour::None => {}
            ExpandBehaviour::Whitelist(types) if !types.contains(&TypeId::of::<P>()) => {}
            ExpandBehaviour::Blacklist(types) if types.contains(&TypeId::of::<P>()) => {}
            _ => {
                let arrow = match direction {
                    Direction::Incoming => ARROW.green(),
                    Direction::Outgoing => ARROW.red(),
                };
                println!(" {arrow} {packet:?}")
            }
        }
    }
}

macro_rules! create_handler {
    ($server_type:expr, $direction:expr, [$($packet:ty),* $(,)?]) => {
        {
            let mut new_handler = PacketHandler::<(), (), _>::with_callback(PrintCallback::new($server_type, $direction));
            $(
                new_handler.register(handler::<$packet>($server_type, $direction)).unwrap();
            )*

            new_handler
        }
    };
}

fn main() {
    const DEVICE: &str = "wlp5s0";

    const LOGIN_SERVER_PORT: u16 = 6900;
    const CHARACTER_SERVER_PORT: u16 = 6121;
    const MAP_SERVER_PORT: u16 = 5121;

    let mut cap = pcap::Capture::from_device(DEVICE).unwrap().immediate_mode(true).open().unwrap();
    cap.filter("host 49.12.109.207", true).unwrap();

    let mut client_login_handler = create_handler!(ServerType::Login, Direction::Incoming, [
        LoginServerLoginSuccessPacket,
        LoginFailedPacket,
        LoginFailedPacket2
    ]);

    let mut server_login_handler = create_handler!(ServerType::Login, Direction::Outgoing, [
        LoginServerLoginPacket,
        LoginServerKeepalivePacket,
    ]);

    let mut client_character_handler = create_handler!(ServerType::Character, Direction::Incoming, [
        LoginFailedPacket,
        CharacterServerLoginSuccessPacket,
        RequestCharacterListSuccessPacket,
        Packet0b18,
        CharacterSelectionSuccessPacket,
        CharacterSelectionFailedPacket,
        MapServerUnavailablePacket,
        CreateCharacterSuccessPacket,
        CharacterCreationFailedPacket,
        CharacterDeletionSuccessPacket,
        CharacterDeletionFailedPacket,
        SwitchCharacterSlotResponsePacket,
    ]);

    let mut server_character_handler = create_handler!(ServerType::Character, Direction::Outgoing, [
        CharacterServerKeepalivePacket,
        RequestCharacterListPacket,
        CharacterListPacket,
        CharacterSlotPagePacket,
        CharacterBanListPacket,
        LoginPincodePacket,
        SelectCharacterPacket,
        CreateCharacterPacket,
        DeleteCharacterPacket,
        SwitchCharacterSlotPacket,
    ]);

    let mut client_map_handler = create_handler!(ServerType::Map, Direction::Incoming, [
        MapServerPingPacket,
        BroadcastMessagePacket,
        Broadcast2MessagePacket,
        OverheadMessagePacket,
        ServerMessagePacket,
        EntityMessagePacket,
        DisplayEmotionPacket,
        EntityMovePacket,
        EntityStopMovePacket,
        PlayerMovePacket,
        ChangeMapPacket,
        ResurrectionPacket,
        EntityAppearedPacket,
        EntityAppeared2Packet,
        MovingEntityAppearedPacket,
        EntityDisappearedPacket,
        UpdateStatPacket,
        UpdateStatPacket1,
        UpdateStatPacket2,
        UpdateStatPacket3,
        UpdateAttackRangePacket,
        NewMailStatusPacket,
        AchievementUpdatePacket,
        AchievementListPacket,
        CriticalWeightUpdatePacket,
        SpriteChangePacket,
        InventoyStartPacket,
        RegularItemListPacket,
        EquippableItemListPacket,
        InventoyEndPacket,
        EquippableSwitchItemListPacket,
        MapTypePacket,
        UpdateSkillTreePacket,
        UpdateHotkeysPacket,
        InitialStatsPacket,
        UpdatePartyInvitationStatePacket,
        UpdateShowEquipPacket,
        UpdateConfigurationPacket,
        NavigateToMonsterPacket,
        MarkMinimapPositionPacket,
        NextButtonPacket,
        CloseButtonPacket,
        DialogMenuPacket,
        DisplaySpecialEffectPacket,
        DisplaySkillCooldownPacket,
        DisplaySkillEffectAndDamagePacket,
        DisplaySkillEffectNoDamagePacket,
        DisplayPlayerHealEffect,
        StatusChangePacket,
        DamagePacket1,
        DamagePacket3,
        QuestNotificationPacket1,
        HuntingQuestNotificationPacket,
        HuntingQuestUpdateObjectivePacket,
        QuestRemovedPacket,
        QuestListPacket,
        VisualEffectPacket,
        DisplayGainedExperiencePacket,
        DisplayImagePacket,
        StateChangePacket,
        QuestEffectPacket,
        ItemPickupPacket,
        RemoveItemFromInventoryPacket,
        ServerTickPacket,
        RequestPlayerDetailsSuccessPacket,
        RequestEntityDetailsSuccessPacket,
        UpdateEntityHealthPointsPacket,
        RequestPlayerAttackFailedPacket,
        NpcDialogPacket,
        RequestEquipItemStatusPacket,
        RequestUnequipItemStatusPacket,
        Packet8302,
        Packet0b18,
        MapServerLoginSuccessPacket,
        RestartResponsePacket,
        DisconnectResponsePacket,
        UseSkillSuccessPacket,
        ToUseSkillSuccessPacket,
        NotifySkillUnitPacket,
        SkillUnitDisappearPacket,
        NotifyGroundSkillPacket,
        FriendListPacket,
        FriendOnlineStatusPacket,
        FriendRequestPacket,
        FriendRequestResultPacket,
        NotifyFriendRemovedPacket,
        PartyInvitePacket,
        StatusChangeSequencePacket,
        ReputationPacket,
        ClanInfoPacket,
        ClanOnlineCountPacket,
        ChangeMapCellPacket,
        OpenMarketPacket,
        BuyOrSellPacket,
        ShopItemListPacket,
        BuyShopItemsResultPacket,
        ParameterChangePacket,
        SellListPacket,
        SellItemsPacket,
        SellItemsResultPacket,
    ]);

    let mut server_map_handler = create_handler!(ServerType::Map, Direction::Outgoing, [
        MapLoadedPacket,
        RestartPacket,
        RequestPlayerMovePacket,
        RequestWarpToMapPacket,
        RequestDetailsPacket,
        RequestActionPacket,
        GlobalMessagePacket,
        StartDialogPacket,
        NextDialogPacket,
        CloseDialogPacket,
        ChooseDialogOptionPacket,
        RequestEquipItemPacket,
        RequestUnequipItemPacket,
        UseSkillAtIdPacket,
        UseSkillOnGroundPacket,
        StartUseSkillPacket,
        EndUseSkillPacket,
        AddFriendPacket,
        RemoveFriendPacket,
        FriendRequestResponsePacket,
        SetHotkeyData2Packet,
        SelectBuyOrSellPacket,
        BuyShopItemsPacket,
        CloseShopPacket,
        SellItemsPacket,
        RequestServerTickPacket,
    ]);

    println!("{}", "Listening for packets".green());

    while let Ok(packet) = cap.next_packet() {
        match SlicedPacket::from_ethernet(packet.data) {
            Err(value) => println!("Err {:?}", value),
            Ok(value) => {
                if let Some(TransportSlice::Tcp(tcp_slice)) = value.transport {
                    let source_port = tcp_slice.source_port();
                    let destination_port = tcp_slice.destination_port();

                    // FIX: Obviously this will break if the local port is one of the server ports.
                    // Check if the packet in incoming or outgoing beforehand.

                    // FIX: Handle cut off packets.

                    if source_port == LOGIN_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = client_login_handler.process_one(&mut byte_reader) {}
                    } else if destination_port == LOGIN_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = server_login_handler.process_one(&mut byte_reader) {}
                    } else if source_port == CHARACTER_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = client_character_handler.process_one(&mut byte_reader) {}
                    } else if destination_port == CHARACTER_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = server_character_handler.process_one(&mut byte_reader) {}
                    } else if source_port == MAP_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = client_map_handler.process_one(&mut byte_reader) {}
                    } else if destination_port == MAP_SERVER_PORT {
                        let mut byte_reader = ByteReader::without_metadata(tcp_slice.payload());
                        while let HandlerResult::Ok(_) = server_map_handler.process_one(&mut byte_reader) {}
                    }
                };
            }
        }
    }
}
