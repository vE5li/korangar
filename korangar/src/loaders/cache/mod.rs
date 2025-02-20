mod format;

use std::path::Path;
use std::sync::Arc;
use std::thread::available_parallelism;

use blake3::Hash;
use cgmath::Vector2;
use encoding_rs::UTF_8;
use hashbrown::{HashMap, HashSet};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::Rectangle;
use korangar_util::container::{SecondarySimpleSlab, SimpleKey};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation};
use ragnarok_bytes::{ByteReader, ByteWriter, ConversionResult, ConversionResultExt, FromBytes, ToBytes};
use ragnarok_formats::signature::Signature;
use ragnarok_formats::version::{InternalVersion, MajorFirst, Version};
use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::loaders::archive::folder::FolderArchive;
use crate::loaders::archive::{Archive, Writable, os_specific_path};
use crate::loaders::cache::format::{AllocationEntry, LookupEntry, TextureAtlasData};
use crate::loaders::{GameFileLoader, MapLoader, ModelLoader, TextureAtlas, TextureAtlasEntry, TextureLoader, UncompressedTextureAtlas};

const HASH_FILE_PATH: &str = "game_file_hash.txt";
const CACHE_PATH_NAME: &str = "cache";
const MAP_FILE_EXTENSION: &str = ".rsw";
const ATLAS_FILE_EXTENSION: &str = ".kta";

pub struct Cache {
    archive: Option<Box<dyn Archive>>,
    texture_compression_supported: bool,
}

impl Cache {
    pub fn new(
        game_file_loader: &GameFileLoader,
        texture_loader: Arc<TextureLoader>,
        map_loader: &MapLoader,
        model_loader: &ModelLoader,
        game_file_hash: Hash,
        texture_compression_supported: bool,
        sync_cache: bool,
    ) -> Self {
        let archive = match sync_cache {
            true => Self::sync_cache_archive(game_file_loader, texture_loader, map_loader, model_loader, game_file_hash),
            false => Self::get_cache_archive(game_file_hash),
        };

        Self {
            archive,
            texture_compression_supported,
        }
    }

