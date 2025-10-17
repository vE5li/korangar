use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::{ItemName, ItemResource, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub(super) identified_name: ItemName,
    pub(super) unidentified_name: ItemName,
    pub(super) identified_resource: ItemResource,
    pub(super) unidentified_resource: ItemResource,
}

impl Table for ItemInfo {
    type Key<'a> = ItemId;
    type Storage = HashMap<ItemId, ItemInfo>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\iteminfo.lub")
            .unwrap();
        state.load(&data).exec()?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("tbl") {
            for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
                let info = ItemInfo {
                    identified_name: ItemName::from_option(item_table.get("identifiedDisplayName").ok().map(fix_encoding)),
                    unidentified_name: ItemName::from_option(item_table.get("unidentifiedDisplayName").ok().map(fix_encoding)),
                    identified_resource: ItemResource::from_option(item_table.get("identifiedResourceName").ok().map(fix_encoding)),
                    unidentified_resource: ItemResource::from_option(item_table.get("unidentifiedResourceName").ok().map(fix_encoding)),
                };

                result.insert(ItemId(item_id), info);
            }
        }

        let compacted = HashMap::from_iter(result);

        Ok(compacted)
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.item_info_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: ItemInfo = ItemInfo {
            identified_name: ItemName::not_found_value(),
            unidentified_name: ItemName::not_found_value(),
            identified_resource: ItemResource::not_found_value(),
            unidentified_resource: ItemResource::not_found_value(),
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
