//! Packet version 2025-04-16.
//!
//! For every packet implemented by this crate the wire format is identical to
//! packet version 2022-04-06 (verified against rAthena's `PACKETVER` guards up
//! to 20250402), so this module re-uses those registrations. It additionally
//! registers no-op handlers for the packets introduced between the two
//! versions, so that receiving one of them does not disrupt the packet stream.

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use super::version_20220406;
use crate::event::NetworkEventList;

pub fn register_login_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    version_20220406::register_login_server_packets(packet_handler)
}

pub fn register_character_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    version_20220406::register_character_server_packets(packet_handler)
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    version_20220406::register_map_server_packets(packet_handler)?;

    // Packets introduced between 2022-04-06 and 2025-04-16. None of them are
    // required for gameplay, but they must at least be consumed correctly so
    // that packets following them in the same read are not discarded.
    packet_handler.register_noop::<SpecialPopupPacket>()?;
    packet_handler.register_noop::<DialogWindowSizePacket>()?;
    packet_handler.register_noop::<DialogWindowPosPacket>()?;
    packet_handler.register_noop::<DialogWindowPos2Packet>()?;
    packet_handler.register_noop::<PlayNpcBgmPacket>()?;
    packet_handler.register_noop::<MacroCheckerResultPacket>()?;

    Ok(())
}
