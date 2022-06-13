use derive_new::new;
use serde::{ Serialize, Deserialize };

use interface::types::{ Position, Size };

#[derive(Serialize, Deserialize, new)]
pub struct WindowState {
    pub position: Position,
    pub size: Size,
}
