use serde::de::Error;
use serde::{Deserialize, Deserializer};

/// The ClientInfo structure.
///
/// It is a file that configures your client to connect to RO Servers.
/// Usually, it uses the *clientinfo.xml* file inside the data folder.
/// But it can use *sclientinfo.xml* if this one exists.
///
/// See more: https://github.com/rathena/rathena/wiki/Clientinfo.xml
#[derive(Default, Debug, Deserialize)]
pub struct ClientInfo {
    /// ClientInfo's description.
    #[serde(alias = "desc")]
    pub description: Option<String>,

    /// Specifies the client service type (country). Affects client behavior
    /// for some features. See `ServiceType` enum for valid values.
    /// Should be set to `korea`, unless you know, what you want to achieve by
    /// using another value.
    #[serde(alias = "servicetype", deserialize_with = "service_type_from_name")]
    pub service_type: ServiceType,

    /// Specifies the server type this client is intended for. Affects client
    /// behavior for some features. See `ServerType` enum for valid values.
    /// Should be set to sakray, unless you know, what you want to achieve by
    /// using another value.
    #[serde(alias = "servertype", deserialize_with = "server_type_from_name")]
    pub server_type: ServerType,

    /// When present, disallows the client from showing Service Select screen
    /// and uses the first entry automatically.
    #[serde(default, alias = "hideaccountlist", deserialize_with = "bool_deserializer")]
    pub hide_account_list: bool,

    /// When present, passwords are encrypted (method 1) before being sent to
    /// the server. Incompatible with **use_MD5_passwords** defined in
    /// server's login_athena.conf.
    #[serde(default, alias = "passwordencrypt", deserialize_with = "bool_deserializer")]
    pub password_encrypt: bool,

    /// When present, passwords are encrypted (method 2) before being sent to
    /// the server. When you use this with `<passwordencrypt />`, method 2
    /// will be used. Incompatible with **use_MD5_passwords** defined in
    /// server's login_athena.conf.
    #[serde(default, alias = "passwordencrypt2", deserialize_with = "bool_deserializer")]
    pub password_encrypt2: bool,

    /// When present, all character slots (usually 9) are available.
    /// Otherwise only 2-4 are enabled for use, others are displayed
    /// as 'Not available'.
    #[serde(default, alias = "extendedslot", deserialize_with = "bool_deserializer")]
    pub extended_slot: bool,

    /// When present, all files are first searched inside the /data/ folder,
    /// then inside the GRF archives. Otherwise files inside the /data/
    /// folder are only accessed, when they were not found inside the GRF
    /// archives.
    #[serde(default, alias = "readfolder", deserialize_with = "bool_deserializer")]
    pub read_folder: bool,

    /// Defines each available connection on the Service Select screen
    #[serde(default, alias = "connection")]
    pub services: Vec<Service>,
}

/// The ClientInfo's Service structure
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Service {
    /// Displays the name of the server at the Service Select screen.
    #[serde(alias = "display")]
    pub display_name: Option<String>,

    /// Server description.
    #[serde(alias = "desc")]
    pub description: Option<String>,

    /// This tag tells the game to display a tooltip when the cursor is over the
    /// server name on the server select screen.
    pub balloon: Option<String>,

    /// IP or DNS address of the server (for DNS, client needs the DNS hex).
    pub address: String,

    /// Connection server port (default 6900).
    pub port: i16,

    /// Must be equal to **client_version_to_connect** defined in server's
    /// login_athena.conf.
    pub version: i8,

    /// Uses the same value from `ServiceType` enum.
    #[serde(default, alias = "langtype", deserialize_with = "language_type_from_index")]
    pub language_type: Option<ServiceType>,

    /// Web address to open when you click the Register button.
    #[serde(default, alias = "registrationweb")]
    pub registration_web: Option<String>,

    /// Characters with the following Account IDs will be seen in GM sprites
    /// and have Yellow name/chat (add the Account IDs of all your GMs here).
    #[serde(default, alias = "yellow")]
    pub game_master_yellow_ids: Vec<GameMasterAccount>,

    /// Use instead of to also allow the right-click menu for your GMs.
    #[serde(default, alias = "aid")]
    pub game_master_accounts: Vec<GameMasterAccount>,

    /// Define each loading screen in the path `/data/texture/À¯ÀúÀÎÅÍÆäÀÌ½º/`
    #[serde(default, alias = "loading")]
    pub loading_images: Option<Vec<LoadingImage>>,
}

