use std::collections::HashMap;

use crate::element::id::ElementIdGenerator;
use crate::element::store::{ElementStoreMut, InternalElementStore};

#[derive(Default)]
pub(crate) struct WindowStore {
    // The element stores need to be in a Box so that we can safely pass out references
    // to them without worrying about relocation of the hashmap when inserting new children.
    stores: HashMap<u64, Box<InternalElementStore>>,
}

impl WindowStore {
    pub fn get_or_create_from_window_id<'a>(&'a mut self, window_id: u64, generator: &'a mut ElementIdGenerator) -> ElementStoreMut<'a> {
        let element_store = self
            .stores
            .entry(window_id)
            .or_insert_with(|| Box::new(InternalElementStore::root(generator)));

        ElementStoreMut::new(element_store, generator, window_id)
    }

    pub(crate) fn get_from_window_id(&self, window_id: u64) -> &InternalElementStore {
        self.stores.get(&window_id).expect("This shouldn't happen")
    }
}
