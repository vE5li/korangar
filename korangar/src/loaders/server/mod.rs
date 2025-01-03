mod client_info;

use encoding_rs::{
    Encoding, BIG5, EUC_KR, SHIFT_JIS, UTF_8, WINDOWS_1250, WINDOWS_1251, WINDOWS_1252, WINDOWS_1253, WINDOWS_1254, WINDOWS_1255,
    WINDOWS_1256, WINDOWS_1257, WINDOWS_1258, WINDOWS_874,
};
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_util::FileLoader;
use quick_xml::events::Event;
use quick_xml::Reader;
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
            let (cow, ..) = encoding.decode(&client_info);
            cow
        }
        None => String::from_utf8_lossy(client_info.as_slice()),
    };

    let client_info: ClientInfo = quick_xml::de::from_str(&content).unwrap();

    #[cfg(feature = "debug")]
    timer.stop();

    client_info
}

fn get_xml_encoding(data: &[u8]) -> Option<&'static Encoding> {
    let mut reader = Reader::from_reader(data);

    let mut buffer = Vec::new();

    let mut xml_encoding = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Decl(xml_declaration)) => {
                if let Some(Ok(encoding)) = xml_declaration.encoding() {
                    xml_encoding = Some(String::from_utf8_lossy(&encoding).into_owned());
                    break;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => (),
        }
        buffer.clear();
    }

    xml_encoding.and_then(|xml_encoding| match xml_encoding.to_ascii_lowercase().as_str() {
        // The following encodings are the legacy standard windows encoding that might have been used.
        "big5" => Some(BIG5),
        "euc-kr" => Some(EUC_KR),
        "shift_js" => Some(SHIFT_JIS),
        "windows-874" => Some(WINDOWS_874),
        "windows-1250" => Some(WINDOWS_1250),
        "windows-1251" => Some(WINDOWS_1251),
        "windows-1252" => Some(WINDOWS_1252),
        "windows-1253" => Some(WINDOWS_1253),
        "windows-1254" => Some(WINDOWS_1254),
        "windows-1255" => Some(WINDOWS_1255),
        "windows-1256" => Some(WINDOWS_1256),
        "windows-1257" => Some(WINDOWS_1257),
        "windows-1258" => Some(WINDOWS_1258),
        "utf-8" => Some(UTF_8),
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown encoding used", "warn".red());
            None
        }
    })
}
