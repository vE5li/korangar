use std::cell::RefCell;

use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(Default)]
pub struct CommandsWindow {}

impl CommandsWindow {
    pub const WINDOW_CLASS: &'static str = "commands";
}

impl PrototypeWindow for CommandsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let input_text = Rc::new(RefCell::new(String::new()));

        let class_action = {
            let input_text = input_text.clone();
            Box::new(move || {
                let mut text = input_text.borrow_mut();

                if text.is_empty() {
                    return Vec::new();
                }

                let message = format!("@jobchange {text}");
                text.clear();

                vec![ClickAction::Event(UserEvent::SendMessage(message))]
            })
        };

        let change_action = {
            let input_text = input_text.clone();

            move || {
                let mut text = input_text.borrow_mut();
                let message = format!("@jobchange {text}");
                text.clear();
                vec![ClickAction::Event(UserEvent::SendMessage(message))]
            }
        };

        let elements = vec![
            Text::default().with_text("change job").wrap(),
            InputField::<30>::new(input_text, "job name or job ID", class_action, dimension!(75%)).wrap(),
            Button::default()
                .with_text("Change")
                .with_width(dimension!(25%))
                .with_event(Box::new(change_action))
                .wrap(),
            Text::default().with_text("Base level").wrap(),
            Button::default()
                .with_text("+1")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@blvl 1".to_string()))
                .wrap(),
            Button::default()
                .with_text("+5")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@blvl 5".to_string()))
                .wrap(),
            Button::default()
                .with_text("+10")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@blvl 10".to_string()))
                .wrap(),
            Button::default()
                .with_text("MAX")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@blvl 9999".to_string()))
                .wrap(),
            Text::default().with_text("Job level").wrap(),
            Button::default()
                .with_text("+1")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 1".to_string()))
                .wrap(),
            Button::default()
                .with_text("+5")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 5".to_string()))
                .wrap(),
            Button::default()
                .with_text("+10")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 10".to_string()))
                .wrap(),
            Button::default()
                .with_text("MAX")
                .with_width(dimension!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 9999".to_string()))
                .wrap(),
            Text::default().with_text("Stats").wrap(),
            Button::default()
                .with_text("Set all stats to max")
                .with_event(UserEvent::SendMessage("@allstats".to_string()))
                .wrap(),
            Text::default().with_text("Skills").wrap(),
            Button::default()
                .with_text("Unlock all skills")
                .with_event(UserEvent::SendMessage("@allskill".to_string()))
                .wrap(),
            Text::default().with_text("Player state").wrap(),
            Button::default()
                .with_text("Mount")
                .with_event(UserEvent::SendMessage("@mount".to_string()))
                .wrap(),
            Button::default()
                .with_text("Heal")
                .with_event(UserEvent::SendMessage("@heal".to_string()))
                .wrap(),
            Button::default()
                .with_text("Fill AP")
                .with_event(UserEvent::SendMessage("@healap".to_string()))
                .wrap(),
            Button::default()
                .with_text("Resurrect")
                .with_event(UserEvent::SendMessage("@alive".to_string()))
                .wrap(),
        ];

        WindowBuilder::default()
            .with_title("Commands".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(SizeConstraint::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
