use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::{HashMapExt, ItemName, ItemResource, Library, LuaExt, Table, game_string};
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
    type Storage = HashMap<ItemId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\iteminfo.lub"])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("tbl") {
            for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
                let info = ItemInfo {
                    identified_name: ItemName::from_option(
                        item_table
                            .get::<Option<mlua::String>>("identifiedDisplayName")?
                            .map(game_string)
                            .transpose()?,
                    ),
                    unidentified_name: ItemName::from_option(
                        item_table
                            .get::<Option<mlua::String>>("unidentifiedDisplayName")?
                            .map(game_string)
                            .transpose()?,
                    ),
                    identified_resource: ItemResource::from_option(
                        item_table
                            .get::<Option<mlua::String>>("identifiedResourceName")?
                            .map(game_string)
                            .transpose()?,
                    ),
                    unidentified_resource: ItemResource::from_option(
                        item_table
                            .get::<Option<mlua::String>>("unidentifiedResourceName")?
                            .map(game_string)
                            .transpose()?,
                    ),
                };

                result.insert(ItemId(item_id), info);
            }
        }

        Ok(result.compact())
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
