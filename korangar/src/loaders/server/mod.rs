mod client_info;

use serde::{Deserialize, Serialize};
use serde_xml_rs::de::Deserializer;
use xml::reader::{EventReader, ParserConfig};

pub use self::client_info::ClientInfo;
use super::GameFileLoader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServiceId(pub usize);

pub fn load_client_info(game_file_loader: &mut GameFileLoader) -> ClientInfo {
    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("read clientinfo");

    let clientinfo = game_file_loader
        .get("data\\sclientinfo.xml")
        .or_else(|_| game_file_loader.get("data\\clientinfo.xml"))
        .expect("failed to find clientinfo");

    let source = String::from_utf8(clientinfo).unwrap();

    // TODO: Make it work with euc-kr, since it panics
    // with error: "Unsupported encoding: euc-kr"
    let replaced_source = source.replace("euc-kr", "utf8");

    let config = ParserConfig::new().trim_whitespace(true);
    let event_reader = EventReader::new_with_config(replaced_source.as_bytes(), config);
    let client_info = ClientInfo::deserialize(&mut Deserializer::new(event_reader)).unwrap();

    #[cfg(feature = "debug")]
    timer.stop();

    client_info
}
