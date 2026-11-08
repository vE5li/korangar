use std::sync::LazyLock;

use hashbrown::HashMap;
use mlua::{ErrorContext, Lua};
use ragnarok_packets::{SkillId, SkillLevel};

use super::{HashMapExt, Library, Table, game_string};
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
    type Storage = HashMap<SkillId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `JOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        ])?;

        Self::from_lua(&state)
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

impl SkillListInformation {
    fn from_lua(state: &Lua) -> mlua::Result<HashMap<SkillId, Self>> {
        let globals = state.globals();
        let skill_info_list = globals.get::<mlua::Table>("SKILL_INFO_LIST")?;

        let mut result = HashMap::new();

        for (skill_id, table) in skill_info_list.pairs::<u16, mlua::Table>().flatten() {
            let file_name = table
                .get(1)
                .and_then(game_string)
                .context(format!("skill {skill_id} resource name"))?;
            let name = table
                .get("SkillName")
                .and_then(game_string)
                .context(format!("skill {skill_id} display name"))?;
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
}

#[cfg(test)]
mod tests {
    use mlua::Lua;
    use ragnarok_packets::SkillId;

    use super::SkillListInformation;
    use crate::world::library::lua::execute_chunk;

    #[test]
    fn raw_legacy_skill_resource_and_display_names_load() {
        let lua = Lua::new();
        execute_chunk(
            &lua,
            "synthetic-skills.lub",
            b"SKILL_INFO_LIST = {[1] = {[1] = '\xb0\xa1', SkillName = '\xb0\xa1', MaxLv = 1, bSeperateLv = false}}",
        )
        .unwrap();
        let table = SkillListInformation::from_lua(&lua).unwrap();
        assert_eq!(table[&SkillId(1)].file_name, "가");
        assert_eq!(table[&SkillId(1)].name, "가");
    }

    #[test]
    fn legacy_display_name_loads_even_when_resource_is_ascii() {
        let lua = Lua::new();
        execute_chunk(
            &lua,
            "synthetic-display-only.lub",
            b"SKILL_INFO_LIST = {[779] = {[1] = 'fixture', SkillName = '\xb0\xa1', MaxLv = 1, bSeperateLv = false}}",
        )
        .unwrap();
        let table = SkillListInformation::from_lua(&lua).unwrap();
        assert_eq!(table[&SkillId(779)].file_name, "fixture");
        assert_eq!(table[&SkillId(779)].name, "가");
    }

    #[test]
    fn skill_translations_are_not_low_byte_reinterpreted() {
        let lua = Lua::new();
        execute_chunk(
            &lua,
            "synthetic-utf8-skills.lub",
            "SKILL_INFO_LIST = {[1] = {[1] = '한글', SkillName = 'ươ 🦀', MaxLv = 1, bSeperateLv = false}}".as_bytes(),
        )
        .unwrap();
        let table = SkillListInformation::from_lua(&lua).unwrap();
        assert_eq!(table[&SkillId(1)].file_name, "한글");
        assert_eq!(table[&SkillId(1)].name, "ươ 🦀");
    }

    #[test]
    fn invalid_skill_string_is_a_contextual_error_not_a_cache_repair() {
        let lua = Lua::new();
        execute_chunk(
            &lua,
            "synthetic-invalid-skills.lub",
            b"SKILL_INFO_LIST = {[1] = {[1] = '\xff', SkillName = 'test', MaxLv = 1, bSeperateLv = false}}",
        )
        .unwrap();
        let error = SkillListInformation::from_lua(&lua).err().unwrap().to_string();
        assert!(error.contains("skill 1 resource name") && error.contains("neither valid"));
    }
}
