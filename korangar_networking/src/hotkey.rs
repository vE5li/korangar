use ragnarok_packets::HotkeyData;

#[derive(Debug)]
pub enum HotkeyState {
    Bound(HotkeyData),
    Unbound,
}
