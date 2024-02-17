//! Manages archives where game assets are stored and provides convenient
//! methods to retrieve each of them individually. The archives implement the
//! [`Archive`](crate::loaders::archive::Archive) trait.
mod list;

use core::panic;
use std::path::Path;
use std::u8;

use self::list::GameArchiveList;
use super::archive::folder::FolderArchive;
use super::archive::native::{NativeArchive, NativeArchiveBuilder};
use super::archive::{Archive, ArchiveType, Writable};
#[cfg(feature = "debug")]
use crate::debug::*;

#[cfg(feature = "patched_as_folder")]
const LUA_GRF_FILE_NAME: &str = "lua_files/";
#[cfg(not(feature = "patched_as_folder"))]
const LUA_GRF_FILE_NAME: &str = "lua_files.grf";

/// Type implementing the game files loader.
///
/// Currently, there are two types implementing
/// [`Archive`](crate::loaders::archive::Archive):
/// - [`NativeArchive`](crate::loaders::archive::native::NativeArchive) -
///   Retrieve assets from GRF files.
/// - [`FolderArchive`](crate::loaders::archive::folder::FolderArchive) -
///   Retrieve assets from an OS folder.
#[derive(Default)]
pub struct GameFileLoader {
    archives: Vec<Box<dyn Archive>>,
}

impl GameFileLoader {
    fn add_archive(&mut self, game_archive: Box<dyn Archive>) {
        self.archives.insert(0, game_archive);
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

    pub fn load_archives_from_settings(&mut self) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        let game_archive_list = GameArchiveList::load();

        game_archive_list.archives.iter().for_each(|path| {
            let game_archive = Self::load_archive_from_path(path);
            self.add_archive(game_archive);
        });

        #[cfg(feature = "debug")]
        timer.stop();
    }

    pub fn load_patched_lua_files(&mut self) {
        if !Path::new(LUA_GRF_FILE_NAME).exists() {
            self.patch_lua_files();
        }

        let lua_archive = Self::load_archive_from_path(LUA_GRF_FILE_NAME);
        self.add_archive(lua_archive);
    }

    fn patch_lua_files(&mut self) {
        use lunify::{unify, Format, Settings};

        let mut lua_files = Vec::new();
        self.archives.iter().for_each(|archive| archive.get_lua_files(&mut lua_files));

        let path = Path::new(LUA_GRF_FILE_NAME);
        let mut lua_archive: Box<dyn Writable> = match GameFileLoader::get_archive_type_by_path(&path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(&path)),
            ArchiveType::Native => Box::new(NativeArchiveBuilder::from_path(&path)),
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
                            "[{}warning{}] failed to extract file {}{file_name}{} from the grf: {_error:?}",
                            YELLOW,
                            NONE,
                            MAGENTA,
                            NONE
                        );
                        failed_count += 1;
                    }

                    continue;
                }
            };

            // Try to unify all bytecode to Lua 5.1 and possibly 64 bit.
            match unify(&bytes, &bytecode_format, &settings) {
                Ok(bytes) => lua_archive.add_file(&file_name, bytes),
                // If the operation fails the file with this error, the Lua file is not actually a
                // pre-compiled binary but rather a source file, so we can safely ignore it.
                #[cfg(feature = "debug")]
                Err(lunify::LunifyError::IncorrectSignature) => total_count -= 1,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        print_debug!(
                            "[{}warning{}] error upcasting {}{file_name}{}: {_error:?}",
                            YELLOW,
                            NONE,
                            MAGENTA,
                            NONE
                        );
                        failed_count += 1;
                    }
                }
            }
        }

        #[cfg(feature = "debug")]
        print_debug!(
            "converted a total of {}{total_count}{} files of which {}{failed_count}{} failed.",
            YELLOW,
            NONE,
            RED,
            NONE
        );

        lua_archive.save();
    }

    pub fn get(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let lowercase_path = path.to_lowercase();
        let result = self
            .archives
            .iter_mut()
            .find_map(|archive| archive.get_file_by_path(&lowercase_path))
            .ok_or(format!("failed to find file {path}"));

        // TODO: should this be removed in the future or left in for resilience?
        if result.is_err() {
            #[cfg(feature = "debug")]
            print_debug!("failed to find file {}; tying to replace it with placeholder", path);

            let delimiter = path.len() - 4;
            match &path[delimiter..] {
                ".bmp" | ".BMP" => return self.get("data\\texture\\missing.bmp"),
                ".tga" | ".TGA" => return self.get("data\\texture\\missing.tga"),
                ".rsm" => return self.get("data\\model\\missing.rsm"),
                ".spr" => return self.get("data\\sprite\\npc\\missing.spr"),
                ".act" => return self.get("data\\sprite\\npc\\missing.act"),
                _other => {}
            }
        }

        result
    }
}