/// The ClientInfo Service's Account ID structure
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GameMasterAccount {
    /// GM's Account ID
    #[serde(alias = "admin")]
    pub account_id: Option<i32>,
}

/// The ClientInfo Service's Loading Image structure
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoadingImage {
    /// File name
    #[serde(alias = "image")]
    pub filename: Option<String>,
}

/// The ClientInfo's ServiceType enumerator.
#[derive(Debug, Clone, Default, Deserialize)]
pub enum ServiceType {
    #[default]
    Korea,
    America,
    Japan,
    China,
    Taiwan,
    Thai,
    Indonesia,
    Philippine,
    Malaysia,
    Singapore,
    Germany,
    India,
    Brazil,
    Australia,
    Russia,
    Vietnam,
    Chile,
    France,
    Uae,
    Unknown = 999,
}

impl ServiceType {
    pub fn from_index(index: i8) -> Self {
        match index {
            0 => ServiceType::Korea,
            1 => ServiceType::America,
            2 => ServiceType::Japan,
            3 => ServiceType::China,
            4 => ServiceType::Taiwan,
            5 => ServiceType::Thai,
            6 => ServiceType::Indonesia,
            7 => ServiceType::Philippine,
            8 => ServiceType::Malaysia,
            9 => ServiceType::Singapore,
            10 => ServiceType::Germany,
            11 => ServiceType::India,
            12 => ServiceType::Brazil,
            13 => ServiceType::Australia,
            14 => ServiceType::Russia,
            15 => ServiceType::Vietnam,
            17 => ServiceType::Chile,
            18 => ServiceType::France,
            19 => ServiceType::Uae,
            _ => ServiceType::Unknown,
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "korea" => ServiceType::Korea,
            "america" => ServiceType::America,
            "japan" => ServiceType::Japan,
            "china" => ServiceType::China,
            "taiwan" => ServiceType::Taiwan,
            "thai" => ServiceType::Thai,
            "indonesia" => ServiceType::Indonesia,
            "philippine" => ServiceType::Philippine,
            "malaysia" => ServiceType::Malaysia,
            "singapore" => ServiceType::Singapore,
            "germany" => ServiceType::Germany,
            "india" => ServiceType::India,
            "brazil" => ServiceType::Brazil,
            "australia" => ServiceType::Australia,
            "russia" => ServiceType::Russia,
            "vietnam" => ServiceType::Vietnam,
            "chile" => ServiceType::Chile,
            "france" => ServiceType::France,
            "uae" => ServiceType::Uae,
            _ => ServiceType::Unknown,
        }
    }
}

fn language_type_from_index<'de, D>(deserializer: D) -> Result<Option<ServiceType>, D::Error>
where
    D: Deserializer<'de>,
{
    let index: i8 = Deserialize::deserialize(deserializer)?;
    let service_type = ServiceType::from_index(index);

    match service_type {
        ServiceType::Unknown => Err(D::Error::custom("invalid service type {index}")),
        value => Ok(Some(value)),
    }
}

fn service_type_from_name<'de, D>(deserializer: D) -> Result<ServiceType, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(deserializer)?;
    let service_type = ServiceType::from_name(&name);

    match service_type {
        ServiceType::Unknown => Err(D::Error::custom("invalid service type {name}")),
        value => Ok(value),
    }
}

/// The ClientInfo's ServerType enumerator.
#[derive(Debug, Clone, Default, Deserialize)]
pub enum ServerType {
    #[default]
    Primary,
    Sakray,
    Local,
    Pk = 5,
    Unknown = 999,
}

impl ServerType {
    pub fn from_name(name: &str) -> Self {
        match name {
            "primary" => ServerType::Primary,
            "sakray" => ServerType::Sakray,
            "local" => ServerType::Local,
            "pk" => ServerType::Pk,
            _ => ServerType::Unknown,
        }
    }
}

fn server_type_from_name<'de, D>(deserializer: D) -> Result<ServerType, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(deserializer)?;
    let server_type = ServerType::from_name(&name);

    match server_type {
        ServerType::Unknown => Err(D::Error::custom("invalid server type {name}")),
        value => Ok(value),
    }
}

fn bool_deserializer<'de, D>(data: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(data)?;

    match value.as_ref() {
        "" => Ok(true),
        _ => panic!("boolean tags may not have any data inside"),
    }
}
