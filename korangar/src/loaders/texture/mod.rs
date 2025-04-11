use std::io::{Cursor, Read};
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use block_compression::{BC7Settings, CompressionVariant, GpuBlockCompressor};
use hashbrown::HashMap;
use image::{GrayImage, ImageBuffer, ImageFormat, ImageReader, Rgba, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use korangar_util::color::contains_transparent_pixel;
use korangar_util::container::SimpleCache;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, Device, Extent3d, MapMode, PollError,
    PollStatus, PollType, Queue, TexelCopyBufferLayout, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

use super::error::LoadError;
use super::{
    FALLBACK_BMP_FILE, FALLBACK_JPEG_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE, VideoLoader, fix_broken_texture_file_endings,
    texture_file_dds_name,
};
use crate::SHUTDOWN_SIGNAL;
use crate::graphics::{BindlessSupport, Capabilities, Lanczos3Drawer, MipMapRenderPassContext, Texture, TextureSet};
use crate::loaders::GameFileLoader;
use crate::world::Video;

const MAX_CACHE_COUNT: u32 = 4096;
const MAX_CACHE_SIZE: usize = 512 * 1024 * 1024;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ImageType {
    Color,
    Sdf,
    Msdf,
}

pub struct TextureLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    mip_map_render_context: MipMapRenderPassContext,
    lanczos3_drawer: Lanczos3Drawer,
    block_compressor: Mutex<GpuBlockCompressor>,
    cache: Mutex<SimpleCache<(String, ImageType), Arc<Texture>>>,
    bindless_support: BindlessSupport,
    supports_texture_compression: bool,
    max_textures_per_shader_stage: u32,
}

impl TextureLoader {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, capabilities: &Capabilities, game_file_loader: Arc<GameFileLoader>) -> Self {
        let lanczos3_drawer = Lanczos3Drawer::new(&device);
        let block_compressor = Mutex::new(GpuBlockCompressor::new(device.clone(), queue.clone()));

