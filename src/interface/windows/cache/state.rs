use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::{Position, Size};

#[derive(Serialize, Deserialize, new)]
pub struct WindowState {
    pub position: Position,
    pub size: Size,
}
