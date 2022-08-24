#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChangeEvent {
    Reresolve,
    Rerender,
    RerenderWindow,
}

impl ChangeEvent {

    pub fn combine(self, other: Self) -> Self {

        let precedence = |event: &ChangeEvent| match *event {
            ChangeEvent::Reresolve => 0,
            ChangeEvent::Rerender => 1,
            ChangeEvent::RerenderWindow => 2,
        };

        IntoIterator::into_iter([self, other])
            .min_by_key(|event| precedence(event))
            .unwrap()
    }
}

pub const RERENDER: Option<ChangeEvent> = Some(ChangeEvent::Rerender);
pub const RERESOLVE: Option<ChangeEvent> = Some(ChangeEvent::Reresolve);
pub const NO_EVENT: Option<ChangeEvent> = None;