        Self {
            device,
            queue,
            game_file_loader,
            mip_map_render_context: MipMapRenderPassContext::default(),
            lanczos3_drawer,
            block_compressor,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
            bindless_support: capabilities.bindless_support(),
            supports_texture_compression: capabilities.supports_texture_compression(),
            max_textures_per_shader_stage: capabilities.get_max_textures_per_shader_stage(),
        }
    }

    pub fn create_raw(
        &self,
        name: &str,
        width: u32,
        height: u32,
        mip_level_count: u32,
        format: TextureFormat,
        transparent: bool,
    ) -> Arc<Texture> {
        let texture = Texture::new(
            &self.device,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            transparent,
        );
        Arc::new(texture)
    }

    pub fn create_raw_with_data(
        &self,
        name: &str,
        width: u32,
        height: u32,
        mip_level_count: u32,
        format: TextureFormat,
        transparent: bool,
        data: &[u8],
    ) -> Arc<Texture> {
        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            data,
            transparent,
        );
        Arc::new(texture)
    }

    pub fn create_color(&self, name: &str, image: RgbaImage, transparent: bool) -> Arc<Texture> {
        self.create_raw_with_data(
            name,
            image.width(),
            image.height(),
            1,
            TextureFormat::Rgba8UnormSrgb,
            transparent,
            image.as_raw(),
        )
    }

    pub fn create_sdf(&self, name: &str, image: GrayImage) -> Arc<Texture> {
        self.create_raw_with_data(
            name,
            image.width(),
            image.height(),
            1,
            TextureFormat::R8Unorm,
            false,
            image.as_raw(),
        )
    }

    pub(crate) fn create_msdf(&self, name: &str, image: RgbaImage) -> Arc<Texture> {
        self.create_raw_with_data(
            name,
            image.width(),
            image.height(),
            1,
            TextureFormat::Rgba8Unorm,
            false,
            image.as_raw(),
        )
    }

    pub(crate) fn create_uncompressed_with_mipmaps(&self, name: &str, transparent: bool, image: RgbaImage) -> Arc<Texture> {
        let width = image.width();
        let height = image.height();
        let mip_level_count = calculate_valid_mip_level_count(width, height);

        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(name),
                size: Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            image.as_raw(),
            transparent,
        );

        if mip_level_count > 1 {
            let mut mip_views = Vec::with_capacity(mip_level_count as usize);

            for level in 0..mip_level_count {
                let view = texture.get_texture().create_view(&TextureViewDescriptor {
                    label: Some(&format!("mip map level {level}")),
                    format: None,
                    dimension: Some(TextureViewDimension::D2),
                    usage: None,
                    aspect: TextureAspect::All,
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                });
                mip_views.push(view);
            }

            let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("TextureLoader"),
            });

            for index in 0..(mip_level_count - 1) as usize {
                let mut pass =
                    self.mip_map_render_context
                        .create_pass(&self.device, &mut encoder, &mip_views[index], &mip_views[index + 1]);

                self.lanczos3_drawer.draw(&mut pass);
            }

            self.queue.submit(Some(encoder.finish()));
        }

        Arc::new(texture)
    }

    pub(crate) fn create_compressed_with_mipmaps(&self, image: RgbaImage, mip_level_count: u32, compressed_data: &mut [u8]) {
        let width = image.width();
        let height = image.height();

        assert_eq!(width % 4, 0, "Texture width must be aligned to 4 pixels");
        assert_eq!(height % 4, 0, "Texture height must be aligned to 4 pixels");

        let temp_texture = Texture::new(
            &self.device,
            &TextureDescriptor {
                label: Some("temporary mip texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[TextureFormat::Rgba8Unorm],
            },
            // Doesn't matter
            true,
        );

        self.queue.write_texture(
            temp_texture.get_texture().as_image_copy(),
            &image,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let mut mip_views = Vec::with_capacity(mip_level_count as usize);
        let variant = CompressionVariant::BC7(BC7Settings::alpha_slow());

        let mut total_size = 0;
        let mut offsets = Vec::with_capacity(mip_level_count as usize);

        for level in 0..mip_level_count {
            let mip_width = width >> level;
            let mip_height = height >> level;

            offsets.push(total_size);
            total_size += variant.blocks_byte_size(mip_width, mip_height);

            let view = temp_texture.get_texture().create_view(&TextureViewDescriptor {
                label: Some(&format!("mip {level} view")),
                format: Some(TextureFormat::Rgba8UnormSrgb),
                dimension: Some(TextureViewDimension::D2),
                usage: None,
                aspect: TextureAspect::All,
                base_mip_level: level,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
            });

            mip_views.push(view);
        }

        let mut compression_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("compression encoder"),
        });

        for index in 0..(mip_level_count - 1) as usize {
            let mut pass =
                self.mip_map_render_context
                    .create_pass(&self.device, &mut compression_encoder, &mip_views[index], &mip_views[index + 1]);
            self.lanczos3_drawer.draw(&mut pass);
        }

        let output_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("compressed output buffer"),
            size: total_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut block_compressor = self.block_compressor.lock().unwrap();

        for level in 0..mip_level_count {
            let mip_width = width >> level;
            let mip_height = height >> level;
            let chunk_height = 128;
            let chunk_count = mip_height.div_ceil(chunk_height);
            let blocks_per_row = mip_width / 4;

            for chunk_index in 0..chunk_count {
                let current_chunk_height = if chunk_index == chunk_count - 1 {
                    mip_height - chunk_index * chunk_height
                } else {
                    chunk_height
                };

                let texture_y_offset = chunk_index * chunk_height;

                let blocks_offset = offsets[level as usize] + (chunk_index * chunk_height / 4 * blocks_per_row * 16) as usize;

                block_compressor.add_compression_task(
                    variant,
                    &temp_texture.get_texture().create_view(&TextureViewDescriptor {
                        label: Some(&format!("compression mip {level} view")),
                        format: Some(TextureFormat::Rgba8Unorm),
                        base_mip_level: level,
                        mip_level_count: Some(1),
                        dimension: Some(TextureViewDimension::D2),
                        usage: None,
                        aspect: TextureAspect::All,
                        base_array_layer: 0,
                        array_layer_count: Some(1),
                    }),
                    mip_width,
                    current_chunk_height,
                    &output_buffer,
                    Some(texture_y_offset),
                    Some(blocks_offset as u32),
                );
            }
        }

        {
            let mut pass = compression_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("texture compression pass"),
                timestamp_writes: None,
            });
            block_compressor.compress(&mut pass);
        }

        self.queue.submit([compression_encoder.finish()]);
        let _ = self.device.poll(PollType::Wait);

        self.download_blocks_data(output_buffer, compressed_data);
    }

    fn download_blocks_data(&self, block_buffer: Buffer, compressed_data: &mut [u8]) {
        let size = block_buffer.size();

        let staging_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("staging buffer"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut download_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("download encoder"),
        });

        download_encoder.copy_buffer_to_buffer(&block_buffer, 0, &staging_buffer, 0, size);

        self.queue.submit([download_encoder.finish()]);
        let _ = self.device.poll(PollType::Wait);

        {
            let buffer_slice = staging_buffer.slice(..);

            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(MapMode::Read, move |v| tx.send(v).unwrap());

            loop {
                match self.device.poll(PollType::Poll) {
                    Ok(PollStatus::Poll | PollStatus::WaitSucceeded) => {
                        // Check if shutdown initiated
                        if SHUTDOWN_SIGNAL.load(Ordering::Relaxed) {
                            staging_buffer.unmap();
                            return;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Ok(PollStatus::QueueEmpty) => break,
                    Err(PollError::Timeout) => panic!("timeout while waiting for blocks data download"),
                }
            }

            match rx.recv() {
                Ok(Ok(())) => {
                    let size = compressed_data.len();
                    compressed_data.copy_from_slice(&buffer_slice.get_mapped_range()[..size]);
                }
                _ => panic!("couldn't read from buffer"),
            }
        }

        staging_buffer.unmap();
    }

    pub fn load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let texture = match image_type {
            ImageType::Color => {
                let path = fix_broken_texture_file_endings(path);

                match self.try_load_compressed(&path) {
                    Some(compressed_texture) => compressed_texture,
                    None => {
                        let (texture_data, transparent) = self.load_texture_data(&path, false)?;
                        self.create_uncompressed_with_mipmaps(&path, transparent, texture_data)
                    }
                }
            }
            ImageType::Sdf => {
                let texture_data = self.load_grayscale_texture_data(path)?;
                self.create_sdf(path, texture_data)
            }
            ImageType::Msdf => {
                let (texture_data, _) = self.load_texture_data(path, true)?;
                self.create_msdf(path, texture_data)
            }
        };

        self.cache
            .lock()
            .as_mut()
            .unwrap()
            .insert((path.to_string(), image_type), texture.clone())
            .unwrap();

        Ok(texture)
    }

    fn try_load_compressed(&self, path: &str) -> Option<Arc<Texture>> {
        if !self.supports_texture_compression {
            return None;
        }

        let dds_file_name = texture_file_dds_name(path);
        let dds_file_path = format!("data\\texture\\{dds_file_name}");

        if !self.game_file_loader.file_exists(&dds_file_path) {
            return None;
        }

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load compressed texture data from {}", dds_file_path.magenta()));

        let dds_file_data = self.game_file_loader.get(&dds_file_path).ok()?;

        let Some(dds) = Dds::read_bc7(dds_file_data.as_slice()) else {
            #[cfg(feature = "debug")]
            print_debug!("Could not decode DDS file: {}", dds_file_path);
            return None;
        };

        let texture = self.create_raw_with_data(
            dds_file_name.as_str(),
            dds.width,
            dds.height,
            dds.num_mipmap_levels,
            TextureFormat::Bc7RgbaUnormSrgb,
            dds.alpha_mode == ddsfile::AlphaMode::PreMultiplied,
            dds.data,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        Some(texture)
    }

    pub fn load_texture_data(&self, path: &str, raw: bool) -> Result<(RgbaImage, bool), LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture data from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".jpg" | ".JPG" => ImageFormat::Jpeg,
            ".png" | ".PNG" => ImageFormat::Png,
            ".tga" | ".TGA" => ImageFormat::Tga,
            _ => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("File with unknown image format found: {:?}", path);
                    print_debug!("Replacing with fallback");
                }

                return self.load_texture_data(FALLBACK_PNG_FILE, raw);
            }
        };

        let file_data = match self.game_file_loader.get(&format!("data\\texture\\{path}")) {
            Ok(file_data) => file_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load_texture_data(FALLBACK_PNG_FILE, raw);
            }
        };
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let mut image_buffer = match reader.decode() {
            Ok(image) => image.to_rgba8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to decode image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Bmp => FALLBACK_BMP_FILE,
                    ImageFormat::Jpeg => FALLBACK_JPEG_FILE,
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.load_texture_data(fallback_path, raw);
            }
        };

        match image_format {
            ImageFormat::Bmp if !raw => {
                // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
                image_buffer
                    .pixels_mut()
                    .filter(|pixel| pixel.0[0] > 0xF0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0F)
                    .for_each(|pixel| *pixel = Rgba([0; 4]));
            }
            ImageFormat::Png | ImageFormat::Tga if !raw => {
                image_buffer = premultiply_alpha(image_buffer);
            }
            _ => {}
        }

        let transparent = match image_format == ImageFormat::Tga {
            true => contains_transparent_pixel(image_buffer.as_raw()),
            false => false,
        };

        #[cfg(feature = "debug")]
        timer.stop();

        Ok((image_buffer, transparent))
    }

    pub fn load_grayscale_texture_data(&self, path: &str) -> Result<GrayImage, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load grayscale texture data from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".png" | ".PNG" => ImageFormat::Png,
            ".tga" | ".TGA" => ImageFormat::Tga,
            _ => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("File with unknown image format found: {:?}", path);
                    print_debug!("Replacing with fallback");
                }

                return self.load_grayscale_texture_data(FALLBACK_PNG_FILE);
            }
        };

        let file_data = match self.game_file_loader.get(&format!("data\\texture\\{path}")) {
            Ok(file_data) => file_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load_grayscale_texture_data(FALLBACK_PNG_FILE);
            }
        };
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let image_buffer = match reader.decode() {
            Ok(image) => image.to_luma8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to decode image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.load_grayscale_texture_data(fallback_path);
            }
        };

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(image_buffer)
    }

    pub fn get(&self, path: &str, image_type: ImageType) -> Option<Arc<Texture>> {
        let mut lock = self.cache.lock().unwrap();
        lock.get(&(path.into(), image_type)).cloned()
    }

    pub fn get_or_load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let Some(texture) = self.cache.lock().unwrap().get(&(path.into(), image_type)).cloned() else {
            return self.load(path, image_type);
        };

        Ok(texture)
    }
}

