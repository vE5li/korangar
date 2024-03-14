use bitflags::bitflags;

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ChangeEvent: u8 {
        const RENDER_WINDOW = 0b00000001;
        const RESOLVE_WINDOW = 0b00000010;
        const RENDER = 0b00000100;
        const RESOLVE = 0b00001000;
    }
}

pub trait IntoChangeEvent {
    fn into_change_event() -> Option<ChangeEvent>;
}

pub struct Render {}
pub struct Resolve {}
pub struct Nothing {}

impl IntoChangeEvent for Render {
    fn into_change_event() -> Option<ChangeEvent> {
        Some(ChangeEvent::RENDER)
    }
}

impl IntoChangeEvent for Resolve {
    fn into_change_event() -> Option<ChangeEvent> {
        Some(ChangeEvent::RESOLVE)
    }
}

impl IntoChangeEvent for Nothing {
    fn into_change_event() -> Option<ChangeEvent> {
        None
    }
}
