use std::sync::LazyLock;

use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::{SkillId, SkillLevel};

use super::{HashMapExt, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;
use crate::state::skills::SkillAcquisition;
use crate::world::library::LuaExt;

static NOT_FOUND_ENTRY: LazyLock<SkillListInformation> = LazyLock::new(|| SkillListInformation {
    file_name: "notfound".to_owned(),
    name: "notfound".to_owned(),
    maximum_level: SkillLevel(100),
    can_select_level: false,
    // To make it unskillable.
    acquisition: SkillAcquisition::Quest,
});

pub struct SkillListInformation {
    pub file_name: String,
    pub name: String,
    pub maximum_level: SkillLevel,
    pub can_select_level: bool,
    pub acquisition: SkillAcquisition,
}

impl Table for SkillListInformation {
    type Key<'a> = SkillId;
    type Storage = HashMap<SkillId, SkillListInformation>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `LOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        ])?;

        let globals = state.globals();
        let skill_info_list = globals.get::<mlua::Table>("SKILL_INFO_LIST")?;

        let mut result = HashMap::new();

        for (skill_id, table) in skill_info_list.pairs::<u16, mlua::Table>().flatten() {
            let file_name = table.get(1)?;
            let name = table.get("SkillName").map(fix_encoding)?;
            let maximum_level = table.get("MaxLv")?;
            let can_select_level = table.get("bSeperateLv")?;
            let acquisition = match table.get::<String>("Type").ok().as_deref() {
                Some("Quest") => SkillAcquisition::Quest,
                Some("Soul") => SkillAcquisition::SoulLink,
                None => SkillAcquisition::Job,
                Some(unknown) => panic!("unknown skill type {}", unknown),
            };

            result.insert(SkillId(skill_id), SkillListInformation {
                file_name,
                name,
                maximum_level: SkillLevel(maximum_level),
                can_select_level,
                acquisition,
            });
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized,
    {
        library.skill_information_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized,
    {
        Self::try_get(library, key).unwrap_or(&*NOT_FOUND_ENTRY)
    }
}
