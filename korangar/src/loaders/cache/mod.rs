// mod format;
//
// use std::fs;
// use std::path::Path;
// use std::sync::Arc;
// use std::sync::atomic::Ordering;
// use std::thread::available_parallelism;
//
// use blake3::Hash;
// use encoding_rs::UTF_8;
// use hashbrown::HashSet;
// #[cfg(feature = "debug")]
// use korangar_debug::logging::{Colorize, print_debug};
// use korangar_util::container::SimpleKey;
// use ragnarok_bytes::{ByteReader, ByteWriter, ConversionResultExt, FromBytes,
// ToBytes}; use ragnarok_formats::version::InternalVersion;
// use rayon::ThreadPoolBuilder;
// use rayon::prelude::*;
//
// use crate::SHUTDOWN_SIGNAL;
// use crate::loaders::archive::seven_zip::{SevenZipArchive,
// SevenZipArchiveBuilder}; use crate::loaders::archive::{Archive, Compression,
// Writable, os_specific_path}; use crate::loaders::{GameFileLoader, MapLoader,
// ModelLoader, TextureLoader};
//
// const HASH_FILE_PATH: &str = "game_file_hash.txt";
// const CACHE_PATH_NAME: &str = "cache.7z";
// const TEMPORARY_CACHE_PATH_NAME: &str = "cache.7z.tmp";
// const MAP_FILE_EXTENSION: &str = ".rsw";
// const ATLAS_FILE_EXTENSION: &str = ".kta";
//
// pub struct Cache {
//     archive: Option<Box<dyn Archive>>,
//     texture_compression_supported: bool,
// }
//
// impl Cache {
//     pub fn new(
//         game_file_loader: &GameFileLoader,
//         texture_loader: Arc<TextureLoader>,
//         map_loader: &MapLoader,
//         model_loader: &ModelLoader,
//         game_file_hash: Hash,
//         texture_compression_supported: bool,
//         sync_cache: bool,
//     ) -> Self {
//         let archive = match sync_cache {
//             true => Self::sync_cache_archive(game_file_loader,
// texture_loader, map_loader, model_loader, game_file_hash),             false
// => Self::get_cache_archive(game_file_hash),         };
//
//         Self {
//             archive,
//             texture_compression_supported,
//         }
//     }
//
//     #[allow(unused_variables)]
//     fn get_cache_archive(game_file_hash: Hash) -> Option<Box<dyn Archive>> {
//         let path = Path::new(CACHE_PATH_NAME);
//
//         if !path.exists() && !path.is_dir() {
//             return None;
//         }
//
//         let archive = Box::new(SevenZipArchive::from_path(path));
//
//         let Some(hash_file) = archive.get_file_by_path(HASH_FILE_PATH) else {
//             #[cfg(feature = "debug")]
//             print_debug!("Can't find game hash file. Using empty cache");
//             return None;
//         };
//         let Ok(_hash) = Hash::from_hex(hash_file) else {
//             #[cfg(feature = "debug")]
//             print_debug!("Can't read game hash file. Using empty cache");
//             return None;
//         };
//
//         #[cfg(feature = "debug")]
//         if _hash != game_file_hash {
//             print_debug!("[{}] Cache is out of sync. Please re-sync or delete
// the cache", "error".red());         }
//
//         Some(archive)
//     }
//
//     fn sync_cache_archive(
//         game_file_loader: &GameFileLoader,
//         texture_loader: Arc<TextureLoader>,
//         map_loader: &MapLoader,
//         model_loader: &ModelLoader,
//         game_file_hash: Hash,
//     ) -> Option<Box<dyn Archive>> {
//         println!("Starting sync of cache");
//
//         let path = Path::new(CACHE_PATH_NAME);
//
//         let game_file_map_names: Vec<String> = game_file_loader
//             .get_files_with_extension(MAP_FILE_EXTENSION)
//             .iter()
//             .map(|map_file_path| {
//                 let os_path = os_specific_path(map_file_path);
//                 os_path.file_stem().unwrap().to_string_lossy().to_string()
//             })
//             .collect();
//
//         let current_archive_exists = fs::exists(path).unwrap_or(false);
//
//         let (mut unused_map_atlas_names, mut outdated_map_names, mut
// new_map_names) = match current_archive_exists {             true => {
//                 let mut cached_texture_atlas_paths = Vec::new();
//
//                 let current_archive =
// Box::new(SevenZipArchive::from_path(path));
// current_archive.get_files_with_extension(&mut cached_texture_atlas_paths,
// ATLAS_FILE_EXTENSION);
//
//                 let cached_texture_atlas_paths: HashSet<String> =
//
// HashSet::from_iter(cached_texture_atlas_paths.iter().map(|map_file_path| {
//                         let os_path = os_specific_path(map_file_path);
//
// os_path.file_stem().unwrap().to_string_lossy().to_string()
// }));
//
//                 let unused_map_atlas_names: HashSet<String> =
// cached_texture_atlas_paths
// .difference(&HashSet::from_iter(game_file_map_names.iter().cloned()))
//                     .cloned()
//                     .collect();
//
//                 let thread_pool = ThreadPoolBuilder::new()
//                     .num_threads(available_parallelism().unwrap().get())
//                     .build()
//                     .unwrap();
//
//                 println!("Test texture atlas compatibility");
//                 let (outdated_map_names, new_map_names): (HashSet<&String>,
// HashSet<&String>) = thread_pool.install(|| {
// game_file_map_names                         .par_iter()
//                         .fold(
//                             || (HashSet::new(), HashSet::new()),
//                             |mut accumulator, map_name| {
//                                 let atlas_path =
// Self::get_texture_atlas_cache_base_path(map_name);
//
//                                 if let Some(atlas_data) =
// current_archive.get_file_by_path(&atlas_path) {
// println!("Checking texture atlas for map `{map_name}`");
//
//                                     let mut byte_reader:
// ByteReader<Option<InternalVersion>> =
// ByteReader::with_default_metadata(&atlas_data);
// byte_reader.set_encoding(UTF_8);
//
//                                     if let Ok(cached_atlas) =
// CachedTextureAtlas::from_bytes(&mut byte_reader) {
// if let Ok(textures) = map_loader.collect_map_textures(model_loader, map_name)
// {                                             let texture_atlas =
// Self::create_texture_atlas(texture_loader.clone(), map_name, textures);
//
//                                             if texture_atlas.hash() !=
// cached_atlas.hash {                                                 //
// Outdated maps
// accumulator.0.insert(map_name);                                             }
//                                         }
//                                     }
//                                 } else {
//                                     // New maps
//                                     accumulator.1.insert(map_name);
//                                 }
//
//                                 accumulator
//                             },
//                         )
//                         .reduce(
//                             || (HashSet::new(), HashSet::new()),
//                             |mut a, b| {
//                                 a.0.extend(b.0);
//                                 a.1.extend(b.1);
//                                 a
//                             },
//                         )
//                 });
//
//                 drop(thread_pool);
//
//                 (
//                     Vec::from_iter(unused_map_atlas_names),
//                     Vec::from_iter(outdated_map_names),
//                     Vec::from_iter(new_map_names),
//                 )
//             }
//             false => (Vec::new(), Vec::new(),
// Vec::from_iter(game_file_map_names.iter())),         };
//
//         unused_map_atlas_names.sort();
//         outdated_map_names.sort();
//         new_map_names.sort();
//
//         let mut created_count = 0;
//         let mut updated_count = 0;
//         let mut error_count = 0;
//
//         let mut to_create_map_names =
// Vec::from_iter(outdated_map_names.iter().copied().chain(new_map_names.iter().
// copied()));         to_create_map_names.sort();
//         to_create_map_names.dedup();
//
//         let outdated_map_names: HashSet<&String> =
// HashSet::from_iter(outdated_map_names.iter().copied());         let
// new_map_names: HashSet<&String> =
// HashSet::from_iter(new_map_names.iter().copied());
//
//         let _ = fs::remove_file(TEMPORARY_CACHE_PATH_NAME);
//
//         let archive_path = match current_archive_exists {
//             true => TEMPORARY_CACHE_PATH_NAME,
//             false => CACHE_PATH_NAME,
//         };
//
//         let mut builder =
// Box::new(SevenZipArchiveBuilder::from_path(Path::new(archive_path)));
//         builder.add_file(HASH_FILE_PATH,
// game_file_hash.to_hex().as_bytes().to_vec(), Compression::No);
//
//         if current_archive_exists {
//             let current_archive = Box::new(SevenZipArchive::from_path(path));
//
//             for map_name in game_file_map_names.iter() {
//                 if !outdated_map_names.contains(&map_name) &&
// !new_map_names.contains(&map_name) {                     let atlas_path =
// Self::get_texture_atlas_cache_base_path(map_name);                     if
// current_archive.file_exists(&atlas_path) {
// println!("Copying existing texture atlas for map `{map_name}`");
// builder.copy_file_from_archive(&current_archive, &atlas_path);
// }                 }
//             }
//         }
//
//         for &map_name in to_create_map_names.iter() {
//             println!("Creating texture atlas for map `{map_name}`");
//
//             match map_loader.collect_map_textures(model_loader, map_name) {
//                 Ok(textures) => {
//                     let texture_atlas =
// Self::create_texture_atlas(texture_loader.clone(), map_name, textures);
//                     let mut byte_writer = ByteWriter::with_encoding(UTF_8);
//
//                     if texture_atlas.to_cached_texture_atlas().to_bytes(&mut
// byte_writer).is_ok() {                         if
// SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
// println!("Cache sync aborted. Created: {created_count} Updated:
// {updated_count} Errors: {error_count}");                             return
// None;                         }
//
//                         let atlas_path =
// Self::get_texture_atlas_cache_base_path(map_name);
// builder.add_file(&atlas_path, byte_writer.into_inner(), Compression::Fast);
//
//                         if outdated_map_names.contains(map_name) {
//                             updated_count += 1;
//                         } else {
//                             created_count += 1;
//                         }
//                     }
//                 }
//                 Err(error) => {
//                     println!("[error] Can't create texture atlas for map
// `{map_name}`: {error:?}");                     error_count += 1;
//                 }
//             }
//         }
//
//         // Drop to finish the writing to the new archive.
//         drop(builder);
//
//         if current_archive_exists {
//             let _ = fs::rename(TEMPORARY_CACHE_PATH_NAME, CACHE_PATH_NAME);
//         }
//
//         println!("Cache sync finished. Created: {created_count} Updated:
// {updated_count} Errors: {error_count}");
//
//         Some(Box::new(SevenZipArchive::from_path(path)))
//     }
// }
