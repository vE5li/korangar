use crate::packet_version::{CharPacketHandlerRegister, LoginPacketHandlerRegister, MapPacketHandlerRegister};

#[derive(Copy, Clone)]
pub struct BaseLoginPacketReceiver;
#[derive(Copy, Clone)]
pub struct BaseCharPacketReceiver;
#[derive(Copy, Clone)]
pub struct BaseMapPacketReceiver;

impl LoginPacketHandlerRegister for BaseLoginPacketReceiver {}

impl CharPacketHandlerRegister for BaseCharPacketReceiver {}

impl MapPacketHandlerRegister for BaseMapPacketReceiver {}
