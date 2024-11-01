use std::sync::Arc;

use korangar_networking::{InventoryItem, NoMetadata, ShopItem};
use korangar_util::FileLoader;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::TextureLoader;
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    pub texture: Arc<Texture>,
    pub name: String,
}

pub struct ScriptLoader {
    state: Lua,
}

impl ScriptLoader {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\jobidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\iteminfo.lub")
            .unwrap();
        state.load(&data).exec()?;

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\npcidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let job_id_function = r#"
function get_job_name_from_id(id)
  for k,v in pairs(JTtbl) do
    if v==id then

        if string.sub(k, 1, 5) == "JT_G_" then
            return string.sub(k, 6)
        end

        if string.sub(k, 1, 6) == "JT_C1_" then
            return string.sub(k, 7)
        end

        if string.sub(k, 1, 6) == "JT_C5_" then
            return string.sub(k, 7)
        end

        return string.sub(k, 4)
    end
  end

  for k,v in pairs(jobtbl) do
    if v==id then
        if string.sub(k, 1, 5) == "JT_G_" then
            return string.sub(k, 6)
        end
        return string.sub(k, 4)
    end
  end

  return "1_f_maria"
  --return nil
end
"#;

        state.load(job_id_function).exec()?;

        Ok(Self { state })
    }

    // TODO: move this to a different class that utilizes the script loader
    pub fn get_job_name_from_id(&self, job_id: usize) -> String {
        use mlua::prelude::*;
        use mlua::Function;

        let globals = self.state.globals();

        let print: Function = globals.get("get_job_name_from_id").unwrap();
        print
            .call::<LuaString>(job_id)
            .unwrap()
            .to_str()
            .unwrap()
            .replace("CHONCHON", "chocho") // TODO: find a way to do this properly
    }

    // TODO: move this to a different class that utilizes the script loader
    fn get_item_name_from_id(&self, item_id: ItemId, is_identified: bool) -> String {
        use mlua::prelude::*;

        let globals = self.state.globals();
        let lookup_name = match is_identified {
            true => "identifiedDisplayName",
            false => "unidentifiedDisplayName",
        };

        globals
            .get::<LuaTable>("tbl")
            .unwrap()
            .get::<LuaTable>(item_id.0)
            .map(|table| table.get::<LuaString>(lookup_name).unwrap().to_str().unwrap().to_owned())
            .unwrap_or_else(|_| "NOTFOUND".to_owned())
    }

    // TODO: move this to a different class that utilizes the script loader
    fn get_item_resource_from_id(&self, item_id: ItemId, is_identified: bool) -> String {
        use mlua::prelude::*;

        let globals = self.state.globals();
        let lookup_name = match is_identified {
            true => "identifiedResourceName",
            false => "undidentifiedResourceName",
        };

        globals
            .get::<LuaTable>("tbl")
            .unwrap()
            .get::<LuaTable>(item_id.0)
            .map(|table| table.get::<LuaString>(lookup_name).unwrap().to_str().unwrap().to_owned())
            .unwrap_or_else(|_| "»ç°ú".to_owned())
    }

    pub fn load_inventory_item_metadata(
        &self,
        texture_loader: &TextureLoader,
        item: InventoryItem<NoMetadata>,
    ) -> InventoryItem<ResourceMetadata> {
        let is_identified = item.is_identifed();

        let resource_name = self.get_item_resource_from_id(item.item_id, is_identified);
        let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{resource_name}.bmp");
        let texture = texture_loader.get(&full_path).unwrap();
        let name = self.get_item_name_from_id(item.item_id, is_identified);

        let metadata = ResourceMetadata { texture, name };

        InventoryItem { metadata, ..item }
    }

    pub fn load_market_item_metadata(&self, texture_loader: &TextureLoader, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
        let resource_name = self.get_item_resource_from_id(item.item_id, true);
        let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{resource_name}.bmp");
        let texture = texture_loader.get(&full_path).unwrap();
        let name = self.get_item_name_from_id(item.item_id, true);

        let metadata = ResourceMetadata { texture, name };

        ShopItem { metadata, ..item }
    }
}
