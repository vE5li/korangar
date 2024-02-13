use bitflags::bitflags;

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ChangeEvent: u8 {
        const RERENDER_WINDOW = 0b00000001;
        const RERESOLVE_WINDOW = 0b00000010;
        const RERENDER = 0b00000100;
        const RERESOLVE = 0b00001000;
    }
}

pub trait IntoChangeEvent {
    fn into_change_event() -> Option<ChangeEvent>;
}

pub struct Rerender {}
pub struct Reresolve {}
pub struct Nothing {}

impl IntoChangeEvent for Rerender {
    fn into_change_event() -> Option<ChangeEvent> {
        Some(ChangeEvent::RERENDER)
    }
}

impl IntoChangeEvent for Reresolve {
    fn into_change_event() -> Option<ChangeEvent> {
        Some(ChangeEvent::RERESOLVE)
    }
}

impl IntoChangeEvent for Nothing {
    fn into_change_event() -> Option<ChangeEvent> {
        None
    }
}