    #[allow(unused_variables)]
    fn get_cache_archive(game_file_hash: Hash) -> Option<Box<dyn Archive>> {
        let folder_path = Path::new(CACHE_PATH_NAME);

        if !folder_path.exists() && !folder_path.is_dir() {
            return None;
        }

        let folder_archive = Box::new(FolderArchive::from_path(folder_path));

        let Some(hash_file) = folder_archive.get_file_by_path(HASH_FILE_PATH) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't find game hash file. Using empty cache");
            return None;
        };
        let Ok(_hash) = Hash::from_hex(hash_file) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't read game hash file. Using empty cache");
            return None;
        };

        #[cfg(feature = "debug")]
        if _hash != game_file_hash {
            print_debug!("[{}] Cache is out of sync. Please re-sync or delete the cache", "error".red());
        }

        Some(folder_archive)
    }

    fn sync_cache_archive(
        game_file_loader: &GameFileLoader,
        texture_loader: Arc<TextureLoader>,
        map_loader: &MapLoader,
        model_loader: &ModelLoader,
        game_file_hash: Hash,
    ) -> Option<Box<dyn Archive>> {
        println!("Starting sync of cache");

        let folder_path = Path::new(CACHE_PATH_NAME);

        let mut folder_archive = Box::new(FolderArchive::from_path(folder_path));
        folder_archive.add_file(HASH_FILE_PATH, game_file_hash.to_hex().as_bytes().to_vec(), false);

        let mut cached_texture_atlas_paths = Vec::new();
        folder_archive.get_files_with_extension(&mut cached_texture_atlas_paths, ATLAS_FILE_EXTENSION);

        let cached_texture_atlas_paths: HashSet<String> = HashSet::from_iter(cached_texture_atlas_paths.iter().map(|map_file_path| {
            let os_path = os_specific_path(map_file_path);
            os_path.file_stem().unwrap().to_string_lossy().to_string()
        }));

        let game_file_map_names: Vec<String> = game_file_loader
            .get_files_with_extension(MAP_FILE_EXTENSION)
            .iter()
            .map(|map_file_path| {
                let os_path = os_specific_path(map_file_path);
                os_path.file_stem().unwrap().to_string_lossy().to_string()
            })
            .collect();

        let unused_map_atlas_names: HashSet<String> = cached_texture_atlas_paths
            .difference(&HashSet::from_iter(game_file_map_names.iter().cloned()))
            .cloned()
            .collect();

        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(available_parallelism().unwrap().get())
            .build()
            .unwrap();

        println!("Test texture atlas compatibility");
        let (outdated_map_names, new_map_names): (HashSet<&String>, HashSet<&String>) = thread_pool.install(|| {
            game_file_map_names
                .par_iter()
                .fold(
                    || (HashSet::new(), HashSet::new()),
                    |mut accumulator, map_name| {
                        let atlas_path = Self::get_texture_atlas_cache_base_path(map_name);

                        if let Some(atlas_data) = folder_archive.get_file_by_path(&atlas_path) {
                            println!("Checking texture atlas for map `{map_name}`");

                            let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&atlas_data);
                            byte_reader.set_encoding(UTF_8);

                            if let Ok(cached_atlas) = CachedTextureAtlas::from_bytes(&mut byte_reader) {
                                if let Ok(textures) = map_loader.collect_map_textures(model_loader, map_name) {
                                    let texture_atlas = Self::create_texture_atlas(texture_loader.clone(), map_name, textures);

                                    if texture_atlas.hash() != cached_atlas.hash {
                                        // Outdated maps
                                        accumulator.0.insert(map_name);
                                    }
                                }
                            }
                        } else {
                            // New maps
                            accumulator.1.insert(map_name);
                        }

                        accumulator
                    },
                )
                .reduce(
                    || (HashSet::new(), HashSet::new()),
                    |mut a, b| {
                        a.0.extend(b.0);
                        a.1.extend(b.1);
                        a
                    },
                )
        });

        drop(thread_pool);

        let mut created_count = 0;
        let mut removed_count = 0;
        let mut updated_count = 0;
        let mut error_count = 0;

        for map_name in unused_map_atlas_names.iter() {
            let atlas_path = Self::get_texture_atlas_cache_base_path(map_name);

            println!("Deleting unused texture atlas file `{}`", atlas_path);

            folder_archive.remove_file(&atlas_path);

            removed_count += 1;
        }

        for &map_name in outdated_map_names.union(&new_map_names) {
            println!("Re-creating texture atlas for map `{}`", map_name);

            match map_loader.collect_map_textures(model_loader, map_name) {
                Ok(textures) => {
                    let texture_atlas = Self::create_texture_atlas(texture_loader.clone(), map_name, textures);
                    let mut byte_writer = ByteWriter::with_encoding(UTF_8);

                    if texture_atlas.to_cached_texture_atlas().to_bytes(&mut byte_writer).is_ok() {
                        let atlas_path = Self::get_texture_atlas_cache_base_path(map_name);
                        folder_archive.add_file(&atlas_path, byte_writer.into_inner(), true);

                        match outdated_map_names.contains(map_name) {
                            true => updated_count += 1,
                            false => created_count += 1,
                        }
                    }
                }
                Err(error) => {
                    println!("[error] Can't create texture atlas for map `{}`: {:?}", map_name, error);
                    error_count += 1;
                }
            }
        }

        println!(
            "Cache sync finished. Created: {} Removed: {} Updated: {} Errors: {}",
            created_count, removed_count, updated_count, error_count
        );

        Some(folder_archive)
    }

    fn create_texture_atlas(texture_loader: Arc<TextureLoader>, map_name: &str, textures: HashSet<String>) -> UncompressedTextureAtlas {
        let mut textures: Vec<String> = textures.into_iter().collect();
        textures.sort();

        let mut texture_atlas = UncompressedTextureAtlas::new(texture_loader, map_name.to_string(), true, true, true);
        textures.iter().for_each(|texture| {
            let _ = texture_atlas.register(texture);
        });
        texture_atlas.build_atlas();

        texture_atlas
    }

    fn get_texture_atlas_cache_base_path(name: &str) -> String {
        format!("atlas\\{}.kta", name,)
    }

    pub fn load_texture_atlas(&self, name: &str) -> Option<CachedTextureAtlas> {
        // Cached textures can only be used, if the graphics card supports BC texture
        // compression (desktop class GPUs have nearly 100% support for it, including
        // M1+ GPUs).
        if !self.texture_compression_supported {
            return None;
        }

        self.archive.as_ref().and_then(|archive| {
            #[cfg(feature = "debug")]
            let _timer = Timer::new_dynamic(format!("load cached texture atlas for {}", name.magenta()));

            let data_path = Self::get_texture_atlas_cache_base_path(name);
            let data = archive.get_file_by_path(&data_path)?;

            let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&data);
            byte_reader.set_encoding(UTF_8);
            let cached_atlas = CachedTextureAtlas::from_bytes(&mut byte_reader).ok()?;

            Some(cached_atlas)
        })
    }
}

