use input::UserEvent;

pub struct ClickableComponent {
    event: UserEvent,
}

impl ClickableComponent {

    pub fn new(event: UserEvent) -> Self {
        return Self { event };
    }

    pub fn click(&self) -> UserEvent {
        return self.event;
    }
}
