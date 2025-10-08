mod item_info;
mod item_name;
mod item_resource;
mod job_identity;
mod map_sky_data;
use std::hash::Hash;
use std::sync::Arc;

use encoding_rs::EUC_KR;

pub use self::item_info::ItemInfo;
pub use self::item_name::{ItemName, ItemNameKey};
pub use self::item_resource::{ItemResource, ItemResourceKey};
pub use self::job_identity::JobIdentity;
pub use self::map_sky_data::MapSkyData;
use crate::loaders::GameFileLoader;
use crate::graphics::{Color, Texture};
use crate::inventory::LearnableSkill;
use crate::loaders::{ActionLoader, AsyncLoader, GameFileLoader, ImageType, ItemLocation, SpriteLoader};

trait HashMapExt {
    fn compact(self) -> Self;
}

impl<K, V> HashMapExt for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn compact(self) -> Self {
        HashMap::from_iter(self)
    }
}

pub struct SkillListEntry {
    pub file_name: String,
    pub name: String,
    pub maximum_level: SkillLevel,
    // TODO: Remove allow
    #[allow(dead_code)]
    pub generic_required_skills: HashMap<SkillId, SkillLevel>,
    // TODO: Remove allow
    #[allow(dead_code)]
    pub job_required_skills: HashMap<JobId, HashMap<SkillId, SkillLevel>>,
    // TODO: Additional fields.
}

pub struct Library {
    job_identity_table: <JobIdentity as Table>::Storage,
    item_info_table: <ItemInfo as Table>::Storage,
    map_sky_data_table: <MapSkyData as Table>::Storage,

    // item_table: HashMap<ItemId, ItemInfo>,
    // skill_list_table: HashMap<SkillId, SkillListEntry>,
    // skill_tree_table: HashMap<JobId, HashMap<usize, SkillId>>,
}

impl Library {
    fn load_state(game_file_loader: &GameFileLoader, files: &[&str]) -> mlua::Result<Lua> {
        let state = Lua::new();

        for file in files {
            // TODO: Handle this better.
            let data = game_file_loader.get(file).unwrap();
            state.load(&data).exec()?;
        }

        Ok(state)
    }

    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let job_identity_table = JobIdentity::load(game_file_loader)?;
        let item_info_table = ItemInfo::load(game_file_loader)?;
        let map_sky_data_table = MapSkyData::load(game_file_loader)?;

        Ok(Self {
            job_identity_table,
            item_info_table,
            map_sky_data_table,
        })