struct Dds<'a> {
    width: u32,
    height: u32,
    num_mipmap_levels: u32,
    alpha_mode: ddsfile::AlphaMode,
    data: &'a [u8],
}

impl<'a> Dds<'a> {
    /// Custom DDS reader, since the upstream [`ddsfile::Dds`] does a needless
    /// copy of the data.
    fn read_bc7(mut data: &'a [u8]) -> Option<Self> {
        const DDS_MAGIC: u32 = 0x20534444; // b"DDS " in little endian

        let mut magic_bytes = [0; 4];
        if (&mut data).read_exact(&mut magic_bytes[..]).is_err() {
            #[cfg(feature = "debug")]
            print_debug!("Could not read magic bytes from DDS file");
        }
        let magic = u32::from_le_bytes(magic_bytes);

        if magic != DDS_MAGIC {
            #[cfg(feature = "debug")]
            print_debug!("DDS file has invalid magic bytes: {:?}", magic_bytes);
            return None;
        }

        let Ok(header) = ddsfile::Header::read(&mut data) else {
            #[cfg(feature = "debug")]
            print_debug!("DDS file has invalid header");
            return None;
        };

        let header10 = if header.spf.fourcc == Some(ddsfile::FourCC(<ddsfile::FourCC>::DX10)) {
            match ddsfile::Header10::read(&mut data) {
                Ok(header) => header,
                Err(_) => {
                    #[cfg(feature = "debug")]
                    print_debug!("Can't read header10 from DDS file");
                    return None;
                }
            }
        } else {
            #[cfg(feature = "debug")]
            print_debug!("DDS file has no valid header10");
            return None;
        };

        if header10.dxgi_format != ddsfile::DxgiFormat::BC7_UNorm_sRGB {
            #[cfg(feature = "debug")]
            print_debug!("DDS file is not a BC7 texture");
            return None;
        }

        let num_mipmap_levels = header.mip_map_count.unwrap_or(1);
        let mut total_size = 0;

        // BC7 format uses 4x4 pixel blocks with 16 bytes per block
        for level in 0..num_mipmap_levels {
            let mip_width = std::cmp::max(1, header.width >> level);
            let mip_height = std::cmp::max(1, header.height >> level);
            let blocks_wide = mip_width.div_ceil(4);
            let blocks_high = mip_height.div_ceil(4);
            let level_size = blocks_wide * blocks_high * 16;
            total_size += level_size;
        }

        let Some(data) = data.get(..total_size as usize) else {
            #[cfg(feature = "debug")]
            print_debug!("DDS file does not contain the expected data size");
            return None;
        };

        Some(Self {
            width: header.width,
            height: header.height,
            num_mipmap_levels,
            alpha_mode: header10.alpha_mode,
            data,
        })
    }
}

