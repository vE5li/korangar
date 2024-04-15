use derive_new::new;
use korangar_interface::state::{PlainTrackedState, TrackedStateExt, TrackedStateVec};

use super::elements::DialogElement;
use super::windows::DialogWindow;
use crate::network::EntityId;

#[derive(new)]
struct DialogHandle {
    elements: PlainTrackedState<Vec<DialogElement>>,
    clear: bool,
}

#[derive(Default)]
pub struct DialogSystem {
    dialog_handle: Option<DialogHandle>,
}

impl DialogSystem {
    #[korangar_procedural::profile]
    pub fn open_dialog_window(&mut self, text: String, npc_id: EntityId) -> Option<DialogWindow> {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.mutate(|elements| {
                if dialog_handle.clear {
                    elements.clear();
                    dialog_handle.clear = false;
                }

                elements.push(DialogElement::Text(text));
            });

            None
        } else {
            let (window, elements) = DialogWindow::new(text, npc_id);
            self.dialog_handle = Some(DialogHandle::new(elements, false));

            Some(window)
        }
    }

    #[korangar_procedural::profile]
    pub fn add_next_button(&mut self) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.push(DialogElement::NextButton);
            dialog_handle.clear = true;
        }
    }

    #[korangar_procedural::profile]
    pub fn add_close_button(&mut self) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.mutate(|elements| {
                elements.retain(|element| *element != DialogElement::NextButton);
                elements.push(DialogElement::CloseButton);
            });
        }
    }

    #[korangar_procedural::profile]
    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.mutate(move |elements| {
                elements.retain(|element| *element != DialogElement::NextButton);

                choices
                    .into_iter()
                    .enumerate()
                    .for_each(|(index, choice)| elements.push(DialogElement::ChoiceButton(choice, index as i8 + 1)));
            });
        }
    }

    #[korangar_procedural::profile]
    pub fn close_dialog(&mut self) {
        self.dialog_handle = None;
    }
}
