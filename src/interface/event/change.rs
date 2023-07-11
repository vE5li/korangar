#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    Reresolve,
    Rerender,
    RerenderWindow,
}

impl ChangeEvent {
    pub fn combine(self, other: Self) -> Self {
        let precedence = |&event: &ChangeEvent| match event {
            ChangeEvent::Reresolve => 0,
            ChangeEvent::Rerender => 1,
            ChangeEvent::RerenderWindow => 2,
        };

        IntoIterator::into_iter([self, other])
            .min_by_key(|event| precedence(event))
            .unwrap()
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
        Some(ChangeEvent::Rerender)
    }
}

impl IntoChangeEvent for Reresolve {
    fn into_change_event() -> Option<ChangeEvent> {
        Some(ChangeEvent::Reresolve)
    }
}

impl IntoChangeEvent for Nothing {
    fn into_change_event() -> Option<ChangeEvent> {
        None
    }
}
