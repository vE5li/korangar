//! Manages archives where game assets are stored and provides convenient
//! methods to retrieve each of them individually. The archives implement the
//! [`Archive`] trait.
mod list;

use core::panic;
use std::path::Path;
use std::sync::RwLock;

use blake3::Hash;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::{FileLoader, FileNotFoundError};

use self::list::GameArchiveList;
use super::archive::folder::FolderArchive;
use super::archive::native::{NativeArchive, NativeArchiveBuilder};
use super::archive::{Archive, ArchiveType, Writable};

#[cfg(feature = "patched_as_folder")]
const LUA_GRF_FILE_NAME: &str = "lua_files/";
#[cfg(not(feature = "patched_as_folder"))]
const LUA_GRF_FILE_NAME: &str = "lua_files.grf";

/// This string is used to derive an initialization vector for the game file
/// hash calculation. We can use this to trigger a de-sync of the cache files of
/// users.
const GAME_FILE_DERIVE_KEY: &str = "korangar 2025-02-18 19:20:03 game file key v1";

struct LoaderArchive {
    archive: Box<dyn Archive>,
    is_game_archive: bool,
}

/// Type implementing the game file loader.
///
/// Currently, there are two types implementing
/// [`Archive`]:
/// - [`NativeArchive`] - Retrieve assets from GRF files.
/// - [`FolderArchive`] - Retrieve assets from an OS folder.
#[derive(Default)]
pub struct GameFileLoader {
    archives: RwLock<Vec<LoaderArchive>>,
}

impl FileLoader for GameFileLoader {
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError> {
        let lowercase_path = path.to_lowercase();
        self.archives
            .read()
            .unwrap()
            .iter()
            .find_map(|archive| archive.archive.get_file_by_path(&lowercase_path))
            .ok_or_else(|| FileNotFoundError::new(path.to_owned()))
    }
}

impl GameFileLoader {
    fn add_archive(&self, archive: Box<dyn Archive>, is_game_archive: bool) {
        self.archives.write().unwrap().insert(0, LoaderArchive { archive, is_game_archive });
    }

    fn get_archive_type_by_path(path: &Path) -> ArchiveType {
        if path.is_dir() || path.display().to_string().ends_with('/') {
            ArchiveType::Folder
        } else if let Some(extension) = path.extension()
            && let Some("grf") = extension.to_str()
        {
            ArchiveType::Native
        } else {
            panic!("Provided archive must be a directory or have a .grf extension")
        }
    }

    fn load_archive_from_path(path: &str) -> Box<dyn Archive> {
        let path = Path::new(path);

        match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchive::from_path(path)),
        }
    }

    pub fn load_archives_from_settings(&self) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        let game_archive_list = GameArchiveList::load();

        game_archive_list.archives.iter().for_each(|path| {
            let game_archive = Self::load_archive_from_path(path);
            self.add_archive(game_archive, true);
        });

        #[cfg(feature = "debug")]
        timer.stop();
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new_derive_key(GAME_FILE_DERIVE_KEY);
        self.archives
            .read()
            .unwrap()
            .iter()
            .filter(|archive| archive.is_game_archive)
            .for_each(|archive| archive.archive.hash(&mut hasher));
        hasher.finalize()
    }

    pub fn remove_patched_lua_files(&self) {
        if Path::new(LUA_GRF_FILE_NAME).exists() {
            #[cfg(feature = "patched_as_folder")]
            std::fs::remove_dir_all(LUA_GRF_FILE_NAME).unwrap();

            #[cfg(not(feature = "patched_as_folder"))]
            std::fs::remove_file(LUA_GRF_FILE_NAME).unwrap();
        }
    }

    pub fn load_patched_lua_files(&self) {
        if !Path::new(LUA_GRF_FILE_NAME).exists() {
            self.patch_lua_files();
        }

        let lua_archive = Self::load_archive_from_path(LUA_GRF_FILE_NAME);
        self.add_archive(lua_archive, false);
    }

    pub fn get_files_with_extension(&self, extension: &str) -> Vec<String> {
        let mut files = Vec::new();
        self.archives
            .read()
            .unwrap()
            .iter()
            .for_each(|archive| archive.archive.get_files_with_extension(&mut files, extension));
        files
    }

    fn patch_lua_files(&self) {
        use lunify::{unify, Format, Settings};

        const LUA_BYTECODE_EXTENSION: &str = ".lub";
        let lua_files = self.get_files_with_extension(LUA_BYTECODE_EXTENSION);

        let path = Path::new(LUA_GRF_FILE_NAME);
        let mut lua_archive: Box<dyn Writable> = match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchiveBuilder::from_path(path)),
        };

        let bytecode_format = Format::default();
        let settings = Settings::default();

        #[cfg(feature = "debug")]
        let mut total_count = lua_files.len();
        #[cfg(feature = "debug")]
        let mut failed_count = 0;

        for file_name in lua_files {
            let bytes = match self.get(&file_name) {
                Ok(bytes) => bytes,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        print_debug!(
                            "[{}] failed to extract file {} from the grf: {:?}",
                            "warning".yellow(),
                            file_name.magenta(),
                            _error
                        );
                        failed_count += 1;
                    }

                    continue;
                }
            };

            // Try to unify all bytecode to Lua 5.1 and possibly 64 bit.
            match unify(&bytes, &bytecode_format, &settings) {
                Ok(bytes) => lua_archive.add_file(&file_name, bytes, true),
                // If the operation fails the file with this error, the Lua file is not actually a
                // pre-compiled binary but rather a source file, so we can safely ignore it.
                #[cfg(feature = "debug")]
                Err(lunify::LunifyError::IncorrectSignature) => total_count -= 1,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        print_debug!("[{}] error upcasting {}: {:?}", "warning".yellow(), file_name.magenta(), _error,);
                        failed_count += 1;
                    }
                }
            }
        }

        #[cfg(feature = "debug")]
        print_debug!(
            "converted a total of {} files of which {} failed.",
            total_count.yellow(),
            failed_count.red(),
        );

        lua_archive.finish().expect("can't save lua archive");
    }
}
