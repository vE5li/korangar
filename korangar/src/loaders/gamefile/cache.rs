use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use blake3::Hash;
use ddsfile::{AlphaMode, D3D10ResourceDimension, Dds, DxgiFormat, NewDxgiParams};
use hashbrown::HashSet;
use image::{EncodableLayout, RgbaImage};
use korangar_util::FileLoader;
use rayon::prelude::*;

use crate::SHUTDOWN_SIGNAL;
use crate::loaders::archive::seven_zip::{SevenZipArchive, SevenZipArchiveBuilder};
use crate::loaders::archive::{Archive, Compression, Writable};
use crate::loaders::texture::calculate_valid_mip_level_count;
use crate::loaders::{CACHE_FILE_NAME, GameFileLoader, HASH_FILE_PATH, TEMPORARY_CACHE_FILE_NAME, TextureLoader};

const BIK_FILE_EXTENSION: &str = ".bik";
const BMP_FILE_EXTENSION: &str = ".bmp";
const JPG_FILE_EXTENSION: &str = ".jpg";
const TGA_FILE_EXTENSION: &str = ".tga";
const PNG_FILE_EXTENSION: &str = ".png";

const DDS_FILE_EXTENSION: &str = ".dds";
const H264_FILE_EXTENSION: &str = ".h264";
const TEXTURE_PREFIX: &str = "data\\texture\\";
const VIDEO_PREFIX: &str = "data\\video\\";

enum MediaType {
    Texture,
    Video,
}

#[derive(Default)]
struct ProcessingCounts {
    created: u32,
    skipped: u32,
    error: u32,
}

