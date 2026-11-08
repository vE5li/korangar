//! Headless test-stack integration. Never accepts arbitrary game endpoints.
//! Run: cargo run -p korangar-networking --example hercules_test --
//! /private/account.json Credentials and session packets are never printed. No
//! game assets are read.
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use korangar_networking::{NetworkEvent, NetworkingSystem, SupportedPacketVersion};
use ragnarok_bytes::{ByteReader, ByteWriter};
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Account {
    username: String,
    password: String,
    login_address: String,
}
#[derive(Clone, Default)]
struct Audit {
    position: Arc<Mutex<Option<WorldPosition>>>,
    unsupported: Arc<Mutex<BTreeSet<u16>>>,
}
impl PacketCallback for Audit {
    fn incoming_packet<P: Packet>(&self, p: &P) {
        if P::HEADER == MapServerLoginSuccessPacket::HEADER {
            let mut writer = ByteWriter::new();
            p.packet_to_bytes(&mut writer).unwrap();
            let mut reader = ByteReader::without_metadata(writer.as_slice());
            let packet = MapServerLoginSuccessPacket::packet_from_bytes(&mut reader).unwrap();
            *self.position.lock().unwrap() = Some(packet.position);
            println!("PASS map-login-ack position={},{}", packet.position.x, packet.position.y);
        }
    }

    fn failed_packet(&self, bytes: Vec<u8>, _: Box<ragnarok_bytes::ConversionError>) {
        if bytes.len() >= 2 {
            eprintln!("codec-failure opcode=0x{:04x}", u16::from_le_bytes([bytes[0], bytes[1]]));
        }
    }

