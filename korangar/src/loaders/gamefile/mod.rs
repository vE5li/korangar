//! Manages archives where game assets are stored and provides convenient
//! methods to retrieve each of them individually. The archives implement the
//! [`Archive`] trait.

mod cache;
mod list;

use core::panic;
use std::path::Path;
use std::sync::{Arc, RwLock};

use blake3::Hash;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_loaders::remote::{PersistentAssets, RemoteConfig};
use korangar_loaders::{FileLoader, FileNotFoundError};

pub use self::cache::{sync_cache_archive, texture_file_dds_name, video_file_ivf_name};
use self::list::GameArchiveList;
use super::archive::folder::FolderArchive;
use super::archive::native::NativeArchive;
use super::archive::{Archive, ArchiveType};
use crate::loaders::archive::seven_zip::SevenZipArchive;

pub(crate) const CACHE_FILE_NAME: &str = "cache.7z";

pub(crate) const TEMPORARY_CACHE_FILE_NAME: &str = "cache.7z.tmp";
pub(crate) const HASH_FILE_PATH: &str = "game_file_hash.txt";

/// This string is used to derive an initialization vector for the game file
/// hash calculation. We can use this to trigger a de-sync of the cache files of
/// users.
const GAME_FILE_DERIVE_KEY: &str = "korangar 2025-03-09 14:17:23 game file key v1";

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
/// - [`SevenZipArchive`] - Retrieve assets from ZIP files.
#[derive(Default)]
pub struct GameFileLoader {
    archives: RwLock<Vec<LoaderArchive>>,
    remote: RwLock<Option<Arc<PersistentAssets>>>,
}

impl FileLoader for GameFileLoader {
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError> {
        let lowercase_path = path.to_lowercase();
        if let Some(bytes) = self
            .archives
            .read()
            .unwrap()
            .iter()
            .find_map(|archive| archive.archive.get_file_by_path(&lowercase_path))
        {
            return Ok(bytes);
        }
        // Existing local archives retain priority. The permanent verified cache
        // is consulted next; the network is only used for missing/corrupt objects.
        self.remote
            .read()
            .unwrap()
            .as_ref()
            .map(|remote| {
                remote
                    .get(&lowercase_path)
                    .map_err(|error| FileNotFoundError::new(format!("{path}: verified remote retrieval failed: {error}")))
            })
            .unwrap_or_else(|| Err(FileNotFoundError::new(path.to_owned())))
    }
}

impl GameFileLoader {
    pub fn file_exists(&self, path: &str) -> bool {
        self.archives
            .read()
            .unwrap()
            .iter()
            .any(|archive| archive.archive.file_exists(path))
            || self.remote.read().unwrap().as_ref().is_some_and(|r| r.contains(path))
    }

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
        } else if let Some(extension) = path.extension()
            && let Some("7z") = extension.to_str()
        {
            ArchiveType::SevenZip
        } else {
            panic!("Provided archive must be a directory or have a .grf extension")
        }
    }

    fn load_archive_from_path(path: &str) -> Box<dyn Archive> {
        let path = Path::new(path);

        match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchive::from_path(path)),
            ArchiveType::SevenZip => Box::new(SevenZipArchive::from_path(path)),
        }
    }

    pub fn load_archives_from_settings(&self) -> Result<(), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        if let Ok(base_url) = std::env::var("KORANGAR_REMOTE_BASE_URL") {
            let cache_dir = std::env::var_os("KORANGAR_REMOTE_CACHE")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| ".korangar-assets".into());
            let enabled = |name| std::env::var(name).is_ok_and(|v| v == "1");
            let config = RemoteConfig {
                base_url,
                cache_dir,
                allow_http: enabled("KORANGAR_REMOTE_ALLOW_HTTP"),
                offline: enabled("KORANGAR_REMOTE_OFFLINE"),
            };
            let remote = PersistentAssets::open(config).map_err(|error| {
                format!(
                    "Cannot initialize verified remote assets: {error}. Stage an approved manifest/cache first; see \
                     docs/turoran-native-test.md. No local GRFs are required in remote mode."
                )
            })?;
            *self.remote.write().unwrap() = Some(Arc::new(remote));
        }
        let remote_enabled = self.remote.read().unwrap().is_some();
        let game_archive_list = GameArchiveList::load(remote_enabled)?;
        if !remote_enabled && game_archive_list.archives.is_empty() {
            return Err(
                "No game assets configured: add licensed local GRFs/overlays to client/game_archives.ron, or configure \
                 KORANGAR_REMOTE_BASE_URL and a staged manifest/cache. See docs/turoran-native-test.md."
                    .into(),
            );
        }
        for path in &game_archive_list.archives {
            if !Path::new(path).exists() {
                return Err(format!(
                    "Configured archive is missing: {path}. Update client/game_archives.ron; remote-only installs should omit missing \
                     GRFs."
                ));
            }
            let game_archive = Self::load_archive_from_path(path);
            self.add_archive(game_archive, true);
        }

        #[cfg(feature = "debug")]
        timer.stop();
        Ok(())
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new_derive_key(GAME_FILE_DERIVE_KEY);
        self.archives
            .read()
            .unwrap()
            .iter()
            .filter(|archive| archive.is_game_archive)
            .for_each(|archive| archive.archive.hash(&mut hasher));
        if let Some(remote) = self.remote.read().unwrap().as_ref() {
            hasher.update(remote.revision().as_bytes());
        }
        hasher.finalize()
    }

    pub fn get_files_with_extension(&self, extensions: &[&str]) -> Vec<String> {
        let mut files = Vec::new();
        self.archives
            .read()
            .unwrap()
            .iter()
            .for_each(|archive| archive.archive.get_files_with_extension(&mut files, extensions));

        if let Some(remote) = self.remote.read().unwrap().as_ref() {
            files.extend(
                remote
                    .paths()
                    .filter(|path| extensions.iter().any(|ext| path.ends_with(ext)))
                    .map(str::to_owned),
            );
        }
        files.sort();
        files.dedup();

        files
    }

    #[allow(unused_variables)]
    pub fn load_cache_archive(&self, game_file_hash: Hash) {
        let path = Path::new(CACHE_FILE_NAME);

        if !path.exists() && !path.is_dir() {
            return;
        }

        let archive = Box::new(SevenZipArchive::from_path(path));

        let Some(hash_file) = archive.get_file_by_path(HASH_FILE_PATH) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't find game hash file. Using empty cache");
            return;
        };

        let Ok(_hash) = Hash::from_hex(hash_file) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't read game hash file. Using empty cache");
            return;
        };

        #[cfg(feature = "debug")]
        if _hash != game_file_hash {
            print_debug!("[{}] Cache is out of sync. Please re-sync or delete the cache", "error".red());
        }

        self.add_archive(archive, false);
    }
}

pub fn fix_broken_texture_file_endings(path: &str) -> String {
    let mut path = path.to_string();

    if path.ends_with(".bm") {
        path.push('p');
    }

    if path.ends_with(".jp") {
        path.push('g');
    }

    if path.ends_with(".pn") {
        path.push('g');
    }

    if path.ends_with(".tg") {
        path.push('a');
    }

    path
}
