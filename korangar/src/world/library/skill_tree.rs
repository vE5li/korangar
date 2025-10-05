use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::{JobId, SkillId};

use super::{HashMapExt, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;
use crate::world::library::LuaExt;

#[derive(Debug, Clone)]
pub struct SkillTabLayout {
    pub name: String,
    pub skills: HashMap<usize, SkillId>,
}

// TODO: Rename this or the other `SkillTreeLayout` to avoid confusion.
// Same for the tab layout.
#[derive(Debug, Clone, Default)]
pub struct SkillTreeLayout {
    pub tabs: Vec<SkillTabLayout>,
}

fn evaluate_job(
    lookup_cache: &mut HashMap<JobId, SkillTreeLayout>,
    skill_tree_view: &mlua::Table,
    inherit_list: &mlua::Table,
    inherit_list_2: &mlua::Table,
    get_tab_for_job: &mlua::Function,
    get_skill_tab_name: &mlua::Function,
    job_id: JobId,
) -> mlua::Result<SkillTreeLayout> {
    if let Some(skill_tree_layout) = lookup_cache.get(&job_id) {
        // Job was already evaluated.
        return Ok(skill_tree_layout.clone());
    }

    let tab = get_tab_for_job.call::<usize>(job_id.0)?;
    let skill_tab_names = get_skill_tab_name.call::<mlua::Table>(job_id.0)?;

    // We need at least as many tabs as this job is on.
    let mut result = vec![HashMap::default(); tab + 1];

    if let Ok(view_table) = skill_tree_view.get::<mlua::Table>(job_id.0) {
        for (slot, skill_id) in view_table.pairs::<usize, u16>().flatten() {
            result[tab].insert(slot, SkillId(skill_id));
        }
    }

    let mut inherit_job = |inherited_job_id: JobId| -> mlua::Result<()> {
        let inherited_skill_tree_layout = evaluate_job(
            lookup_cache,
            skill_tree_view,
            inherit_list,
            inherit_list_2,
            get_tab_for_job,
            get_skill_tab_name,
            inherited_job_id,
        )?;

        for (tab_index, tab) in inherited_skill_tree_layout.tabs.iter().enumerate() {
            for (slot, skill_id) in tab.skills.iter() {
                // This assumes that a job can not inherit from a job on a higher tab.
                if result[tab_index].get(slot).is_none() {
                    result[tab_index].insert(*slot, *skill_id);
                }
            }
        }

        Ok(())
    };

    // Inherit from JOB_INHERIT_LIST2 first.
    if let Ok(inherited_job_id) = inherit_list_2.get(job_id.0) {
        inherit_job(JobId(inherited_job_id))?;
    }

    // Inherit from JOB_INHERIT_LIST second.
    if let Ok(inherited_job_id) = inherit_list.get(job_id.0) {
        inherit_job(JobId(inherited_job_id))?;
    }

    let tabs = result
        .into_iter()
        .enumerate()
        .map(|(tab_index, hash_map)| {
            let name = skill_tab_names
                .get(tab_index)
                .map(fix_encoding)
                .unwrap_or_else(|_| "<unnamed>".to_owned());
            let skills = hash_map.compact();

            SkillTabLayout { name, skills }
        })
        .collect();
    let skill_tree_layout = SkillTreeLayout { tabs };

    lookup_cache.insert(job_id, skill_tree_layout.clone());
    Ok(skill_tree_layout)
}

impl Table for SkillTreeLayout {
    type Key<'a> = JobId;
    type Storage = HashMap<JobId, SkillTreeLayout>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `LOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            // Better version of the `JobSkillTable.ChangeSkillTabName` function defined in `skillinfo_f.lub`.
            "data\\lua-scaffolding\\skill-tree-tab-name.lua",
            // Needed for the `SKILL_TREEVIEW_FOR_JOB` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skilltreeview.lub",
            // Needed for `GET_TAB_FOR_JOB`.
            "data\\lua-scaffolding\\skill-tree-tab-for-job.lua",
        ])?;

        let globals = state.globals();

        let inherit_list = globals.get::<mlua::Table>("JOB_INHERIT_LIST")?;
        let inherit_list_2 = globals.get::<mlua::Table>("JOB_INHERIT_LIST2")?;
        let skill_tree_view = globals.get::<mlua::Table>("SKILL_TREEVIEW_FOR_JOB")?;
        let get_skill_tab_name = globals.get::<mlua::Function>("GET_SKILL_TAB_NAME")?;
        let get_tab_for_job = globals.get::<mlua::Function>("GET_TAB_FOR_JOB")?;
        let job_ids = globals.get::<mlua::Table>("JOBID")?;

        let mut result = HashMap::new();

        for (_, job_id) in job_ids.pairs::<String, u16>().flatten() {
            evaluate_job(
                &mut result,
                &skill_tree_view,
                &inherit_list,
                &inherit_list_2,
                &get_tab_for_job,
                &get_skill_tab_name,
                JobId(job_id),
            )?;
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
        Self::try_get(library, key).unwrap()
    }
}
