use std::cell::RefCell;
use std::rc::Rc;

use korangar_interface::builder::Unset;
use korangar_interface::state::PlainRemote;

use super::Chat;
use crate::loaders::FontLoader;
use crate::network::ChatMessage;

/// Type state [`Chat`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct ChatBuilder<Messages, Font> {
    messages: Messages,
    font_loader: Font,
}

impl ChatBuilder<Unset, Unset> {
    pub fn new() -> Self {
        Self {
            messages: Unset,
            font_loader: Unset,
        }
    }
}

impl<Font> ChatBuilder<Unset, Font> {
    pub fn with_messages(self, messages: PlainRemote<Vec<ChatMessage>>) -> ChatBuilder<PlainRemote<Vec<ChatMessage>>, Font> {
        ChatBuilder { messages, ..self }
    }
}

impl<Messages> ChatBuilder<Messages, Unset> {
    pub fn with_font_loader(self, font_loader: Rc<RefCell<FontLoader>>) -> ChatBuilder<Messages, Rc<RefCell<FontLoader>>> {
        ChatBuilder { font_loader, ..self }
    }
}

impl ChatBuilder<PlainRemote<Vec<ChatMessage>>, Rc<RefCell<FontLoader>>> {
    /// Take the builder and turn it into a [`Chat`].
    ///
    /// NOTE: This method is only available if
    /// [`with_messages`](Self::with_messages)
    /// and [`with_font_loader`](Self::with_font_loader) have been called on
    /// the builder.
    pub fn build(self) -> Chat {
        let Self { messages, font_loader } = self;

        Chat {
            messages,
            font_loader,
            stamp: true,
            state: Default::default(),
        }
    }
}
