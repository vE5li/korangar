use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use blake3::Hash;
use ddsfile::{AlphaMode, D3D10ResourceDimension, Dds, DxgiFormat, NewDxgiParams};
use hashbrown::HashSet;
use image::EncodableLayout;

use crate::SHUTDOWN_SIGNAL;
use crate::loaders::archive::seven_zip::{SevenZipArchive, SevenZipArchiveBuilder};
use crate::loaders::archive::{Archive, Compression, Writable};
use crate::loaders::texture::calculate_valid_mip_level_count;
use crate::loaders::{CACHE_FILE_NAME, GameFileLoader, HASH_FILE_PATH, TEMPORARY_CACHE_FILE_NAME, TextureLoader};

const BMP_FILE_EXTENSION: &str = ".bmp";
const JPG_FILE_EXTENSION: &str = ".jpg";
const TGA_FILE_EXTENSION: &str = ".tga";
const PNG_FILE_EXTENSION: &str = ".png";

const DDS_FILE_EXTENSION: &str = ".dds";
const TEXTURE_PREFIX: &str = "data\\texture\\";

pub fn sync_cache_archive(game_file_loader: &GameFileLoader, texture_loader: Arc<TextureLoader>, game_file_hash: Hash) {
    println!("Starting sync of cache");

    let path = Path::new(CACHE_FILE_NAME);

    println!("Collecting all texture file names");

    let mut texture_files = game_file_loader
        .get_files_with_extension(&[BMP_FILE_EXTENSION, JPG_FILE_EXTENSION, TGA_FILE_EXTENSION, PNG_FILE_EXTENSION])
        .drain(..)
        .filter(|file_name| file_name.starts_with(TEXTURE_PREFIX))
        .collect::<Vec<String>>();

    texture_files.sort();
    texture_files.dedup();

    let current_archive_exists = fs::exists(path).unwrap_or(false);
    let mut existing_dds_files = Vec::new();

    let (mut unused_textures, mut outdated_textures, mut new_textures): (Vec<&String>, Vec<&String>, Vec<&String>) = {
        if current_archive_exists {
            let current_archive = SevenZipArchive::from_path(path);
            current_archive.get_files_with_extension(&mut existing_dds_files, &[DDS_FILE_EXTENSION]);

            let existing_dds_set: HashSet<String> = existing_dds_files.into_iter().collect();
            let texture_to_dds: HashSet<String> = texture_files.iter().map(|texture_file| dds_file_name(texture_file)).collect();
            let unused_textures = existing_dds_set
                .iter()
                .filter(|dds_file| !texture_to_dds.contains(*dds_file) && *dds_file != HASH_FILE_PATH)
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            let mut outdated_textures = Vec::new();
            let mut new_textures = Vec::new();

            for texture_file_name in &texture_files {
                let dds_name = dds_file_name(texture_file_name);

                match current_archive.get_file_by_path(&dds_name) {
                    Some(dds_file) if dds_file.len() >= blake3::OUT_LEN => {
                        println!("Checking compressed texture '{dds_name}'");

                        let mut dds_hash_bytes = [0; blake3::OUT_LEN];
                        let size = dds_file.len();

                        dds_hash_bytes.copy_from_slice(&dds_file[size - blake3::OUT_LEN..]);
                        let dds_hash = Hash::from_bytes(dds_hash_bytes);

                        if let Some(texture_name) = texture_file_name.strip_prefix(TEXTURE_PREFIX) {
                            match texture_loader.load_texture_data(texture_name, false) {
                                Ok((image, _)) => {
                                    let hash = blake3::hash(image.as_bytes());
                                    if hash != dds_hash {
                                        outdated_textures.push(texture_file_name);
                                    }
                                }
                                Err(err) => {
                                    println!("Can't load texture file data: {err:?}");
                                }
                            }
                        }
                    }
                    _ => {
                        new_textures.push(texture_file_name);
                    }
                }
            }

            let unused_textures: Vec<&String> = unused_textures
                .iter()
                .filter_map(|unused_texture| {
                    texture_files
                        .iter()
                        .find(|texture_file| dds_file_name(texture_file) == *unused_texture)
                })
                .collect();

            (unused_textures, outdated_textures, new_textures)
        } else {
            (Vec::new(), Vec::new(), texture_files.iter().collect())
        }
    };

    unused_textures.sort();
    outdated_textures.sort();
    new_textures.sort();

    let mut created_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    let mut to_create_textures = Vec::from_iter(outdated_textures.iter().copied().chain(new_textures.iter().copied()));
    to_create_textures.sort();
    to_create_textures.dedup();

    let outdated_textures: HashSet<&String> = HashSet::from_iter(outdated_textures.iter().copied());
    let new_textures: HashSet<&String> = HashSet::from_iter(new_textures.iter().copied());

    let _ = fs::remove_file(TEMPORARY_CACHE_FILE_NAME);

    let archive_path = match current_archive_exists {
        true => TEMPORARY_CACHE_FILE_NAME,
        false => CACHE_FILE_NAME,
    };

    let mut builder = Box::new(SevenZipArchiveBuilder::from_path(Path::new(archive_path)));
    builder.add_file(HASH_FILE_PATH, game_file_hash.to_hex().as_bytes().to_vec(), Compression::No);

    if current_archive_exists {
        let current_archive = Box::new(SevenZipArchive::from_path(path));

        for texture_file_name in texture_files.iter() {
            let dds_file_name = dds_file_name(texture_file_name);

            if current_archive.file_exists(&dds_file_name)
                && !outdated_textures.contains(&texture_file_name)
                && !new_textures.contains(&texture_file_name)
            {
                println!("Copying existing compressed texture `{dds_file_name}`");
                builder.copy_file_from_archive(&current_archive, &dds_file_name);
            }
        }
    }

    for &texture_file_name in to_create_textures.iter() {
        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            finish_archive(current_archive_exists, builder);
            println!(
                "Cache sync aborted. Created: {created_count} Updated: {updated_count} Skipped: {skipped_count} Errors: {error_count}"
            );
            return;
        }

        if let Some(texture_name) = texture_file_name.strip_prefix(TEXTURE_PREFIX) {
            let dds_file_name = dds_file_name(texture_file_name);

            match texture_loader.load_texture_data(texture_name, false) {
                Ok((image, transparent)) if image.height() % 4 == 0 && image.width() % 4 == 0 => {
                    println!("Creating compressed texture for `{texture_file_name}`");
                    let hash = blake3::hash(image.as_bytes());

                    let width = image.width();
                    let height = image.height();
                    let mip_level_count = calculate_valid_mip_level_count(width, height);

                    let mut dds = Dds::new_dxgi(NewDxgiParams {
                        height,
                        width,
                        depth: None,
                        format: DxgiFormat::BC7_UNorm_sRGB,
                        mipmap_levels: Some(mip_level_count),
                        array_layers: None,
                        caps2: None,
                        is_cubemap: false,
                        resource_dimension: D3D10ResourceDimension::Texture2D,
                        alpha_mode: match transparent {
                            // We use the alpha_mode field to encode if a texture has semi-transparent pixels.
                            // This is fine, since applying pre-multiplied alpha to our opaque texture won't
                            // change it's visible content.
                            true => AlphaMode::PreMultiplied,
                            false => AlphaMode::Straight,
                        },
                    })
                    .expect("can't create DDS file");
                    texture_loader.create_compressed_with_mipmaps(image, mip_level_count, &mut dds.data);

                    let mut dds_file_data = Vec::with_capacity(dds.data.len() + 512);
                    dds.write(&mut dds_file_data).expect("can't write DDS file");
                    dds_file_data.write_all(hash.as_bytes()).expect("can't append hash");

                    builder.add_file(dds_file_name.as_str(), dds_file_data, Compression::Fast);

                    if outdated_textures.contains(texture_file_name) {
                        updated_count += 1;
                    } else {
                        created_count += 1;
                    }
                }
                Ok(_) => {
                    // We only can compress textures which both sides can be divided
                    // by 4 (because BC7 is compression blocks of 4 pixels). Everything
                    // else is skipped.
                    skipped_count += 1;
                }
                Err(err) => {
                    println!("Failed to load texture for `{texture_file_name}`: {err:?}");
                    error_count += 1;
                }
            }
        }
    }

    finish_archive(current_archive_exists, builder);

    println!("Cache sync finished. Created: {created_count} Updated: {updated_count} Skipped: {skipped_count} Errors: {error_count}");
}

fn finish_archive(current_archive_exists: bool, builder: Box<SevenZipArchiveBuilder>) {
    // Drop to finish the writing to the new archive.
    drop(builder);

    if current_archive_exists {
        let _ = fs::rename(TEMPORARY_CACHE_FILE_NAME, CACHE_FILE_NAME);
    }
}

fn dds_file_name(image_file_name: &str) -> String {
    let char_count = image_file_name.chars().count();
    let mut name: String = image_file_name.chars().take(char_count - 4).collect();
    name.push_str(DDS_FILE_EXTENSION);
    name
}