fn is_ffmpeg_available() -> bool {
    match Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

pub fn sync_cache_archive(game_file_loader: &GameFileLoader, texture_loader: Arc<TextureLoader>, game_file_hash: Hash) {
    println!("Starting sync of cache");
    let ffmpeg_available = is_ffmpeg_available();

    if !ffmpeg_available {
        println!("FFmpeg not found. Video re-encoding will be skipped");
    }

    let path = Path::new(CACHE_FILE_NAME);
    let current_archive_exists = fs::exists(path).unwrap_or(false);

    println!("Collecting all media files");
    let texture_files = collect_files(game_file_loader, MediaType::Texture);
    let video_files = if ffmpeg_available {
        collect_files(game_file_loader, MediaType::Video)
    } else {
        Vec::new()
    };

    let texture_to_process = analyze_files(
        &texture_files,
        current_archive_exists,
        path,
        MediaType::Texture,
        game_file_loader,
        Some(&texture_loader),
    );

    let video_to_process = analyze_files(
        &video_files,
        current_archive_exists,
        path,
        MediaType::Video,
        game_file_loader,
        None,
    );

    let _ = fs::remove_file(TEMPORARY_CACHE_FILE_NAME);
    let archive_path = match current_archive_exists {
        true => TEMPORARY_CACHE_FILE_NAME,
        false => CACHE_FILE_NAME,
    };

    let mut builder = Box::new(SevenZipArchiveBuilder::from_path(Path::new(archive_path)));
    builder.add_file(HASH_FILE_PATH, game_file_hash.to_hex().as_bytes().to_vec(), Compression::Off);

    if current_archive_exists {
        let current_archive = Box::new(SevenZipArchive::from_path(path));
        copy_existing_files(
            &mut builder,
            &current_archive,
            &texture_files,
            &texture_to_process,
            MediaType::Texture,
        );
        copy_existing_files(
            &mut builder,
            &current_archive,
            &video_files,
            &video_to_process,
            MediaType::Video,
        );
    }

    let texture_counts = process_media_files(
        &texture_to_process,
        &mut builder,
        MediaType::Texture,
        game_file_loader,
        Some(&texture_loader),
    );

    let video_counts = process_media_files(&video_to_process, &mut builder, MediaType::Video, game_file_loader, None);

    finish_archive(current_archive_exists, builder);

    println!("Cache sync finished");
    println!(
        "Textures - Created: {} Skipped: {} Errors: {}",
        texture_counts.created, texture_counts.skipped, texture_counts.error
    );
    println!(
        "Videos - Created: {} Skipped: {} Errors: {}",
        video_counts.created, video_counts.skipped, video_counts.error
    );

    if !ffmpeg_available && !video_files.is_empty() {
        println!("No videos re-encoded. Is FFmpeg installed and added to PATH?");
    }
}

fn collect_files(game_file_loader: &GameFileLoader, media_type: MediaType) -> Vec<String> {
    let mut files: Vec<String> = match media_type {
        MediaType::Texture => game_file_loader
            .get_files_with_extension(&[BMP_FILE_EXTENSION, JPG_FILE_EXTENSION, TGA_FILE_EXTENSION, PNG_FILE_EXTENSION])
            .drain(..)
            .filter(|file_name| file_name.starts_with(TEXTURE_PREFIX))
            .collect(),
        MediaType::Video => game_file_loader
            .get_files_with_extension(&[BIK_FILE_EXTENSION])
            .drain(..)
            .filter(|file_name| file_name.starts_with(VIDEO_PREFIX))
            .collect(),
    };
    files.sort();
    files.dedup();
    files
}

fn analyze_files(
    source_files: &[String],
    current_archive_exists: bool,
    archive_path: &Path,
    media_type: MediaType,
    game_file_loader: &GameFileLoader,
    texture_loader: Option<&Arc<TextureLoader>>,
) -> Vec<String> {
    if !current_archive_exists {
        return source_files.to_vec();
    }

    let current_archive = SevenZipArchive::from_path(archive_path);

    let extension = match media_type {
        MediaType::Texture => DDS_FILE_EXTENSION,
        MediaType::Video => H264_FILE_EXTENSION,
    };

    let mut existing_files = Vec::new();
    current_archive.get_files_with_extension(&mut existing_files, &[extension]);
    let existing_set: HashSet<String> = existing_files.into_iter().collect();

    source_files
        .par_iter()
        .filter(|source_file| {
            if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
                return false;
            }

            let target_name = get_target_filename(source_file, &media_type);

            if !existing_set.contains(&target_name) {
                return true;
            }

            // Check if file is outdated
            match current_archive.get_file_by_path(&target_name) {
                Some(cached_file) if cached_file.len() >= blake3::OUT_LEN => {
                    println!("Checking file '{target_name}'");

                    let mut hash_bytes = [0; blake3::OUT_LEN];
                    let size = cached_file.len();

                    hash_bytes.copy_from_slice(&cached_file[size - blake3::OUT_LEN..]);
                    let cached_hash = Hash::from_bytes(hash_bytes);

                    match media_type {
                        MediaType::Texture => {
                            if let (Some(texture_loader), Some(texture_name)) = (texture_loader, source_file.strip_prefix(TEXTURE_PREFIX)) {
                                match texture_loader.load_texture_data(texture_name, false) {
                                    Ok((image, _)) => {
                                        let hash = blake3::hash(image.as_bytes());
                                        hash != cached_hash
                                    }
                                    Err(_) => false,
                                }
                            } else {
                                false
                            }
                        }
                        MediaType::Video => match game_file_loader.get(source_file) {
                            Ok(data) => {
                                let hash = blake3::hash(&data);
                                hash != cached_hash
                            }
                            Err(_) => false,
                        },
                    }
                }
                _ => false,
            }
        })
        .map(|s| s.to_string())
        .collect()
}

fn copy_existing_files(
    builder: &mut SevenZipArchiveBuilder,
    current_archive: &SevenZipArchive,
    source_files: &[String],
    to_process: &[String],
    media_type: MediaType,
) {
    let to_process_set: HashSet<&str> = to_process.iter().map(|s| s.as_str()).collect();

    for source_file in source_files {
        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            return;
        }

        if to_process_set.contains(source_file.as_str()) {
            // Skip files that need processing.
            continue;
        }

        let target_file = get_target_filename(source_file, &media_type);

        if current_archive.file_exists(&target_file) {
            let file_type = match media_type {
                MediaType::Texture => "compressed texture",
                MediaType::Video => "encoded video",
            };

            println!("Copying existing {} `{}`", file_type, target_file);
            builder.copy_file_from_archive(current_archive, &target_file);
        }
    }
}

