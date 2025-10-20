use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::{JobId, SkillId, SkillLevel};

use super::{HashMapExt, ItemName, ItemResource, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;
use crate::world::library::LuaExt;

pub struct SkillTreeLayout(pub HashMap<usize, SkillId>);

impl Table for SkillTreeLayout {
    type Key<'a> = JobId;
    type Storage = HashMap<JobId, SkillTreeLayout>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `LOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            // Needed to get the `JobSkillTable.ChangeSkillTabName` function.
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfo_f.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skilltreeview.lub",
            // "data\luafiles514\lua files\skillinfoz\skilltreeview 20180621.lub",
        ])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("SKILL_TREEVIEW_FOR_JOB") {
            for (job_id, view_table) in table.pairs::<u16, mlua::Table>().flatten() {
                let mut view_result = HashMap::new();

                for (slot, skill_id) in view_table.pairs::<usize, u16>().flatten() {
                    view_result.insert(slot, SkillId(skill_id));
                }

                result.insert(JobId(job_id), SkillTreeLayout(view_result.compact()));
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized,
    {
        library.skill_tree_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized,
    {
        todo!()
        // TODO: Replicate Lua scripts behaviour.
        // Self::try_get(library, key).unwrap_or_default()
    }
}
