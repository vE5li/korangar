use korangar_interface::elements::{ButtonBuilder, ElementWrap, InputFieldBuilder, Text};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainTrackedState, TrackedState, TrackedStateExt, ValueState};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct CommandsWindow;

impl CommandsWindow {
    pub const WINDOW_CLASS: &'static str = "commands";
}

impl PrototypeWindow<InterfaceSettings> for CommandsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let input_text = PlainTrackedState::<String>::default();

        let class_action = {
            let mut input_text = input_text.clone();

            Box::new(move || {
                let message = input_text.with_mut(|text| {
                    if text.is_empty() {
                        return ValueState::Unchanged(None);
                    }

                    let message = format!("@jobchange {text}");
                    text.clear();
                    ValueState::Mutated(Some(message))
                });

                let Some(message) = message else {
                    return Vec::new();
                };

                vec![ClickAction::Custom(UserEvent::SendMessage(message))]
            })
        };

        let change_action = {
            let mut input_text = input_text.clone();

            move || {
                let message = input_text.mutate(|text| {
                    let message = format!("@jobchange {text}");
                    text.clear();
                    message
                });

                vec![ClickAction::Custom(UserEvent::SendMessage(message))]
            }
        };

        let elements = vec![
            Text::default().with_text("change job").wrap(),
            InputFieldBuilder::new()
                .with_state(input_text)
                .with_ghost_text("Job name or job ID")
                .with_enter_action(class_action)
                .with_length(30)
                .with_width_bound(dimension_bound!(75%))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Change")
                .with_width_bound(dimension_bound!(25%))
                .with_event(Box::new(change_action))
                .build()
                .wrap(),
            Text::default().with_text("Base level").wrap(),
            ButtonBuilder::new()
                .with_text("+1")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@blvl 1".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("+5")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@blvl 5".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("+10")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@blvl 10".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("MAX")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@blvl 9999".to_string()))
                .build()
                .wrap(),
            Text::default().with_text("Job level").wrap(),
            ButtonBuilder::new()
                .with_text("+1")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 1".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("+5")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 5".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("+10")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 10".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("MAX")
                .with_width_bound(dimension_bound!(25%))
                .with_event(UserEvent::SendMessage("@jlvl 9999".to_string()))
                .build()
                .wrap(),
            Text::default().with_text("Stats").wrap(),
            ButtonBuilder::new()
                .with_text("Set all stats to max")
                .with_event(UserEvent::SendMessage("@allstats".to_string()))
                .build()
                .wrap(),
            Text::default().with_text("Skills").wrap(),
            ButtonBuilder::new()
                .with_text("Unlock all skills")
                .with_event(UserEvent::SendMessage("@allskill".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Give 10,000 Zeny")
                .with_event(UserEvent::SendMessage("@zeny 10000".to_string()))
                .build()
                .wrap(),
            Text::default().with_text("Player state").wrap(),
            ButtonBuilder::new()
                .with_text("Mount")
                .with_event(UserEvent::SendMessage("@mount".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Heal")
                .with_event(UserEvent::SendMessage("@heal".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Fill AP")
                .with_event(UserEvent::SendMessage("@healap".to_string()))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Resurrect")
                .with_event(UserEvent::SendMessage("@alive".to_string()))
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Commands".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
