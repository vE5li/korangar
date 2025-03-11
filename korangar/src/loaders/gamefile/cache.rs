use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use blake3::Hash;
use ddsfile::{AlphaMode, D3D10ResourceDimension, Dds, DxgiFormat, NewDxgiParams};
use hashbrown::HashSet;
use image::{EncodableLayout, RgbaImage};
use rayon::prelude::*;

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

    let mut texture_files: Vec<String> = game_file_loader
        .get_files_with_extension(&[BMP_FILE_EXTENSION, JPG_FILE_EXTENSION, TGA_FILE_EXTENSION, PNG_FILE_EXTENSION])
        .drain(..)
        .filter(|file_name| file_name.starts_with(TEXTURE_PREFIX))
        .collect();

    texture_files.sort();
    texture_files.dedup();

    let current_archive_exists = fs::exists(path).unwrap_or(false);

    let (mut unused_textures, mut outdated_textures, mut new_textures): (Vec<&String>, Vec<&String>, Vec<&String>) = {
        if current_archive_exists {
            let current_archive = SevenZipArchive::from_path(path);

            let mut existing_dds_files = Vec::new();
            current_archive.get_files_with_extension(&mut existing_dds_files, &[DDS_FILE_EXTENSION]);
            let existing_dds_set: HashSet<String> = existing_dds_files.into_iter().collect();

            let texture_to_dds: HashSet<String> = texture_files
                .iter()
                .map(|texture_file| texture_file_dds_name(texture_file))
                .collect();
            let unused_textures: Vec<&String> = existing_dds_set
                .iter()
                .filter(|dds_file| !texture_to_dds.contains(dds_file.as_str()) && dds_file.as_str() != HASH_FILE_PATH)
                .collect();

            let outdated_textures: Vec<&String> = texture_files
                .par_iter()
                .filter(|texture_file_name| {
                    if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
                        return false;
                    }

                    let dds_name = texture_file_dds_name(texture_file_name);

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
                                        hash != dds_hash
                                    }
                                    Err(err) => {
                                        println!("Can't load texture file data: {err:?}");
                                        false
                                    }
                                }
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                })
                .collect();

            let new_textures: Vec<&String> = texture_files
                .par_iter()
                .filter(|texture_file_name| {
                    if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
                        return false;
                    }

                    let dds_name = texture_file_dds_name(texture_file_name);
                    !existing_dds_set.contains(&dds_name)
                })
                .collect();

            let unused_textures: Vec<&String> = unused_textures
                .par_iter()
                .filter_map(|unused_texture| {
                    texture_files
                        .iter()
                        .find(|texture_file| texture_file_dds_name(texture_file).as_str() == unused_texture.as_str())
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

    let mut to_create_textures: Vec<&String> = outdated_textures.iter().copied().chain(new_textures.iter().copied()).collect();
    to_create_textures.sort();
    to_create_textures.dedup();

    let outdated_textures: HashSet<&String> = outdated_textures.iter().copied().collect();
    let new_textures: HashSet<&String> = new_textures.iter().copied().collect();

    let _ = fs::remove_file(TEMPORARY_CACHE_FILE_NAME);

    let archive_path = match current_archive_exists {
        true => TEMPORARY_CACHE_FILE_NAME,
        false => CACHE_FILE_NAME,
    };

    let mut builder = Box::new(SevenZipArchiveBuilder::from_path(Path::new(archive_path)));
    builder.add_file(HASH_FILE_PATH, game_file_hash.to_hex().as_bytes().to_vec(), Compression::Off);

    if current_archive_exists {
        let current_archive = Box::new(SevenZipArchive::from_path(path));

        for texture_file_name in texture_files.iter() {
            if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
                return;
            }

            let dds_file_name = texture_file_dds_name(texture_file_name);

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
            let dds_file_name = texture_file_dds_name(texture_file_name);

            match texture_loader.load_texture_data(texture_name, false) {
                Ok((image, transparent))
                    if (image.height() % 4 == 0 && image.width() % 4 == 0) || (image.height() >= 48 && image.width() >= 48) =>
                {
                    compress_image(
                        &texture_loader,
                        &mut builder,
                        &mut created_count,
                        &mut updated_count,
                        &outdated_textures,
                        texture_file_name,
                        dds_file_name.as_str(),
                        image,
                        transparent,
                    );
                }
                Ok(_) => {
                    // We skip compressing textures if they would need to be cropped while also
                    // being very small, which would lead to noticeable cropping artifacts.
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

fn compress_image(
    texture_loader: &TextureLoader,
    builder: &mut SevenZipArchiveBuilder,
    created_count: &mut i32,
    updated_count: &mut i32,
    outdated_textures: &HashSet<&String>,
    texture_file_name: &String,
    dds_file_name: &str,
    image: RgbaImage,
    transparent: bool,
) {
    println!("Creating compressed texture for `{texture_file_name}`");
    let hash = blake3::hash(image.as_bytes());

    let image = crop_to_multiple_of_four(image);

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

    // `Compression::Off` currently gives the best load times with not too much
    // higher file sizes.
    builder.add_file(dds_file_name, dds_file_data, Compression::Off);

    if outdated_textures.contains(texture_file_name) {
        *updated_count += 1;
    } else {
        *created_count += 1;
    }
}

fn crop_to_multiple_of_four(mut image: RgbaImage) -> RgbaImage {
    let width = image.width();
    let height = image.height();

    let new_width = width - (width % 4);
    let new_height = height - (height % 4);

    if new_width == width && new_height == height {
        return image;
    }

    let x_offset = (width - new_width) / 2;
    let y_offset = (height - new_height) / 2;

    image::imageops::crop(&mut image, x_offset, y_offset, new_width, new_height).to_image()
}

fn finish_archive(current_archive_exists: bool, builder: Box<SevenZipArchiveBuilder>) {
    // Drop to finish the writing to the new archive.
    drop(builder);

    if current_archive_exists {
        let _ = fs::rename(TEMPORARY_CACHE_FILE_NAME, CACHE_FILE_NAME);
    }
}

pub fn texture_file_dds_name(image_file_name: &str) -> String {
    format!("{image_file_name}{DDS_FILE_EXTENSION}")
}