fn premultiply_alpha(image_buffer: RgbaImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // Iterating over "pixels_mut()" is considerably slower than iterating over the
    // raw bates, so we have to do this conversion to get raw, mutable access.
    let width = image_buffer.width();
    let height = image_buffer.height();
    let mut bytes = image_buffer.into_raw();

    korangar_util::color::premultiply_alpha(&mut bytes);

    RgbaImage::from_raw(width, height, bytes).unwrap()
}

pub struct TextureSetBuilder {
    texture_loader: Arc<TextureLoader>,
    video_loader: Arc<VideoLoader>,
    name: String,
    textures: Vec<Arc<Texture>>,
    videos: Vec<Video>,
    lookup: HashMap<String, i32>,
}

impl TextureSetBuilder {
    pub fn new(texture_loader: Arc<TextureLoader>, video_loader: Arc<VideoLoader>, name: impl Into<String>) -> Self {
        Self {
            texture_loader,
            video_loader,
            name: name.into(),
            textures: Vec::new(),
            videos: Vec::new(),
            lookup: HashMap::default(),
        }
    }

    #[cfg(feature = "debug")]
    pub fn build_from_group(
        texture_loader: Arc<TextureLoader>,
        video_loader: Arc<VideoLoader>,
        name: impl Into<String>,
        paths: &[&str],
    ) -> TextureSet {
        let mut factory = Self::new(texture_loader.clone(), video_loader, name);

        paths.iter().for_each(|path| {
            let _ = factory.register(path);
        });

        let (set, _) = factory.build();

        set
    }

