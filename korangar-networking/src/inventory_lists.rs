//! Modern inventory lists share framing across inventory, cart and storage.
//! Never publish a non-inventory or failed transfer as the player inventory.
use std::collections::HashMap;

pub(crate) struct InventoryLists<T> {
    pending: HashMap<u8, Vec<T>>,
}
impl<T> Default for InventoryLists<T> {
    fn default() -> Self {
        Self { pending: HashMap::new() }
    }
}
impl<T> InventoryLists<T> {
    pub fn start(&mut self, context: u8) {
        self.pending.insert(context, Vec::new());
    }

    pub fn extend(&mut self, context: u8, items: impl IntoIterator<Item = T>) {
        if let Some(pending) = self.pending.get_mut(&context) {
            pending.extend(items);
        }
    }

    pub fn finish(&mut self, context: u8, flag: u8) -> Option<Vec<T>> {
        let items = self.pending.remove(&context)?;
        // Other container UIs are not implemented yet. Discard their completed
        // snapshot rather than corrupting the ordinary inventory state.
        (context == 0 && flag == 0).then_some(items)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interleaved_containers_cannot_replace_or_mix_player_inventory() {
        let mut lists = InventoryLists::default();
        lists.start(0);
        lists.extend(0, [10]);
        for context in 1..=3 {
            lists.start(context);
            lists.extend(context, [context as i32]);
        }
        lists.extend(0, [20]);
        for context in 1..=3 {
            assert_eq!(lists.finish(context, 0), None);
        }
        assert_eq!(lists.finish(0, 0), Some(vec![10, 20]));
        assert_eq!(lists.finish(0, 0), None);
    }
    #[test]
    fn failed_or_unstarted_transfers_do_not_clear_inventory() {
        let mut lists = InventoryLists::<u8>::default();
        lists.extend(0, [1]);
        assert_eq!(lists.finish(0, 0), None);
        lists.start(0);
        lists.extend(0, [1]);
        assert_eq!(lists.finish(0, 1), None);
        lists.start(0);
        assert_eq!(lists.finish(0, 0), Some(vec![]));
    }
    #[test]
    fn wire_contexts_only_publish_one_player_inventory_for_all_profiles() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        use ragnarok_bytes::ByteWriter;
        use ragnarok_packets::*;

        use crate::{CharacterServerLoginData, LoginServerLoginData, NetworkEvent, NetworkingSystem, SupportedPacketVersion};
        for version in [
            SupportedPacketVersion::_20190530,
            SupportedPacketVersion::_20220406,
            SupportedPacketVersion::_20250716,
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let length = match version {
                    SupportedPacketVersion::_20190530 => 19,
                    _ => 23,
                };
                stream.read_exact(&mut vec![0u8; length]).unwrap();
                let mut writer = ByteWriter::new();
                for inventory_type in 0..=3 {
                    InventoyStartPacket {
                        inventory_type,
                        inventory_name: "Synthetic".into(),
                    }
                    .packet_to_bytes(&mut writer)
                    .unwrap();
                    InventoyEndPacket { inventory_type, flag: 0 }.packet_to_bytes(&mut writer).unwrap();
                }
                stream.write_all(writer.as_slice()).unwrap();
                std::thread::sleep(Duration::from_millis(500));
            });
            let (mut network, mut events) = NetworkingSystem::spawn();
            network.connect_to_map_server(
                version,
                &LoginServerLoginData {
                    account_id: AccountId(1),
                    login_id1: 0,
                    login_id2: 0,
                    sex: Sex::Male,
                },
                CharacterServerLoginData {
                    server_ip: "127.0.0.1".parse().unwrap(),
                    server_port: port,
                    character_id: CharacterId(1),
                },
            );
            let start = Instant::now();
            let mut replacements = 0;
            while start.elapsed() < Duration::from_secs(4) {
                network.get_events(&mut events);
                let mut disconnected = false;
                for event in events.drain() {
                    match event {
                        NetworkEvent::SetInventory { .. } => replacements += 1,
                        NetworkEvent::MapServerDisconnected { .. } => disconnected = true,
                        _ => {}
                    }
                }
                if disconnected {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            network.disconnect_from_map_server();
            server.join().unwrap();
            assert_eq!(replacements, 1, "only ordinary inventory may replace the player inventory");
        }
    }
}
