use std::sync::Arc;

use encoding_rs::EUC_KR;
use hashbrown::HashMap;
use korangar_networking::{InventoryItem, NoMetadata, ShopItem};
use korangar_util::FileLoader;
use mlua::Lua;
use ragnarok_packets::ItemId;

use crate::graphics::Texture;
use crate::loaders::{AsyncLoader, GameFileLoader, ImageType, ItemLocation};

#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    pub texture: Option<Arc<Texture>>,
    pub name: String,
}

#[derive(Debug, Clone)]
struct ItemInfo {
    identified_name: Option<String>,
    unidentified_name: Option<String>,
    identified_resource: Option<String>,
    unidentified_resource: Option<String>,
}

pub struct Library {
    job_identity_table: HashMap<usize, String>,
    item_table: HashMap<ItemId, ItemInfo>,
}

impl Library {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\jobidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\npcidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let job_identity_table = Self::load_job_identity_table(&state)?;

        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\iteminfo.lub")
            .unwrap();
        state.load(&data).exec()?;

        let item_table = Self::load_item_table(&state)?;

        Ok(Self {
            job_identity_table,
            item_table,
        })
    }

    pub fn load_job_identity_table(state: &Lua) -> mlua::Result<HashMap<usize, String>> {
        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(jobtbl) = globals.get::<mlua::Table>("jobtbl") {
            for (key, value) in jobtbl.pairs::<String, usize>().flatten() {
                let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
                    end.to_string()
                } else {
                    key[3..].to_string()
                };

                result.insert(value, cleaned_key);
            }
        }

        if let Ok(jttbl) = globals.get::<mlua::Table>("JTtbl") {
            for (key, value) in jttbl.pairs::<String, usize>().flatten() {
                let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
                    end.to_string()
                } else if key.starts_with("JT_C1_")
                    || key.starts_with("JT_C2_")
                    || key.starts_with("JT_C3_")
                    || key.starts_with("JT_C4_")
                    || key.starts_with("JT_C5_")
                {
                    key[6..].to_string()
                } else {
                    key[3..].to_string()
                };

                let cleaned_key = cleaned_key.replace("CHONCHON", "chocho");

                result.insert(value, cleaned_key);
            }
        }

        let compacted = HashMap::from_iter(result);

        Ok(compacted)
    }

    fn load_item_table(state: &Lua) -> mlua::Result<HashMap<ItemId, ItemInfo>> {
        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("tbl") {
            for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
                let info = ItemInfo {
                    identified_name: item_table.get("identifiedDisplayName").ok().map(fix_encoding),
                    unidentified_name: item_table.get("unidentifiedDisplayName").ok().map(fix_encoding),
                    identified_resource: item_table.get("identifiedResourceName").ok().map(fix_encoding),
                    unidentified_resource: item_table.get("unidentifiedResourceName").ok().map(fix_encoding),
                };

                result.insert(ItemId(item_id), info);
            }
        }

        let compacted = HashMap::from_iter(result);

        Ok(compacted)
    }

    pub fn get_job_identity_from_id(&self, job_id: usize) -> &str {
        self.job_identity_table
            .get(&job_id)
            .map(|name| name.as_str())
            .unwrap_or("1_f_maria")
    }

    fn get_item_name_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
        match is_identified {
            true => self.item_table.get(&item_id).and_then(|info| info.identified_name.as_deref()),
            false => self.item_table.get(&item_id).and_then(|info| info.unidentified_name.as_deref()),
        }
        .unwrap_or("NOTFOUND")
    }

    fn get_item_resource_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
        match is_identified {
            true => self.item_table.get(&item_id).and_then(|info| info.identified_resource.as_deref()),
            false => self.item_table.get(&item_id).and_then(|info| info.unidentified_resource.as_deref()),
        }
        .unwrap_or("사과") // Apple
    }

    pub fn load_inventory_item_metadata(
        &self,
        async_loader: &AsyncLoader,
        item: InventoryItem<NoMetadata>,
    ) -> InventoryItem<ResourceMetadata> {
        let is_identified = item.is_identified();

        let resource_name = self.get_item_resource_from_id(item.item_id, is_identified);
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = async_loader.request_item_sprite_load(ItemLocation::Inventory, item.item_id, &full_path, ImageType::Color);
        let name = self.get_item_name_from_id(item.item_id, is_identified).to_string();

        let metadata = ResourceMetadata { texture, name };

        InventoryItem { metadata, ..item }
    }

    pub fn load_shop_item_metadata(&self, async_loader: &AsyncLoader, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
        let resource_name = self.get_item_resource_from_id(item.item_id, true);
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = async_loader.request_item_sprite_load(ItemLocation::Shop, item.item_id, &full_path, ImageType::Color);
        let name = self.get_item_name_from_id(item.item_id, true).to_string();

        let metadata = ResourceMetadata { texture, name };

        ShopItem { metadata, ..item }
    }
}

fn fix_encoding(broken: String) -> String {
    let bytes: Vec<u8> = broken.chars().map(|char| char as u8).collect();
    match EUC_KR.decode_without_bom_handling_and_without_replacement(&bytes) {
        None => broken.to_string(),
        Some(char) => char.to_string(),
    }
}