    pub fn build(self) -> (TextureSet, Vec<Video>) {
        let set = TextureSet::new(
            &self.texture_loader.device,
            self.texture_loader.bindless_support,
            self.texture_loader.max_textures_per_shader_stage,
            &self.name,
            self.textures,
        );
        (set, self.videos)
    }

    #[must_use]
    pub fn register(&mut self, path: &str) -> (i32, bool) {
        if let Some(index) = self.lookup.get(path).copied() {
            let is_transparent = self.textures[index as usize].is_transparent();
            return (index, is_transparent);
        }

        let texture;

        match self.video_loader.is_video_file(path) {
            true => {
                let video = self.video_loader.load(path);
                texture = video.get_texture().clone();
                self.videos.push(video);
            }
            false => {
                texture = self.texture_loader.get_or_load(path, ImageType::Color).expect("can't load texture");
            }
        }

        let index = i32::try_from(self.textures.len()).expect("texture set is full");
        let is_transparent = texture.is_transparent();

        self.textures.push(texture);
        self.lookup.insert(path.to_string(), index);

        (index, is_transparent)
    }
}

/// This function can be used for both uncompressed and compressed textures.
pub fn calculate_valid_mip_level_count(width: u32, height: u32) -> u32 {
    let mut mip_level = 0;
    let mut current_width = width;
    let mut current_height = height;

    while current_width >= 4 && current_height >= 4 && current_width % 4 == 0 && current_height % 4 == 0 {
        mip_level += 1;
        current_width /= 2;
        current_height /= 2;
    }

    mip_level.max(1)
}