pub struct CachedTextureAtlas {
    pub hash: Hash,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub mipmaps_count: u32,
    pub transparent: bool,
    pub lookup: HashMap<String, TextureAtlasEntry>,
    pub allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
    pub compressed_data: Vec<u8>,
}

impl FromBytes for CachedTextureAtlas {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let mut atlas_data = TextureAtlasData::from_bytes(byte_reader).trace::<Self>()?;

        let transparent = atlas_data.lookup.iter().any(|entry| entry.transparent != 0);

        let mut lookup = HashMap::with_capacity(atlas_data.lookup.len());
        atlas_data.lookup.drain(..).for_each(|entry| {
            lookup.insert(entry.name, TextureAtlasEntry {
                allocation_id: AllocationId::new(entry.allocation_id),
                transparent: entry.transparent != 0,
            });
        });

        let mut allocations = SecondarySimpleSlab::with_capacity(atlas_data.allocations.len() as _);

        // It's faster to insert last to front, since we can then allocate all empty
        // slots right from the start.
        atlas_data.allocations.sort_by(|a, b| b.id.cmp(&a.id));

        atlas_data.allocations.drain(..).for_each(|entry| {
            allocations.insert(AllocationId::new(entry.id), AtlasAllocation {
                rectangle: Rectangle::new(entry.min, entry.max),
                atlas_size: Vector2::new(atlas_data.width, atlas_data.height),
            });
        });

        let compressed_data = byte_reader.slice::<u8>(atlas_data.compressed_data_size as usize)?.to_vec();

        Ok(CachedTextureAtlas {
            hash: Hash::from_bytes(atlas_data.hash),
            name: atlas_data.name,
            width: atlas_data.width,
            height: atlas_data.height,
            mipmaps_count: atlas_data.mipmaps_count,
            transparent,
            compressed_data,
            lookup,
            allocations,
        })
    }
}

impl ToBytes for CachedTextureAtlas {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            let lookup = Vec::from_iter(self.lookup.iter().map(|(name, atlas_entry)| LookupEntry {
                name: name.clone(),
                allocation_id: atlas_entry.allocation_id.key(),
                transparent: u32::from(atlas_entry.transparent),
            }));
            let allocations = Vec::from_iter(self.allocations.iter().map(|(id, atlas_allocation)| AllocationEntry {
                id: id.key(),
                min: atlas_allocation.rectangle.min,
                max: atlas_allocation.rectangle.max,
            }));
            let atlas_data = TextureAtlasData {
                signature: Signature::<b"kta">,
                version: Version::<MajorFirst>::new(1, 0),
                name: self.name.clone(),
                format: 0,
                width: self.width,
                height: self.height,
                mipmaps_count: self.mipmaps_count,
                hash: *self.hash.as_bytes(),
                lookup_count: u32::try_from(lookup.len()).expect("lookup_count bigger than u32::MAX"),
                lookup,
                allocations_count: u32::try_from(allocations.len()).expect("allocations_count bigger than u32::MAX"),
                allocations,
                compressed_data_size: u32::try_from(self.compressed_data.len()).expect("compressed_data_size bigger than u32::MAX"),
            };

            atlas_data.to_bytes(writer).trace::<Self>()?;
            writer.extend_from_slice(&self.compressed_data);

            Ok(())
        })
    }
}