        // let state = Self::load_state(game_file_loader, &[
        //     "data\\luafiles514\\lua files\\datainfo\\jobidentity.lub",
        //     "data\\luafiles514\\lua files\\datainfo\\npcidentity.lub",
        // ])?;
        // let job_identity_table = Self::load_job_identity_table(state)?;
        //
        // let state = Self::load_state(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\iteminfo.lub"])?;
        // let item_table = Self::load_item_table(state)?;
        //
        // let state = Self::load_state(game_file_loader, &[
        //     // Needed to get the `LOBID` table.
        //     "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
        //     // Needed to get the `SKID` table.
        //     "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
        //     "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        // ])?;
        // let skill_list_table = Self::load_skill_list_table(state)?;
        //
        // let state = Self::load_state(game_file_loader, &[
        //     // Needed to get the `LOBID` table.
        //     "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
        //     // Needed to get the `SKID` table.
        //     "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
        //     // Needed to get the `JobSkillTable.ChangeSkillTabName` function.
        //     "data\\luafiles514\\lua files\\skillinfoz\\skillinfo_f.lub",
        //     "data\\luafiles514\\lua files\\skillinfoz\\skilltreeview.lub",
        //     // "data\\luafiles514\\lua files\\skillinfoz\\skilltreeview 20180621.lub",
        // ])?;
        // let skill_tree_table = Self::load_skill_tree_table(state)?;
        //
        // let map_sky_data_table = match Self::load_state(game_file_loader, &["data\\luafiles514\\lua files\\mapskydata\\mapskydata.lub"]) {
        //     Ok(state) => Self::load_map_sky_data_table(state)?,
        //     Err(_) => HashMap::new(),
        // };
        //
        // Ok(Self {
        //     job_identity_table,
        //     item_table,
        //     skill_list_table,
        //     skill_tree_table,
    }

    #[inline(always)]
    pub fn get<T: Table>(&self, key: T::Key<'_>) -> &T {
        T::get(self, key)
    }
}
    // pub fn load_job_identity_table(state: Lua) -> mlua::Result<HashMap<JobId, String>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(jobtbl) = globals.get::<mlua::Table>("jobtbl") {
    //         for (key, value) in jobtbl.pairs::<String, u16>().flatten() {
    //             let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
    //                 end.to_string()
    //             } else {
    //                 key[3..].to_string()
    //             };
    //
    //             result.insert(JobId(value), cleaned_key);
    //         }
    //     }
    //
    //     if let Ok(jttbl) = globals.get::<mlua::Table>("JTtbl") {
    //         for (key, value) in jttbl.pairs::<String, u16>().flatten() {
    //             let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
    //                 end.to_string()
    //             } else if key.starts_with("JT_C1_")
    //                 || key.starts_with("JT_C2_")
    //                 || key.starts_with("JT_C3_")
    //                 || key.starts_with("JT_C4_")
    //                 || key.starts_with("JT_C5_")
    //             {
    //                 key[6..].to_string()
    //             } else {
    //                 key[3..].to_string()
    //             };
    //
    //             let cleaned_key = cleaned_key.replace("CHONCHON", "chocho");
    //
    //             result.insert(JobId(value), cleaned_key);
    //         }
    //     }
    //
    //     Ok(result.compact())

    // pub fn load_job_identity_table(state: Lua) -> mlua::Result<HashMap<JobId, String>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(jobtbl) = globals.get::<mlua::Table>("jobtbl") {
    //         for (key, value) in jobtbl.pairs::<String, u16>().flatten() {
    //             let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
    //                 end.to_string()
    //             } else {
    //                 key[3..].to_string()
    //             };
    //
    //             result.insert(JobId(value), cleaned_key);
    //         }
    //     }
    //
    //     if let Ok(jttbl) = globals.get::<mlua::Table>("JTtbl") {
    //         for (key, value) in jttbl.pairs::<String, u16>().flatten() {
    //             let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
    //                 end.to_string()
    //             } else if key.starts_with("JT_C1_")
    //                 || key.starts_with("JT_C2_")
    //                 || key.starts_with("JT_C3_")
    //                 || key.starts_with("JT_C4_")
    //                 || key.starts_with("JT_C5_")
    //             {
    //                 key[6..].to_string()
    //             } else {
    //                 key[3..].to_string()
    //             };
    //
    //             let cleaned_key = cleaned_key.replace("CHONCHON", "chocho");
    //
    //             result.insert(JobId(value), cleaned_key);
    //         }
    //     }
    //
    //     let compacted = HashMap::from_iter(result);
    //
    //     Ok(compacted)
    // }
    
    // fn load_item_table(state: Lua) -> mlua::Result<HashMap<ItemId, ItemInfo>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //             let compacted = HashMap::from_iter(result);
    //     Ok(compacted)
    // }
    //
    // fn load_skill_list_table(state: Lua) -> mlua::Result<HashMap<SkillId, SkillListEntry>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("SKILL_INFO_LIST") {
    //         for (skill_id, table) in table.pairs::<u16, mlua::Table>().flatten() {
    //             let file_name = table.get(1)?;
    //             let name = table.get("SkillName")?;
    //             let maximum_level = table.get("MaxLv")?;
    //
    //             let generic_required_skills = match table.get::<mlua::Table>("_NeedSkillList") {
    //                 Ok(sequence) => {
    //                     let mut required_skills = HashMap::new();
    //
    //                     for required_skill in sequence.sequence_values::<mlua::Table>() {
    //                         let required_skill = required_skill?;
    //                         let skill_id = required_skill.get::<u16>(1)?;
    //                         // TODO: At least one skill does not have a level set. I am just
    //                         // assuming that that means level 1 is required but should be
    //                         // confirmed.
    //                         let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
    //                         required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
    //                     }
    //
    //                     required_skills
    //                 }
    //                 Err(..) => HashMap::new(),
    //             };
    //
    //             let job_required_skills = match table.get::<mlua::Table>("NeedSkillList") {
    //                 Ok(table) => {
    //                     let mut job_specific = HashMap::new();
    //
    //                     for (job_id, sequence) in table.pairs::<u16, mlua::Table>().flatten() {
    //                         let mut required_skills = HashMap::new();
    //
    //                         for required_skill in sequence.sequence_values::<mlua::Table>() {
    //                             let required_skill = required_skill?;
    //                             let skill_id = required_skill.get::<u16>(1)?;
    //                             // TODO: At least one skill does not have a level set. I am just
    //                             // assuming that that means level 1 is required but should be
    //                             // confirmed.
    //                             let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
    //                             required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
    //                         }
    //
    //                         job_specific.insert(JobId(job_id), required_skills);
    //                     }
    //
    //                     job_specific
    //                 }
    //                 Err(..) => HashMap::new(),
    //             };
    //
    //             result.insert(SkillId(skill_id), SkillListEntry {
    //                 file_name,
    //                 name,
    //                 maximum_level: SkillLevel(maximum_level),
    //                 generic_required_skills,
    //                 job_required_skills,
    //             });
    //         }
    //     }
    //
    //     let compacted = HashMap::from_iter(result);
    //
    //     Ok(compacted)
    // }
    //
    // fn load_skill_tree_table(state: Lua) -> mlua::Result<HashMap<JobId, HashMap<usize, SkillId>>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("SKILL_TREEVIEW_FOR_JOB") {
    //         for (job_id, view_table) in table.pairs::<u16, mlua::Table>().flatten() {
    //             let mut view_result = HashMap::new();
    //
    //             for (slot, skill_id) in view_table.pairs::<usize, u16>().flatten() {
    //                 view_result.insert(slot, SkillId(skill_id));
    //             }
    //
    //             let compacted = HashMap::from_iter(view_result);
    //             result.insert(JobId(job_id), compacted);
    //         }
    //     }
    //
    //     let compacted = HashMap::from_iter(result);
    //
    //     Ok(compacted)
    // }
    //
    // fn load_map_sky_data_table(state: Lua) -> mlua::Result<HashMap<String, MapSkyData>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("MapSkyData") {
    //         for (map_rsw, map_sky_data_table) in table.pairs::<String, mlua::Table>().flatten() {
    //             let resource_name = map_rsw.strip_suffix(".rsw").unwrap_or(&map_rsw).to_string();
    //             let map_sky_data = Self::parse_map_sky_data(&map_sky_data_table);
    //             result.insert(resource_name, map_sky_data);
    //         }
    //     }
    //
    //     let compacted = HashMap::from_iter(result);
    //
    //     Ok(compacted)
    // }
    //
    // fn parse_map_sky_data(table: &mlua::Table) -> MapSkyData {
    //     let mut cloud_effect = Vec::new();
    //
    //     if let Ok(Value::Table(cloud_effect_table)) = table.get("Cloud_Effect") {
    //         let _ = cloud_effect_table.for_each(|_key: usize, value: mlua::Table| {
    //             let effect = CloudEffect {
    //                 num: value.get("Num").unwrap_or_default(),
    //                 cull_dist: value.get("CullDist").unwrap_or_default(),
    //                 color: value.get("Color").unwrap_or_default(),
    //                 size: value.get("Size").unwrap_or_default(),
    //                 size_extra: value.get("Size_Extra").unwrap_or_default(),
    //                 expand_rate: value.get("Expand_Rate").unwrap_or_default(),
    //                 alpha_inc_time: value.get("Alpha_Inc_Time").unwrap_or_default(),
    //                 alpha_inc_time_extra: value.get("Alpha_Inc_Time_Extra").unwrap_or_default(),
    //                 alpha_inc_speed: value.get("Alpha_Inc_Speed").unwrap_or_default(),
    //                 alpha_dec_time: value.get("Alpha_Dec_Time").unwrap_or_default(),
    //                 alpha_dec_time_extra: value.get("Alpha_Dec_Time_Extra").unwrap_or_default(),
    //                 alpha_dec_speed: value.get("Alpha_Dec_Speed").unwrap_or_default(),
    //                 height: value.get("Height").unwrap_or_default(),
    //                 height_extra: value.get("Height_Extra").unwrap_or_default(),
    //             };
    //             cloud_effect.push(effect);
    //
    //             Ok(())
    //         });
    //     }
    //
    //     MapSkyData {
    //         old_cloud_effect: table.get::<[usize; 1]>("Old_Cloud_Effect").ok().map(|effect| effect[0]),
    //         bg_color: table.get("BG_Color").ok(),
    //         bg_fog: table.get("BG_Fog").unwrap_or(false),
    //         star_effect: table.get("Star_Effect").unwrap_or(false),
    //         cloud_effect,
    //     }
    // }
    //
    // pub fn get_job_identity_from_id(&self, job_id: JobId) -> &str {
    //     self.job_identity_table
    //         .get(&job_id)
    //         .map(|name| name.as_str())
    //         .unwrap_or("1_f_maria")
    // }
    //
    // fn get_item_name_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
    //     match is_identified {
    //         true => self.item_table.get(&item_id).and_then(|info| info.identified_name.as_deref()),
    //         false => self.item_table.get(&item_id).and_then(|info| info.unidentified_name.as_deref()),
    //     }
    //     .unwrap_or("NOTFOUND")
    // }
    //
    // fn get_item_resource_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
    //     match is_identified {
    //         true => self.item_table.get(&item_id).and_then(|info| info.identified_resource.as_deref()),
    //         false => self.item_table.get(&item_id).and_then(|info| info.unidentified_resource.as_deref()),
    //     }
    //     .unwrap_or("사과") // Apple
    // }
    //
    // pub fn get_skill_list_entry(&self, skill_id: SkillId) -> &SkillListEntry {
    //     // TODO: Handle this better.
    //     self.skill_list_table.get(&skill_id).unwrap()
    // }
    //
    // pub fn get_skill_tree_layout_from_job_id(
    //     &self,
    //     sprite_loader: &SpriteLoader,
    //     action_loader: &ActionLoader,
    //     job_id: JobId,
    //     client_tick: ClientTick,
    // ) -> HashMap<usize, LearnableSkill> {
    //     match self.skill_tree_table.get(&job_id) {
    //         Some(layout) => HashMap::from_iter(layout.iter().map(|(slot, skill_id)| {
    //             let skill_entry = self.get_skill_list_entry(*skill_id);
    //             let skill = LearnableSkill::load(
    //                 sprite_loader,
    //                 action_loader,
    //                 *skill_id,
    //                 skill_entry.maximum_level,
    //                 skill_entry.file_name.clone(),
    //                 skill_entry.name.clone(),
    //                 client_tick,
    //             );
    //
    //             (*slot, skill)
    //         })),
    //         // TODO: Replicate the default behavior of the lua files
    //         None => HashMap::new(),
    //     }
    // }
    //
    // pub fn get_map_sky_data_from_resource_file(&self, resource_file: &str) -> Option<&MapSkyData> {
    //     self.map_sky_data_table.get(resource_file)
    // }
    //
    // pub fn load_inventory_item_metadata(
    //     &self,
    //     async_loader: &AsyncLoader,
    //     item: InventoryItem<NoMetadata>,
    // ) -> InventoryItem<ResourceMetadata> {
    //     let is_identified = item.is_identified();
    //
    //     let resource_name = self.get_item_resource_from_id(item.item_id, is_identified);
    //     let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
    //     let texture = async_loader.request_item_sprite_load(ItemLocation::Inventory, item.item_id, &full_path, ImageType::Color);
    //     let name = self.get_item_name_from_id(item.item_id, is_identified).to_string();
    //
    //     let metadata = ResourceMetadata { texture, name };
    //
    //     InventoryItem { metadata, ..item }
    // }
    //
    // pub fn load_shop_item_metadata(&self, async_loader: &AsyncLoader, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
    //     let resource_name = self.get_item_resource_from_id(item.item_id, true);
    //     let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
    //     let texture = async_loader.request_item_sprite_load(ItemLocation::Shop, item.item_id, &full_path, ImageType::Color);
    //     let name = self.get_item_name_from_id(item.item_id, true).to_string();
    //
    //     let metadata = ResourceMetadata { texture, name };
    //
    //     ShopItem { metadata, ..item }
    // }
    //
    //     fn load_skill_list_table(state: Lua) -> mlua::Result<HashMap<SkillId, SkillListEntry>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("SKILL_INFO_LIST") {
    //         for (skill_id, table) in table.pairs::<u16, mlua::Table>().flatten() {
    //             let file_name = table.get(1)?;
    //             let name = table.get("SkillName")?;
    //             let maximum_level = table.get("MaxLv")?;
    //
    //             let generic_required_skills = match table.get::<mlua::Table>("_NeedSkillList") {
    //                 Ok(sequence) => {
    //                     let mut required_skills = HashMap::new();
    //
    //                     for required_skill in sequence.sequence_values::<mlua::Table>() {
    //                         let required_skill = required_skill?;
    //                         let skill_id = required_skill.get::<u16>(1)?;
    //                         // TODO: At least one skill does not have a level set. I am just
    //                         // assuming that that means level 1 is required but should be
    //                         // confirmed.
    //                         let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
    //                         required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
    //                     }
    //
    //                     required_skills
    //                 }
    //                 Err(..) => HashMap::new(),
    //             };
    //
    //             let job_required_skills = match table.get::<mlua::Table>("NeedSkillList") {
    //                 Ok(table) => {
    //                     let mut job_specific = HashMap::new();
    //
    //                     for (job_id, sequence) in table.pairs::<u16, mlua::Table>().flatten() {
    //                         let mut required_skills = HashMap::new();
    //
    //                         for required_skill in sequence.sequence_values::<mlua::Table>() {
    //                             let required_skill = required_skill?;
    //                             let skill_id = required_skill.get::<u16>(1)?;
    //                             // TODO: At least one skill does not have a level set. I am just
    //                             // assuming that that means level 1 is required but should be
    //                             // confirmed.
    //                             let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
    //                             required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
    //                         }
    //
    //                         job_specific.insert(JobId(job_id), required_skills);
    //                     }
    //
    //                     job_specific
    //                 }
    //                 Err(..) => HashMap::new(),
    //             };
    //
    //             result.insert(SkillId(skill_id), SkillListEntry {
    //                 file_name,
    //                 name,
    //                 maximum_level: SkillLevel(maximum_level),
    //                 generic_required_skills,
    //                 job_required_skills,
    //             });
    //         }
    //     }
    //
    //     Ok(result.compact())
    // }
    //
    // fn load_skill_tree_table(state: Lua) -> mlua::Result<HashMap<JobId, HashMap<usize, SkillId>>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("SKILL_TREEVIEW_FOR_JOB") {
    //         for (job_id, view_table) in table.pairs::<u16, mlua::Table>().flatten() {
    //             let mut view_result = HashMap::new();
    //
    //             for (slot, skill_id) in view_table.pairs::<usize, u16>().flatten() {
    //                 view_result.insert(slot, SkillId(skill_id));
    //             }
    //
    //             result.insert(JobId(job_id), view_result.compact());
    //         }
    //     }
    //
    //     Ok(result.compact())
    // }
    //
    // fn load_map_sky_data_table(state: Lua) -> mlua::Result<HashMap<String, MapSkyData>> {
    //     let globals = state.globals();
    //     let mut result = HashMap::new();
    //
    //     if let Ok(table) = globals.get::<mlua::Table>("MapSkyData") {
    //         for (map_rsw, map_sky_data_table) in table.pairs::<String, mlua::Table>().flatten() {
    //             let resource_name = map_rsw.strip_suffix(".rsw").unwrap_or(&map_rsw).to_string();
    //             let map_sky_data = Self::parse_map_sky_data(&map_sky_data_table);
    //             result.insert(resource_name, map_sky_data);
    //         }
    //     }
    //
    //     Ok(result.compact())
    // }
    //
    // fn parse_map_sky_data(table: &mlua::Table) -> MapSkyData {
    //     let mut cloud_effect = Vec::new();
    //
    //     if let Ok(Value::Table(cloud_effect_table)) = table.get("Cloud_Effect") {
    //         let _ = cloud_effect_table.for_each(|_key: usize, value: mlua::Table| {
    //             let effect = CloudEffect {
    //                 num: value.get("Num").unwrap_or_default(),
    //                 cull_dist: value.get("CullDist").unwrap_or_default(),
    //                 color: value.get("Color").unwrap_or_default(),
    //                 size: value.get("Size").unwrap_or_default(),
    //                 size_extra: value.get("Size_Extra").unwrap_or_default(),
    //                 expand_rate: value.get("Expand_Rate").unwrap_or_default(),
    //                 alpha_inc_time: value.get("Alpha_Inc_Time").unwrap_or_default(),
    //                 alpha_inc_time_extra: value.get("Alpha_Inc_Time_Extra").unwrap_or_default(),
    //                 alpha_inc_speed: value.get("Alpha_Inc_Speed").unwrap_or_default(),
    //                 alpha_dec_time: value.get("Alpha_Dec_Time").unwrap_or_default(),
    //                 alpha_dec_time_extra: value.get("Alpha_Dec_Time_Extra").unwrap_or_default(),
    //                 alpha_dec_speed: value.get("Alpha_Dec_Speed").unwrap_or_default(),
    //                 height: value.get("Height").unwrap_or_default(),
    //                 height_extra: value.get("Height_Extra").unwrap_or_default(),
    //             };
    //             cloud_effect.push(effect);
    //
    //             Ok(())
    //         });
    //     }
    //
    //     MapSkyData {
    //         old_cloud_effect: table.get::<[usize; 1]>("Old_Cloud_Effect").ok().map(|effect| effect[0]),
    //         bg_color: table.get("BG_Color").ok(),
    //         bg_fog: table.get("BG_Fog").unwrap_or(false),
    //         star_effect: table.get("Star_Effect").unwrap_or(false),
    //         cloud_effect,
    //     }
    // }
    //
    // pub fn get_job_identity_from_id(&self, job_id: JobId) -> &str {
    //     self.job_identity_table
    //         .get(&job_id)
    //         .map(|name| name.as_str())
    //         .unwrap_or("1_f_maria")
    // }
    //
    // fn get_item_name_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
    //     match is_identified {
    //         true => self.item_table.get(&item_id).and_then(|info| info.identified_name.as_deref()),
    //         false => self.item_table.get(&item_id).and_then(|info| info.unidentified_name.as_deref()),
    //     }
    //     .unwrap_or("NOTFOUND")
    // }
    //
    // fn get_item_resource_from_id(&self, item_id: ItemId, is_identified: bool) -> &str {
    //     match is_identified {
    //         true => self.item_table.get(&item_id).and_then(|info| info.identified_resource.as_deref()),
    //         false => self.item_table.get(&item_id).and_then(|info| info.unidentified_resource.as_deref()),
    //     }
    //     .unwrap_or("사과") // Apple
    // }
    //
    // pub fn get_skill_list_entry(&self, skill_id: SkillId) -> &SkillListEntry {
    //     // TODO: Handle this better.
    //     self.skill_list_table.get(&skill_id).unwrap()
    // }
    //
    // pub fn get_skill_tree_layout_from_job_id(
    //     &self,
    //     sprite_loader: &SpriteLoader,
    //     action_loader: &ActionLoader,
    //     job_id: JobId,
    //     client_tick: ClientTick,
    // ) -> HashMap<usize, LearnableSkill> {
    //     match self.skill_tree_table.get(&job_id) {
    //         Some(layout) => HashMap::from_iter(layout.iter().map(|(slot, skill_id)| {
    //             let skill_entry = self.get_skill_list_entry(*skill_id);
    //             let skill = LearnableSkill::load(
    //                 sprite_loader,
    //                 action_loader,
    //                 *skill_id,
    //                 skill_entry.maximum_level,
    //                 skill_entry.file_name.clone(),
    //                 skill_entry.name.clone(),
    //                 client_tick,
    //             );
    //
    //             (*slot, skill)
    //         })),
    //         // TODO: Replicate the default behavior of the lua files
    //         None => HashMap::new(),
    //     }
    // }
    //
    // pub fn get_map_sky_data_from_resource_file(&self, resource_file: &str) -> Option<&MapSkyData> {
    //     self.map_sky_data_table.get(resource_file)
    // }
    //
    // pub fn load_inventory_item_metadata(
    //     &self,
    //     async_loader: &AsyncLoader,
    //     item: InventoryItem<NoMetadata>,
    // ) -> InventoryItem<ResourceMetadata> {
    //     let is_identified = item.is_identified();
    //
    //     let resource_name = self.get_item_resource_from_id(item.item_id, is_identified);
    //     let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
    //     let texture = async_loader.request_item_sprite_load(ItemLocation::Inventory, item.item_id, &full_path, ImageType::Color);
    //     let name = self.get_item_name_from_id(item.item_id, is_identified).to_string();
    //
    //     let metadata = ResourceMetadata { texture, name };
    //
    //     InventoryItem { metadata, ..item }
    // }
    //
    // pub fn load_shop_item_metadata(&self, async_loader: &AsyncLoader, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
    //     let resource_name = self.get_item_resource_from_id(item.item_id, true);
    //     let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
    //     let texture = async_loader.request_item_sprite_load(ItemLocation::Shop, item.item_id, &full_path, ImageType::Color);
    //     let name = self.get_item_name_from_id(item.item_id, true).to_string();
    //
    //     let metadata = ResourceMetadata { texture, name };
    //
    //     ShopItem { metadata, ..item }
    // }



/// Trait for data that can be stored in a table and retrieved using a key.
pub trait Table {
    type Key<'a>;
    type Storage;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage>;

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized;

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized;
}

fn fix_encoding(broken: String) -> String {
    let bytes: Vec<u8> = broken.chars().map(|char| char as u8).collect();
    match EUC_KR.decode_without_bom_handling_and_without_replacement(&bytes) {
        None => broken.to_string(),
        Some(char) => char.to_string(),
    }
}
