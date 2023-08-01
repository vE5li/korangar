mod archive;
mod list;

use std::path::Path;

use self::archive::GameArchive;
use self::list::GameArchiveList;
#[cfg(feature = "debug")]
use crate::debug::*;

const LUA_GRF_FILE_NAME: &str = "lua_files.grf";

#[derive(Default)]
pub struct GameFileLoader {
    archives: Vec<GameArchive>,
    lua_files: Vec<String>,
}

impl GameFileLoader {
    fn add_archive(&mut self, game_archive: GameArchive) {
        self.archives.insert(0, game_archive);
    }

    pub fn load_archives_from_settings(&mut self) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        let game_archive_list = GameArchiveList::load();

        game_archive_list.archives.iter().for_each(|path| {
            let game_archive = GameArchive::load(path, &mut self.lua_files);
            self.add_archive(game_archive);
        });

        #[cfg(feature = "debug")]
        timer.stop();
    }

    pub fn load_patched_lua_files(&mut self) {
        let lua_archive = match Path::new(LUA_GRF_FILE_NAME).exists() {
            true => GameArchive::load(LUA_GRF_FILE_NAME, &mut Vec::new()),
            false => self.patch_lua_files(),
        };

        self.add_archive(lua_archive);
    }

    fn patch_lua_files(&mut self) -> GameArchive {
        use lunify::{unify, Format, Settings};

        let lua_files: Vec<String> = self.lua_files.drain(..).collect();
        let mut lua_archive = GameArchive::default();
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
                Ok(bytes) => lua_archive.add_file(file_name, bytes),
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

        lua_archive.save(LUA_GRF_FILE_NAME);
        lua_archive
    }

    pub fn get(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let result = self
            .archives
            .iter_mut() // convert this to a multithreaded iter ?
            .find_map(|archive| archive.get(&path.to_lowercase()))
            .ok_or(format!("failed to find file {path}"));

        // TODO: should this be removed in the future or left in for resilience?
        if result.is_err() {
            #[cfg(feature = "debug")]
            print_debug!("failed to find file {}; tying to replace it with placeholder", path);

            let delimiter = path.len() - 4;
            match &path[delimiter..] {
                ".bmp" | ".BMP" => return self.get("data\\texture\\backside.bmp"),
                ".rsm" => return self.get("data\\model\\abyss\\coin_j_01.rsm"),
                ".spr" => return self.get("data\\sprite\\npc\\1_f_maria.spr"),
                ".act" => return self.get("data\\sprite\\npc\\1_f_maria.act"),
                _other => {}
            }
        }

        result
    }
}
