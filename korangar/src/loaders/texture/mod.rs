use std::io::Cursor;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use blake3::{Hash, Hasher};
use block_compression::{BC7Settings, CompressionVariant, GpuBlockCompressor};
use hashbrown::HashMap;
use image::codecs::tga::TgaEncoder;
use image::{GrayImage, ImageBuffer, ImageFormat, ImageReader, Rgba, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use korangar_util::color::contains_transparent_pixel;
use korangar_util::container::SimpleCache;
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation, OfflineTextureAtlas};
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, Device, Extent3d, Maintain, MapMode, Queue,
    TexelCopyBufferLayout, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use super::error::LoadError;
use super::{CachedTextureAtlas, FALLBACK_BMP_FILE, FALLBACK_JPEG_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE, MIP_LEVELS};
use crate::graphics::{Lanczos3Drawer, MipMapRenderPassContext, Texture};
use crate::loaders::GameFileLoader;

const MAX_CACHE_COUNT: u32 = 512;
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
}

impl TextureLoader {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, game_file_loader: Arc<GameFileLoader>) -> Self {
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
        }
    }

    fn create_raw(
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
        self.create_raw(
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
        self.create_raw(
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
        self.create_raw(
            name,
            image.width(),
            image.height(),
            1,
            TextureFormat::Rgba8Unorm,
            false,
            image.as_raw(),
        )
    }

    pub(crate) fn create_uncompressed_with_mipmaps(
        &self,
        name: &str,
        mips_level: u32,
        transparent: bool,
        image: RgbaImage,
    ) -> Arc<Texture> {
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
                mip_level_count: MIP_LEVELS,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            image.as_raw(),
            transparent,
        );

        if mips_level > 1 {
            let mut mip_views = Vec::with_capacity(mips_level as usize);

            for level in 0..mips_level {
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

            for index in 0..(mips_level - 1) as usize {
                let mut pass =
                    self.mip_map_render_context
                        .create_pass(&self.device, &mut encoder, &mip_views[index], &mip_views[index + 1]);

                self.lanczos3_drawer.draw(&mut pass);
            }

            self.queue.submit(Some(encoder.finish()));
        }

        Arc::new(texture)
    }

    pub(crate) fn create_compressed_with_mipmaps(&self, mips_level: u32, image: RgbaImage) -> Vec<u8> {
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
                mip_level_count: mips_level,
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

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("compression encoder"),
        });

        let mut mip_views = Vec::with_capacity(mips_level as usize);
        let variant = CompressionVariant::BC7(BC7Settings::alpha_slow());

        let mut total_size = 0;
        let mut offsets = Vec::with_capacity(mips_level as usize);

        for level in 0..mips_level {
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

        for index in 0..(mips_level - 1) as usize {
            let mut pass = self
                .mip_map_render_context
                .create_pass(&self.device, &mut encoder, &mip_views[index], &mip_views[index + 1]);
            self.lanczos3_drawer.draw(&mut pass);
        }

        let output_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("compressed output buffer"),
            size: total_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut block_compressor = self.block_compressor.lock().unwrap();

        for level in 0..mips_level {
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
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("texture compression pass"),
                timestamp_writes: None,
            });
            block_compressor.compress(&mut pass);
        }

        drop(block_compressor);

        self.queue.submit([encoder.finish()]);

        self.download_blocks_data(output_buffer)
    }

    fn download_blocks_data(&self, block_buffer: Buffer) -> Vec<u8> {
        let size = block_buffer.size();

        let staging_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("staging buffer"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut copy_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("copy encoder"),
        });

        copy_encoder.copy_buffer_to_buffer(&block_buffer, 0, &staging_buffer, 0, size);

        self.queue.submit([copy_encoder.finish()]);

        let result;

        {
            let buffer_slice = staging_buffer.slice(..);

            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(MapMode::Read, move |v| tx.send(v).unwrap());

            self.device.poll(Maintain::Wait);

            match rx.recv() {
                Ok(Ok(())) => {
                    result = buffer_slice.get_mapped_range().to_vec();
                }
                _ => panic!("couldn't read from buffer"),
            }
        }

        staging_buffer.unmap();

        result
    }

    pub fn load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let texture = match image_type {
            ImageType::Color => {
                let (texture_data, transparent) = self.load_texture_data(path, false)?;
                self.create_color(path, texture_data, transparent)
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

    pub fn load_texture_data(&self, path: &str, raw: bool) -> Result<(RgbaImage, bool), LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture data from {}", path.magenta()));

        let path = Self::fix_broken_file_endings(path);

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

    fn fix_broken_file_endings(path: &str) -> String {
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

fn premultiply_alpha(image_buffer: RgbaImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    // Iterating over "pixels_mut()" is considerably slower than iterating over the
    // raw bates, so we have to do this conversion to get raw, mutable access.
    let width = image_buffer.width();
    let height = image_buffer.height();
    let mut bytes = image_buffer.into_raw();

    korangar_util::color::premultiply_alpha(&mut bytes);

    RgbaImage::from_raw(width, height, bytes).unwrap()
}

#[derive(Copy, Clone)]
pub struct TextureAtlasEntry {
    pub allocation_id: AllocationId,
    pub transparent: bool,
}

pub trait TextureAtlas {
    /// Registers the given texture by its path. Will return an allocation ID
    /// which can later be used to get the actual allocation and flag that shows
    /// if a texture contains transparent pixels.
    fn register(&mut self, path: &str) -> TextureAtlasEntry;

    fn build_atlas(&mut self);

    fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation>;
    fn create_texture(&mut self, texture_loader: &TextureLoader) -> Arc<Texture>;
}

pub struct UncompressedTextureAtlas {
    texture_loader: Arc<TextureLoader>,
    offline_atlas: OfflineTextureAtlas,
    lookup: HashMap<String, TextureAtlasEntry>,
    name: String,
    create_mip_map: bool,
    transparent: bool,
    hasher: Option<Hasher>,
}

impl UncompressedTextureAtlas {
    pub fn new(
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        create_mip_map: bool,
        calculate_hash: bool,
    ) -> Self {
        let mip_level_count = if create_mip_map { NonZeroU32::new(MIP_LEVELS) } else { None };

        Self {
            offline_atlas: OfflineTextureAtlas::new(add_padding, mip_level_count),
            texture_loader,
            lookup: HashMap::default(),
            name: name.into(),
            create_mip_map,
            transparent: false,
            hasher: if calculate_hash { Some(Hasher::default()) } else { None },
        }
    }

    #[cfg(feature = "debug")]
    pub fn create_from_group(
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        paths: &[&str],
    ) -> (Vec<AtlasAllocation>, Arc<Texture>) {
        let mut factory = Self::new(texture_loader.clone(), name, add_padding, false, false);

        let mut ids: Vec<TextureAtlasEntry> = paths.iter().map(|path| factory.register(path)).collect();
        factory.offline_atlas.build_atlas();

        let mapping = ids
            .drain(..)
            .map(|entry| factory.get_allocation(entry.allocation_id).unwrap())
            .collect();

        let texture = factory.create_texture(&texture_loader);

        (mapping, texture)
    }

    pub fn to_cached_texture_atlas(mut self) -> CachedTextureAtlas {
        let hash = self.hash();
        self.build_atlas();

        let lookup = self.lookup;
        let allocations = self.offline_atlas.get_allocations();
        let rgba_image = self.offline_atlas.get_atlas();

        let mut uncompressed_data = Vec::new();
        rgba_image
            .write_with_encoder(TgaEncoder::new(&mut uncompressed_data))
            .expect("can't encode texture atlas as TGA file");

        let width = rgba_image.width();
        let height = rgba_image.height();
        let compressed_data = self.texture_loader.create_compressed_with_mipmaps(MIP_LEVELS, rgba_image);

        CachedTextureAtlas {
            hash,
            name: self.name,
            width,
            height,
            mipmaps_count: MIP_LEVELS,
            transparent: self.transparent,
            lookup,
            allocations,
            compressed_data,
        }
    }

    pub fn hash(&self) -> Hash {
        match self.hasher.as_ref() {
            None => Hash::from_bytes([0; 32]),
            Some(hasher) => hasher.finalize(),
        }
    }
}

impl TextureAtlas for UncompressedTextureAtlas {
    fn register(&mut self, path: &str) -> TextureAtlasEntry {
        if let Some(cached_entry) = self.lookup.get(path).copied() {
            return cached_entry;
        }

        let (data, transparent) = self.texture_loader.load_texture_data(path, false).expect("can't load texture data");
        self.transparent |= transparent;

        if let Some(hasher) = &mut self.hasher {
            hasher.update(data.as_raw());
        }

        let allocation_id = self.offline_atlas.register_image(data);

        let entry = TextureAtlasEntry {
            allocation_id,
            transparent,
        };
        self.lookup.insert(path.to_string(), entry);

        entry
    }

    fn build_atlas(&mut self) {
        self.offline_atlas.build_atlas();
    }

    fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        self.offline_atlas.get_allocation(allocation_id)
    }

    fn create_texture(&mut self, texture_loader: &TextureLoader) -> Arc<Texture> {
        self.build_atlas();

        let name = format!("{} texture atlas", self.name);

        let mips_level = match self.create_mip_map {
            true => MIP_LEVELS,
            false => 1,
        };

        texture_loader.create_uncompressed_with_mipmaps(&name, mips_level, self.transparent, self.offline_atlas.get_atlas())
    }
}

impl TextureAtlas for CachedTextureAtlas {
    fn register(&mut self, path: &str) -> TextureAtlasEntry {
        match self.lookup.get(path).copied() {
            None => {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] Texture not found in cached texture atlas. Please re-sync or delete the cache",
                    "error".red()
                );
                self.lookup.values().copied().take(1).next().expect("lookup is empty")
            }
            Some(path) => path,
        }
    }

    fn build_atlas(&mut self) {
        /* Nothing to do */
    }

    fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        self.allocations.get(allocation_id).copied()
    }

    fn create_texture(&mut self, texture_loader: &TextureLoader) -> Arc<Texture> {
        texture_loader.create_raw(
            &self.name,
            self.width,
            self.height,
            self.mipmaps_count,
            TextureFormat::Bc7RgbaUnormSrgb,
            self.transparent,
            &self.compressed_data,
        )
    }
}
