use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::JobId;

use super::{HashMapExt, Library, LuaExt, Table};
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IsBabyJob(pub bool);

impl Table for IsBabyJob {
    type Key<'a> = JobId;
    type Storage = HashMap<JobId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed for `JTtbl`.
            "data\\luafiles514\\lua files\\datainfo\\jobidentity.lub",
            // Lookup for checking if a given job id is a baby job using `IS_BABY_JOB`.
            "data\\lua-scaffolding\\baby-job-lookup.lua",
        ])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        let job_table = globals.get::<mlua::Table>("JTtbl")?;
        let is_baby_job = globals.get::<mlua::Function>("IS_BABY_JOB")?;

        for (_, job_id) in job_table.pairs::<String, u16>().flatten() {
            let is_baby_job = Self(is_baby_job.call(job_id)?);
            result.insert(JobId(job_id), is_baby_job);
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.baby_job_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: IsBabyJob = IsBabyJob(false);

        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
