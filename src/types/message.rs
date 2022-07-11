use derive_new::new;

use crate::graphics::Color;

#[derive(new)]
pub struct ChatMessage {
    pub text: String,
    pub color: Color,
}
