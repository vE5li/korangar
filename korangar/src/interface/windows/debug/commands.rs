use korangar_interface::elements::{ButtonBuilder, ElementWrap, InputFieldBuilder, Text};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainTrackedState, TrackedState, TrackedStateExt, ValueState};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};
use rust_state::{Context, SafeUnwrap, Selector};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

pub struct CommandsWindow<InputSelector> {
    input_selector: InputSelector,
}

impl<InputSelector> CommandsWindow<InputSelector> {
    pub const WINDOW_CLASS: &'static str = "commands";

    pub fn new(input_selector: InputSelector) -> Self {
        Self { input_selector }
    }
}

impl<InputSelector> PrototypeWindow<GameState> for CommandsWindow<InputSelector>
where
    InputSelector: for<'a> Selector<'a, GameState, String> + SafeUnwrap,
{
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let class_action = {
            let mut input_selector = self.input_selector.clone();

            Box::new(move |state: &Context<GameState>| {
                let input = state.get_safe(&input_selector);

                if input.is_empty() {
                    return Vec::new();
                };

                let message = format!("@jobchange {input}");

                state.update_value(&input_selector, String::new());

                vec![ClickAction::Custom(UserEvent::SendMessage(message))]
            })
        };

        let change_action = {
            let mut input_selector = self.input_selector.clone();

            move |state: &Context<GameState>| {
                let input = state.get_safe(&input_selector);
                let message = format!("@jobchange {input}");

                state.update_value(&input_selector, String::new());

                vec![ClickAction::Custom(UserEvent::SendMessage(message))]
            }
        };

        let elements = vec![
            Text::default().with_text("change job").wrap(),
            InputFieldBuilder::new()
                .with_state(self.input_selector.clone())
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
