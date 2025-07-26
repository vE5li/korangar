use std::collections::HashMap;

use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;

#[derive(Default)]
pub struct WindowStore {
    // The element stores need to be in a Box so that we can safely pass out references
    // to them without worrying about relocation of the hashmap when inserting new children.
    stores: HashMap<u64, Box<ElementStore>>,
}

impl WindowStore {
    pub fn get_or_create_from_window_id(&mut self, window_id: u64, generator: &mut ElementIdGenerator) -> &mut ElementStore {
        self.stores
            .entry(window_id)
            .or_insert_with(|| Box::new(ElementStore::root(generator)))
    }

    pub fn get_from_window_id(&self, window_id: u64) -> &ElementStore {
        self.stores.get(&window_id).expect("This shouldn't happen")
    }
}
