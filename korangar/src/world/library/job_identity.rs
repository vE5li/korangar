use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;

use super::{Library, Table};
use crate::loaders::GameFileLoader;

pub struct JobIdentity(Cow<'static, str>);

impl Display for JobIdentity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Table for JobIdentity {
    type Key<'a> = usize;
    type Storage = HashMap<usize, JobIdentity>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::new();

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\jobidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let data = game_file_loader
            .get("data\\luafiles514\\lua files\\datainfo\\npcidentity.lub")
            .unwrap();
        state.load(&data).exec()?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(jobtbl) = globals.get::<mlua::Table>("jobtbl") {
            for (key, value) in jobtbl.pairs::<String, usize>().flatten() {
                let cleaned_key = if let Some(end) = key.strip_prefix("JT_G_") {
                    end.to_string()
                } else {
                    key[3..].to_string()
                };

                result.insert(value, JobIdentity(cleaned_key.into()));
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

                result.insert(value, JobIdentity(cleaned_key.into()));
            }
        }

        let compacted = HashMap::from_iter(result);

        Ok(compacted)
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.job_identity_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: JobIdentity = JobIdentity(Cow::Borrowed("1_f_maria"));
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
