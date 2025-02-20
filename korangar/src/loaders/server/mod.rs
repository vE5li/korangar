mod client_info;

use encoding_rs::Encoding;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use korangar_util::FileLoader;
use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

pub use self::client_info::ClientInfo;
use super::GameFileLoader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServiceId(pub usize);

pub fn load_client_info(game_file_loader: &GameFileLoader) -> ClientInfo {
    #[cfg(feature = "debug")]
    let timer = Timer::new("read clientinfo");

    let client_info = game_file_loader
        .get("data\\sclientinfo.xml")
        .or_else(|_| game_file_loader.get("data\\clientinfo.xml"))
        .expect("failed to find clientinfo");

    let content = match get_xml_encoding(&client_info) {
        Some(encoding) => {
            let (cow, _) = encoding.decode_without_bom_handling(&client_info);
            cow
        }
        None => String::from_utf8_lossy(client_info.as_slice()),
    };

    let client_info: ClientInfo = from_str(&content).unwrap();

    #[cfg(feature = "debug")]
    timer.stop();

    client_info
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
