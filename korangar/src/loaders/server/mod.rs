mod client_info;

use encoding_rs::Encoding;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use korangar_interface::element::StateElement;
use korangar_loaders::FileLoader;
use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::Event;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

pub use self::client_info::{ClientInfo, ClientInfoPathExt, PacketVersion};
use super::GameFileLoader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, RustState, StateElement)]
pub struct ServiceId(pub usize);

pub fn load_client_info(game_file_loader: &GameFileLoader) -> ClientInfo {
    try_load_client_info(game_file_loader).expect("clientinfo setup failed; run --check-bootstrap for diagnostics")
}

pub fn try_load_client_info(game_file_loader: &GameFileLoader) -> Result<ClientInfo, String> {
    #[cfg(feature = "debug")]
    let timer = Timer::new("read clientinfo");

    let client_info = game_file_loader
        .get("data\\sclientinfo.xml")
        .or_else(|_| game_file_loader.get("data\\clientinfo.xml"))
        .map_err(|_| "Missing clientinfo: provide a local archive/data/sclientinfo.xml overlay (see profiles/turoran-test)".to_owned())?;

    let content = match get_xml_encoding(&client_info) {
        Some(encoding) => {
            let (cow, _) = encoding.decode_without_bom_handling(&client_info);
            cow
        }
        None => String::from_utf8_lossy(client_info.as_slice()),
    };

    let client_info: ClientInfo =
        from_str(&content).map_err(|_| "Malformed clientinfo XML; check the selected native profile".to_owned())?;

    #[cfg(feature = "debug")]
    timer.stop();

    Ok(client_info)
}

fn get_xml_encoding(data: &[u8]) -> Option<&'static Encoding> {
    let mut reader = Reader::from_reader(data);

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Decl(xml_declaration)) => {
                if let Some(Ok(encoding)) = xml_declaration.encoding() {
                    return Encoding::for_label(encoding.as_ref());
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => (),
        }
        buffer.clear();
    }

    None
}

#[cfg(test)]
mod native_profile_tests {
    use super::*;
    #[test]
    fn checked_in_test_profile_selects_only_isolated_modern_server() {
        let info: ClientInfo = from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../profiles/turoran-test/clientinfo.xml"
        )))
        .unwrap();
        assert_eq!(info.services.len(), 1);
        let service = &info.services[0];
        assert_eq!(service.address, "192.168.11.110");
        assert_eq!(service.port, 6910);
        assert!(matches!(service.packet_version, Some(PacketVersion::_20250716)));
    }
}
