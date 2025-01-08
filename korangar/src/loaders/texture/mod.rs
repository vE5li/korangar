use std::io::Cursor;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
use image::{GrayImage, ImageBuffer, ImageFormat, ImageReader, Rgba, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::color::contains_transparent_pixel;
use korangar_util::container::SimpleCache;
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation, OfflineTextureAtlas};
use korangar_util::FileLoader;
use wgpu::{
    CommandEncoderDescriptor, Device, Extent3d, Queue, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

use super::error::LoadError;
use super::{FALLBACK_BMP_FILE, FALLBACK_JPEG_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE, MIP_LEVELS};
use crate::graphics::{Lanczos3Drawer, MipMapRenderPassContext, Texture, TextureCompression};
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
    cache: Mutex<SimpleCache<(String, ImageType), Arc<Texture>>>,
}

impl TextureLoader {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, game_file_loader: Arc<GameFileLoader>) -> Self {
        let lanczos3_drawer = Lanczos3Drawer::new(&device);

        Self {
            device,
            queue,
            game_file_loader,
            mip_map_render_context: MipMapRenderPassContext::default(),
            lanczos3_drawer,
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

    pub(crate) fn create_with_mipmaps(
        &self,
        name: &str,
        texture_compression: TextureCompression,
        mips_level: u32,
        transparent: bool,
        image: RgbaImage,
    ) -> Arc<Texture> {
        #[cfg(feature = "texture_compression")]
        match texture_compression.is_uncompressed() {
            true => self.create_uncompressed_with_mipmaps(name, texture_compression, mips_level, transparent, image),
            false => self.create_compressed_with_mipmaps(name, texture_compression, mips_level, transparent, image),
        }
        #[cfg(not(feature = "texture_compression"))]
        self.create_uncompressed_with_mipmaps(name, texture_compression, mips_level, transparent, image)
    }

    pub(crate) fn create_uncompressed_with_mipmaps(
        &self,
        name: &str,
        _texture_compression: TextureCompression,
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

    #[cfg(feature = "texture_compression")]
    pub(crate) fn create_compressed_with_mipmaps(
        &self,
        name: &str,
        texture_compression: TextureCompression,
        mips_level: u32,
        transparent: bool,
        image: RgbaImage,
    ) -> Arc<Texture> {
        use fast_image_resize::images::Image;
        use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, SrcCropping};
        use image::DynamicImage;
        use intel_tex_2::{bc3, bc7, RgbaSurface};
        use rayon::iter::{IndexedParallelIterator, ParallelIterator};
        use rayon::prelude::ParallelSliceMut;

        const BC_BLOCK_SIZE: usize = 16;
        const BC_STRIP_HEIGHT: usize = 16;

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("pre-process texture for {}", name.magenta()));

        let mut width = image.width();
        let mut height = image.height();

        let base_width = width;
        let base_height = height;

        assert_eq!(width % 4, 0, "Texture width must be aligned to 4 pixels");
        assert_eq!(height % 4, 0, "Texture height must be aligned to 4 pixels");

        let mut total_size = 0;
        let mut mip_width = width as usize;
        let mut mip_height = height as usize;

        for _ in 0..mips_level {
            let mip_size = (mip_width / 4) * (mip_height / 4) * BC_BLOCK_SIZE;

            total_size += mip_size;

            mip_width = (mip_width / 2).max(4);
            mip_height = (mip_height / 2).max(4);
        }

        let mut final_buffer = vec![0u8; total_size];
        let mut current_image = image;
        let mut offset = 0;

        let mut resizer = Resizer::new();
        let resize_options = ResizeOptions {
            algorithm: ResizeAlg::Convolution(FilterType::Lanczos3),
            cropping: SrcCropping::None,
            mul_div_alpha: true,
        };

        let bc7_settings = match texture_compression {
            TextureCompression::Bc7UltraFast => bc7::alpha_ultra_fast_settings(),
            TextureCompression::Bc7VeryFast => bc7::alpha_very_fast_settings(),
            TextureCompression::Bc7Fast => bc7::alpha_fast_settings(),
            TextureCompression::Bc7Slow => bc7::alpha_basic_settings(),
            TextureCompression::Bc7Slowest => bc7::alpha_slow_settings(),
            TextureCompression::Bc3 | TextureCompression::Off => bc7::alpha_slow_settings(),
        };

        for level in 0..mips_level {
            if level > 0 {
                width = (width / 2).max(4);
                height = (height / 2).max(4);

                let mut dst_image = Image::new(width, height, PixelType::U8x4);
                resizer
                    .resize(&DynamicImage::ImageRgba8(current_image), &mut dst_image, &resize_options)
                    .unwrap();

                current_image = RgbaImage::from_raw(width, height, dst_image.into_vec()).unwrap();
            }

            assert_eq!(width % 4, 0, "Mipmap width must be aligned to 4 pixels");
            assert_eq!(height % 4, 0, "Mipmap height must be aligned to 4 pixels");

            let surface = RgbaSurface {
                data: current_image.as_raw(),
                width,
                height,
                stride: width * 4,
            };

            let bytes_per_row = width as usize * 4;
            let blocks_per_row = width as usize / 4;
            let strip_output_size = blocks_per_row * BC_STRIP_HEIGHT / 4 * BC_BLOCK_SIZE;
            let mip_size = (width / 4) as usize * (height / 4) as usize * BC_BLOCK_SIZE;

            final_buffer[offset..offset + mip_size]
                .par_chunks_mut(strip_output_size)
                .enumerate()
                .for_each(|(strip_idx, output_chunk)| {
                    let strip_y = strip_idx * BC_STRIP_HEIGHT;
                    let strip_height = (height as usize - strip_y).min(BC_STRIP_HEIGHT);
                    let strip_height = strip_height - (strip_height % 4);

                    let src_offset = strip_y * bytes_per_row;
                    let strip_surface = RgbaSurface {
                        data: &surface.data[src_offset..],
                        width,
                        height: strip_height as u32,
                        stride: surface.stride,
                    };

                    match texture_compression {
                        TextureCompression::Bc3 => {
                            bc3::compress_blocks_into(&strip_surface, output_chunk);
                        }
                        _ => {
                            bc7::compress_blocks_into(&bc7_settings, &strip_surface, output_chunk);
                        }
                    }
                });

            offset += mip_size;
        }

        let texture = self.create_raw(
            name,
            base_width,
            base_height,
            mips_level,
            texture_compression.into(),
            transparent,
            &final_buffer,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        texture
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
        let mut lock = self.cache.lock().unwrap();
        match lock.get(&(path.into(), image_type)) {
            Some(texture) => Ok(texture.clone()),
            None => {
                // We need to drop to avoid a deadlock here.
                drop(lock);
                self.load(path, image_type)
            }
        }
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

pub struct TextureAtlasFactory {
    name: String,
    texture_loader: Arc<TextureLoader>,
    texture_atlas: OfflineTextureAtlas,
    lookup: HashMap<String, TextureAtlasEntry>,
    create_mip_map: bool,
    transparent: bool,
    texture_compression: TextureCompression,
}

#[derive(Copy, Clone)]
pub struct TextureAtlasEntry {
    pub allocation_id: AllocationId,
    pub transparent: bool,
}

impl TextureAtlasFactory {
    #[cfg(feature = "debug")]
    pub fn create_from_group(
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        paths: &[&str],
        texture_compression: TextureCompression,
    ) -> (Vec<AtlasAllocation>, Arc<Texture>) {
        let mut factory = Self::new(texture_loader, name, add_padding, false, texture_compression);

        let mut ids: Vec<TextureAtlasEntry> = paths.iter().map(|path| factory.register(path)).collect();
        factory.build_atlas();

        let mapping = ids
            .drain(..)
            .map(|entry| factory.get_allocation(entry.allocation_id).unwrap())
            .collect();
        let texture = factory.upload_texture_atlas_texture();

        (mapping, texture)
    }

    pub fn new(
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        create_mip_map: bool,
        texture_compression: TextureCompression,
    ) -> Self {
        let mip_level_count = if create_mip_map { NonZeroU32::new(MIP_LEVELS) } else { None };

        Self {
            name: name.into(),
            texture_loader,
            texture_atlas: OfflineTextureAtlas::new(add_padding, mip_level_count),
            lookup: HashMap::default(),
            create_mip_map,
            transparent: false,
            texture_compression,
        }
    }

    /// Registers the given texture by its path. Will return an allocation ID
    /// which can later be used to get the actual allocation and flag that shows
    /// if a texture contains transparent pixels.
    pub fn register(&mut self, path: &str) -> TextureAtlasEntry {
        if let Some(cached_entry) = self.lookup.get(path).copied() {
            return cached_entry;
        }

        let (data, transparent) = self.texture_loader.load_texture_data(path, false).expect("can't load texture data");
        self.transparent |= transparent;
        let allocation_id = self.texture_atlas.register_image(data);

        let entry = TextureAtlasEntry {
            allocation_id,
            transparent,
        };
        self.lookup.insert(path.to_string(), entry);

        entry
    }

    pub fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        self.texture_atlas.get_allocation(allocation_id)
    }

    pub fn build_atlas(&mut self) {
        self.texture_atlas.build_atlas();
    }

    pub fn upload_texture_atlas_texture(self) -> Arc<Texture> {
        let atlas = self.texture_atlas.get_atlas();
        let name = format!("{} texture atlas", self.name);

        let mips_level = match self.create_mip_map {
            true => MIP_LEVELS,
            false => 1,
        };

        self.texture_loader
            .create_with_mipmaps(&name, self.texture_compression, mips_level, self.transparent, atlas)
    }
}
