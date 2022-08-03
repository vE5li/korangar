use crate::interface::ElementCell;
use crate::interface::elements::*;

#[derive(Clone, PartialEq)]
pub enum DialogElement {
    Text(String),
    NextButton,
    CloseButton,
    ChoiceButton(String, i8),
}