fn process_media_files(
    to_process: &[String],
    builder: &mut SevenZipArchiveBuilder,
    media_type: MediaType,
    game_file_loader: &GameFileLoader,
    texture_loader: Option<&Arc<TextureLoader>>,
) -> ProcessingCounts {
    let mut counts = ProcessingCounts::default();

    for source_file in to_process {
        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            let media = match media_type {
                MediaType::Texture => "Textures",
                MediaType::Video => "Videos",
            };
            println!("Cache sync aborted");
            println!(
                "{} - Created: {} Skipped: {} Errors: {}",
                media, counts.created, counts.skipped, counts.error
            );
            return counts;
        }

        let target_file = get_target_filename(source_file, &media_type);

        match media_type {
            MediaType::Texture => {
                if let (Some(texture_loader), Some(texture_name)) = (texture_loader, source_file.strip_prefix(TEXTURE_PREFIX)) {
                    match texture_loader.load_texture_data(texture_name, false) {
                        Ok((image, transparent))
                            if (image.height() % 4 == 0 && image.width() % 4 == 0) || (image.height() >= 48 && image.width() >= 48) =>
                        {
                            process_texture(
                                texture_loader,
                                builder,
                                &mut counts.created,
                                source_file,
                                &target_file,
                                image,
                                transparent,
                            );
                        }
                        Ok(_) => {
                            counts.skipped += 1;
                        }
                        Err(error) => {
                            println!("Failed to load texture for `{source_file}`: {error:?}");
                            counts.error += 1;
                        }
                    }
                }
            }
            MediaType::Video => match game_file_loader.get(source_file) {
                Ok(bik_data) => {
                    process_video(builder, &mut counts.created, source_file, &target_file, bik_data);
                }
                Err(error) => {
                    println!("Failed to load video for `{source_file}`: {error:?}");
                    counts.error += 1;
                }
            },
        }
    }

    counts
}

fn get_target_filename(source_file: &str, media_type: &MediaType) -> String {
    match media_type {
        MediaType::Texture => texture_file_dds_name(source_file),
        MediaType::Video => video_file_h264_name(source_file),
    }
}

fn process_texture(
    texture_loader: &TextureLoader,
    builder: &mut SevenZipArchiveBuilder,
    created_count: &mut u32,
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

    *created_count += 1;
}

fn process_video(
    builder: &mut SevenZipArchiveBuilder,
    created_count: &mut u32,
    bik_file_name: &String,
    h264_file_name: &str,
    bik_data: Vec<u8>,
) {
    println!("Encoding video for `{bik_file_name}`");
    let hash = blake3::hash(&bik_data);

    let cmd = Command::new("ffmpeg")
        .arg("-i")
        .arg("pipe:0")
        .arg("-profile")
        .arg("main")
        .arg("-crf")
        .arg("18")
        .arg("-preset")
        .arg("slow")
        .arg("-pix_fmt")
        .arg("nv12")
        .arg("-f")
        .arg("h264")
        .arg("pipe:1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match cmd {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                std::thread::spawn(move || {
                    if let Err(error) = stdin.write_all(&bik_data) {
                        println!("Failed to write to FFmpeg stdin: {error:?}");
                        return;
                    }

                    if let Err(error) = stdin.flush() {
                        println!("Failed to flush FFmpeg stdin: {error:?}");
                    }
                });
            }

            match child.wait_with_output() {
                Ok(output) => {
                    if output.status.success() {
                        let mut h264_data = output.stdout;

                        h264_data.extend_from_slice(hash.as_bytes());

                        builder.add_file(h264_file_name, h264_data, Compression::Off);

                        *created_count += 1;
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        println!("FFmpeg failed to encode `{bik_file_name}`: {error}");
                    }
                }
                Err(error) => {
                    println!("Failed to get FFmpeg output: {error:?}");
                }
            }
        }
        Err(error) => {
            println!("Failed to start FFmpeg: {error:?}");
        }
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
    drop(builder);

    if current_archive_exists {
        let _ = fs::rename(TEMPORARY_CACHE_FILE_NAME, CACHE_FILE_NAME);
    }
}

pub fn texture_file_dds_name(image_file_name: &str) -> String {
    format!("{image_file_name}{DDS_FILE_EXTENSION}")
}

pub fn video_file_h264_name(bik_file_name: &str) -> String {
    format!("{bik_file_name}{H264_FILE_EXTENSION}")
}