    fn unsupported_packet(&self, opcode: u16) {
        self.unsupported.lock().unwrap().insert(opcode);
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let file = args.get(1).expect("private account JSON path required");
    let create_only = args.iter().any(|arg| arg == "--create-only");
    let scenario = args.iter().find_map(|a| a.strip_prefix("--scenario=")).unwrap_or("smoke");
    assert!(matches!(scenario, "smoke" | "warp" | "npc" | "combat"));
    let account: Account = serde_json::from_slice(&std::fs::read(file).expect("account file unavailable")).expect("invalid account file");
    assert_eq!(
        account.login_address, "192.168.11.110:6910",
        "only isolated hercules-test is permitted"
    );
    assert!(account.username.starts_with("korangar_m1_"), "dedicated test account required");
    let audit = Audit::default();
    let (mut net, mut events) = NetworkingSystem::spawn_with_callback(audit.clone());
    let version = SupportedPacketVersion::_20250716;
    net.connect_to_login_server(
        version,
        account.login_address.parse().unwrap(),
        account.username,
        account.password,
    );
    let start = Instant::now();
    let mut login = None;
    let mut selected = false;
    let mut created = false;
    let mut loaded = false;
    let mut walked = false;
    let mut inventory = false;
    let mut actors = 0;
    let mut actor_updates = 0;
    let mut move_requested = false;
    let mut skill_requested = false;
    let mut healed = false;
    let mut dialog_sent_at: Option<Instant> = None;
    let mut dialog = false;
    let mut npc = None;
    let mut monster = None;
    let mut attacked = false;
    let mut attack_sent_at = None;
    let mut warps = 0;
    let mut map_loaded_at = None;
    while start.elapsed() < Duration::from_secs(16) {
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    let server = character_servers.into_iter().next().expect("missing test character server");
                    assert_eq!(IpAddr::V4(server.server_ip.into()).to_string(), "192.168.11.110");
                    assert_eq!(server.server_port, 6131);
                    println!("PASS login");
                    net.disconnect_from_login_server();
                    net.connect_to_character_server(version, &login_data, server);
                    login = Some(login_data);
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    println!("PASS character-login");
                    net.request_character_list().unwrap();
                }
                NetworkEvent::CharacterList { characters } if !selected && !created => {
                    if characters.is_empty() {
                        created = true;
                        net.create_character(0, "KorangarM1".into()).unwrap();
                    } else {
                        println!("PASS character-list records={}", characters.len());
                        selected = true;
                        net.select_character(0).unwrap();
                    }
                }
                NetworkEvent::CharacterCreated { .. } => {
                    println!("PASS character-create");
                    if create_only {
                        net.disconnect_from_character_server();
                        return;
                    }
                    selected = true;
                    net.select_character(0).unwrap();
                }
                NetworkEvent::CharacterSelected { login_data } => {
                    assert_eq!(login_data.server_ip.to_string(), "192.168.11.110");
                    assert_eq!(login_data.server_port, 5131);
                    println!("PASS character-select");
                    net.disconnect_from_character_server();
                    net.connect_to_map_server(version, login.as_ref().unwrap(), login_data);
                }
                NetworkEvent::UpdateClientTick { .. } if !loaded => {
                    loaded = true;
                    map_loaded_at = Some(Instant::now());
                    net.map_loaded().unwrap();
                    println!("SENT map-loaded");
                }
                NetworkEvent::SetInventory { items } => {
                    inventory = true;
                    println!("PASS inventory items={}", items.len());
                }
                NetworkEvent::PlayerMove { .. } => {
                    walked = true;
                    println!("PASS walk-ack");
                }
                NetworkEvent::EntityMove { .. } | NetworkEvent::UpdateEntityHealth { .. } | NetworkEvent::RemoveEntity { .. } => {
                    actor_updates += 1;
                }
                NetworkEvent::AddEntity { entity_data } => {
                    actors += 1;
                    if !move_requested && (1000..2000).contains(&entity_data.job_id.0) {
                        let distance = |p: WorldPosition| {
                            audit
                                .position
                                .lock()
                                .unwrap()
                                .map(|origin| origin.x.abs_diff(p.x) as u32 + origin.y.abs_diff(p.y) as u32)
                                .unwrap_or(u32::MAX)
                        };
                        if monster.is_none_or(|(_, old)| distance(entity_data.position) < distance(old)) {
                            monster = Some((entity_data.entity_id, entity_data.position));
                        }
                    }
                    println!(
                        "ACTOR job={} position={},{}",
                        entity_data.job_id.0, entity_data.position.x, entity_data.position.y
                    );
                    if npc.is_none()
                        && entity_data.job_id.0 > 40
                        && entity_data.job_id.0 < 1000
                        && entity_data.job_id.0 != 139
                        && entity_data.job_id.0 != 45
                    {
                        npc = Some(entity_data.entity_id);
                    }
                }
                NetworkEvent::ChangeMap { position, .. } => {
                    if audit
                        .position
                        .lock()
                        .unwrap()
                        .is_some_and(|p| p.x != position.x || p.y != position.y)
                    {
                        warps += 1;
                    }
                    *audit.position.lock().unwrap() = Some(WorldPosition {
                        x: position.x,
                        y: position.y,
                        direction: Direction::North,
                    });
                    net.map_loaded().unwrap();
                    println!("PASS warp-map-change position={},{}", position.x, position.y);
                }
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    damage_amount,
                    ..
                } if login.is_some_and(|l| source_entity_id.0 == l.account_id.0) && damage_amount.is_some_and(|v| v > 0) => {
                    attacked = true;
                    println!("PASS attack-damage-event");
                }
                NetworkEvent::HealEffect { entity_id, .. } if login.is_some_and(|l| entity_id.0 == l.account_id.0) => {
                    healed = true;
                    println!("PASS skill-heal");
                }
                NetworkEvent::OpenDialog { .. } => {
                    dialog = true;
                    println!("PASS NPC-dialog");
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    net.close_dialog(npc_id).unwrap();
                }
                NetworkEvent::AddNextButton { npc_id } => {
                    net.next_dialog(npc_id).unwrap();
                }
                NetworkEvent::LoginServerConnectionFailed { reason, .. } => panic!("test login failed: {reason:?}"),
                NetworkEvent::CharacterServerConnectionFailed { .. } => panic!("test character login failed"),
                NetworkEvent::CharacterCreationFailed { reason, .. } => panic!("test character creation failed: {reason:?}"),
                NetworkEvent::CharacterServerDisconnected {
                    reason: korangar_networking::DisconnectReason::ConnectionError,
                } => panic!("test character stream/profile mismatch"),
                NetworkEvent::MapServerDisconnected { .. } => panic!("test map disconnected"),
                _ => {}
            }
        }
        if loaded && map_loaded_at.is_some_and(|t: Instant| t.elapsed() > Duration::from_secs(2)) {
            if !skill_requested {
                net.cast_skill(SkillId(142), SkillLevel(1), EntityId(login.unwrap().account_id.0))
                    .unwrap();
                skill_requested = true;
            }
            if !move_requested && map_loaded_at.is_some_and(|t| t.elapsed() > Duration::from_secs(3)) {
                let target = if scenario == "combat" {
                    monster.map(|(_, mut pos)| {
                        pos.x += 1;
                        pos
                    })
                } else {
                    audit.position.lock().unwrap().map(|mut pos| {
                        if scenario == "warp" {
                            pos.x = if pos.x < 40 { 27 } else { 47 };
                            pos.y = 30;
                        } else {
                            pos.x += 1;
                        }
                        pos
                    })
                };
                if let Some(target) = target {
                    net.player_move(target).unwrap();
                    move_requested = true;
                }
            }
            if scenario == "combat"
                && walked
                && map_loaded_at.is_some_and(|t| t.elapsed() > Duration::from_secs(6))
                && attack_sent_at.is_none_or(|t: Instant| t.elapsed() > Duration::from_secs(1))
                && let Some((target, _)) = monster
            {
                net.player_attack(target).unwrap();
                attack_sent_at = Some(Instant::now());
            }
            if scenario == "npc"
                && !dialog
                && dialog_sent_at.is_none_or(|t| t.elapsed() > Duration::from_secs(2))
                && map_loaded_at.is_some_and(|t| t.elapsed() > Duration::from_secs(5))
                && let Some(npc_id) = npc
            {
                net.start_dialog(npc_id).unwrap();
                dialog_sent_at = Some(Instant::now());
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    net.disconnect_from_map_server();
    println!(
        "EVIDENCE scenario={scenario} loaded={loaded} walk={walked} actors={actors} actor_updates={actor_updates} inventory={inventory} \
         skill={healed} npc_dialog={dialog} attack={attacked} warps={warps}"
    );
    println!("UNSUPPORTED opcodes={:04x?}", audit.unsupported.lock().unwrap());
    assert!(loaded && walked && inventory && actors > 0, "minimum map smoke failed");
    match scenario {
        "warp" => assert!(warps > 0, "warp not acknowledged"),
        "npc" => assert!(dialog, "NPC dialog not observed"),
        "combat" => assert!(
            attacked && healed && actor_updates > 0,
            "attack, First Aid, and actor updates must be acknowledged"
        ),
        _ => {}
    }
}
