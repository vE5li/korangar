use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::{JobId, SkillId, SkillLevel};

use super::{HashMapExt, ItemName, ItemResource, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;
use crate::world::library::LuaExt;

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

impl Table for SkillListEntry {
    type Key<'a> = SkillId;
    type Storage = HashMap<SkillId, SkillListEntry>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `LOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        ])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("SKILL_INFO_LIST") {
            for (skill_id, table) in table.pairs::<u16, mlua::Table>().flatten() {
                let file_name = table.get(1)?;
                let name = table.get("SkillName")?;
                let maximum_level = table.get("MaxLv")?;

                let generic_required_skills = match table.get::<mlua::Table>("_NeedSkillList") {
                    Ok(sequence) => {
                        let mut required_skills = HashMap::new();

                        for required_skill in sequence.sequence_values::<mlua::Table>() {
                            let required_skill = required_skill?;
                            let skill_id = required_skill.get::<u16>(1)?; // TODO: At least one skill does not have a level set. I am just assuming that that means level 1 is required but should be confirmed.                   
                            let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
                            required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
                        }

                        required_skills
                    }
                    Err(..) => HashMap::new(),
                };

                let job_required_skills = match table.get::<mlua::Table>("NeedSkillList") {
                    Ok(table) => {
                        let mut job_specific = HashMap::new();

                        for (job_id, sequence) in table.pairs::<u16, mlua::Table>().flatten() {
                            let mut required_skills = HashMap::new();

                            for required_skill in sequence.sequence_values::<mlua::Table>() {
                                let required_skill = required_skill?;
                                let skill_id = required_skill.get::<u16>(1)?; // TODO: At least one skill does not have a level set. I am just assuming that that means level 1 is required but should be confirmed.
                                let skill_level = required_skill.get::<u16>(2).unwrap_or(1);
                                required_skills.insert(SkillId(skill_id), SkillLevel(skill_level));
                            }

                            job_specific.insert(JobId(job_id), required_skills);
                        }

                        job_specific
                    }
                    Err(..) => HashMap::new(),
                };

                result.insert(SkillId(skill_id), SkillListEntry {
                    file_name,
                    name,
                    maximum_level: SkillLevel(maximum_level),
                    generic_required_skills,
                    job_required_skills,
                });
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized,
    {
        library.skill_list_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized,
    {
        todo!()
        // Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
