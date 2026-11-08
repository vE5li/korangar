mod baby_job;
mod item_info;
mod item_name;
mod item_resource;
mod job_identity;
mod lua;
mod map_sky_data;
mod skill_information;
mod skill_requirements;
mod skill_tree;

use std::hash::Hash;

use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::{ErrorContext, Lua};

pub use self::baby_job::IsBabyJob;
pub use self::item_info::ItemInfo;
pub use self::item_name::{ItemName, ItemNameKey};
pub use self::item_resource::{ItemResource, ItemResourceKey};
pub use self::job_identity::JobIdentity;
use self::lua::{execute_chunk, game_string};
pub use self::map_sky_data::MapSkyData;
pub use self::skill_tree::SkillTreeLayout;
use crate::loaders::GameFileLoader;
pub use crate::world::library::skill_information::SkillListInformation;
pub use crate::world::library::skill_requirements::{SkillListKey, SkillListRequirements};

pub struct Library {
    job_identity_table: <JobIdentity as Table>::Storage,
    item_info_table: <ItemInfo as Table>::Storage,
    map_sky_data_table: <MapSkyData as Table>::Storage,
    skill_information_table: <SkillListInformation as Table>::Storage,
    skill_requirements_table: <SkillListRequirements as Table>::Storage,
    skill_tree_table: <SkillTreeLayout as Table>::Storage,
    baby_job_table: <IsBabyJob as Table>::Storage,
}

impl Library {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let job_identity_table = JobIdentity::load(game_file_loader).context("loading JobIdentity")?;
        let item_info_table = ItemInfo::load(game_file_loader).context("loading ItemInfo")?;
        let map_sky_data_table = MapSkyData::load(game_file_loader).context("loading MapSkyData")?;
        let skill_information_table = SkillListInformation::load(game_file_loader).context("loading SkillListInformation")?;
        let skill_requirements_table = SkillListRequirements::load(game_file_loader).context("loading SkillListRequirements")?;
        let skill_tree_table = SkillTreeLayout::load(game_file_loader).context("loading SkillTreeLayout")?;
        let baby_job_table = IsBabyJob::load(game_file_loader).context("loading IsBabyJob")?;

        Ok(Self {
            job_identity_table,
            item_info_table,
            map_sky_data_table,
            skill_information_table,
            skill_requirements_table,
            skill_tree_table,
            baby_job_table,
        })
    }

    #[inline(always)]
    pub fn get<T: Table>(&self, key: T::Key<'_>) -> &T {
        T::get(self, key)
    }
}

/// Trait for compacting a hash map after it is completely populated.
trait HashMapExt {
    /// Compact the hash map, possibly by creating a second one.
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

trait LuaExt: Sized {
    fn load_from_game_files(game_file_loader: &impl FileLoader, files: &[&str]) -> mlua::Result<Self>;
}

impl LuaExt for Lua {
    fn load_from_game_files(game_file_loader: &impl FileLoader, files: &[&str]) -> mlua::Result<Self> {
        let state = Lua::new();

        for file in files {
            let data = game_file_loader
                .get(file)
                .map_err(|error| mlua::Error::RuntimeError(format!("Cannot read Lua asset {file}: {error:?}")))?;
            execute_chunk(&state, file, &data)?;
        }

        Ok(state)
    }
}

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
