use std::net::{IpAddr, SocketAddr};

use ragnarok_packets::{AccountId, CharacterId, Sex};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::event::NetworkEvent;

#[derive(Debug, Clone, Copy)]
pub struct LoginServerLoginData {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    pub sex: Sex,
}

#[derive(Debug, Clone, Copy)]
pub enum UnifiedLoginFailedReason {
    ServerClosed,
    AlreadyLoggedIn,
    AlreadyOnline,
    UnregisteredId,
    IncorrectPassword,
    IdExpired,
    RejectedFromServer,
    BlockedByGMTeam,
    GameOutdated,
    LoginProhibitedUntil,
    ServerFull,
    CompanyAccountLimitReached,
}

#[derive(Debug, Clone, Copy)]
pub enum UnifiedCharacterSelectionFailedReason {
    RejectedFromServer,
    MapServerUnavailable,
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterServerLoginData {
    pub server_ip: IpAddr,
    pub server_port: u16,
    pub character_id: CharacterId,
}

pub(crate) enum ServerConnectCommand {
    Login {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
    },
    Character {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
    },
    Map {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
    },
}

#[derive(Debug)]
pub(crate) enum NetworkTaskError {
    FailedToConnect,
    ConnectionClosed,
}

#[derive(Debug)]
pub struct NotConnectedError;

pub(crate) enum ServerConnection {
    Connected {
        action_sender: UnboundedSender<Vec<u8>>,
        event_receiver: UnboundedReceiver<NetworkEvent>,
    },
    ClosingManually,
    Disconnected,
}

impl ServerConnection {
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, ServerConnection::Disconnected)
    }
}
