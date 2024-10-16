use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::Anchor;
use crate::application::Appli;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct WindowState<App: Appli> {
    anchor: Anchor<App>,
    size: App::Size,
}

impl<App: Appli> WindowState<App> {
    fn new(anchor: Anchor<App>, size: App::Size) -> Self {
        Self { anchor, size }
    }
}

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WindowCache<App: Appli> {
    entries: HashMap<String, WindowState<App>>,
}

impl<App: Appli> WindowCache<App> {
    pub(crate) fn register_window(&mut self, identifier: &str, anchor: Anchor<App>, size: App::Size) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.anchor = anchor;
            entry.size = size;
        } else {
            let entry = WindowState::new(anchor, size);
            self.entries.insert(identifier.to_string(), entry);
        }
    }

    pub(crate) fn update_anchor(&mut self, identifier: &str, anchor: Anchor<App>) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.anchor = anchor;
        }
    }

    pub(crate) fn update_size(&mut self, identifier: &str, size: App::Size) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.size = size;
        }
    }

    pub(crate) fn get_window_state(&self, identifier: &str) -> Option<(Anchor<App>, App::Size)> {
        self.entries.get(identifier).map(|entry| (entry.anchor.clone(), entry.size))
    }
}
