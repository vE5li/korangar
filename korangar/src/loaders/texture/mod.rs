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

    fn create(&self, name: &str, image: RgbaImage, transparent: bool) -> Arc<Texture> {
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
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            transparent,
        );
        Arc::new(texture)
    }

    pub fn create_sdf(&self, name: &str, image: GrayImage) -> Arc<Texture> {
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
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            false,
        );
        Arc::new(texture)
    }

    pub fn create_msdf(&self, name: &str, image: RgbaImage) -> Arc<Texture> {
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
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image.as_raw(),
            false,
        );
        Arc::new(texture)
    }

    fn create_with_mip_maps(&self, name: &str, image: RgbaImage, transparent: bool) -> Arc<Texture> {
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

        let mut mip_views = Vec::with_capacity(MIP_LEVELS as usize);

        for level in 0..MIP_LEVELS {
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

        for index in 0..(MIP_LEVELS - 1) as usize {
            let mut pass = self
                .mip_map_render_context
                .create_pass(&self.device, &mut encoder, &mip_views[index], &mip_views[index + 1]);
            self.lanczos3_drawer.draw(&mut pass);
        }

        self.queue.submit(Some(encoder.finish()));

        Arc::new(texture)
    }

    fn load(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let texture = match image_type {
            ImageType::Color => {
                let (texture_data, transparent) = self.load_texture_data(path, false)?;
                self.create(path, texture_data, transparent)
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

    pub fn get(&self, path: &str, image_type: ImageType) -> Result<Arc<Texture>, LoadError> {
        let mut lock = self.cache.lock();
        match lock.as_mut().unwrap().get(&(path.into(), image_type)) {
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
    ) -> (Vec<AtlasAllocation>, Arc<Texture>) {
        let mut factory = Self::new(texture_loader, name, add_padding, false);

        let mut ids: Vec<TextureAtlasEntry> = paths.iter().map(|path| factory.register(path)).collect();
        factory.build_atlas();

        let mapping = ids
            .drain(..)
            .map(|entry| factory.get_allocation(entry.allocation_id).unwrap())
            .collect();
        let texture = factory.upload_texture_atlas_texture();

        (mapping, texture)
    }

    pub fn new(texture_loader: Arc<TextureLoader>, name: impl Into<String>, add_padding: bool, create_mip_map: bool) -> Self {
        let mip_level_count = if create_mip_map { NonZeroU32::new(MIP_LEVELS) } else { None };

        Self {
            name: name.into(),
            texture_loader,
            texture_atlas: OfflineTextureAtlas::new(add_padding, mip_level_count),
            lookup: HashMap::default(),
            create_mip_map,
            transparent: false,
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
        if self.create_mip_map {
            self.texture_loader.create_with_mip_maps(
                &format!("{} texture atlas", self.name),
                self.texture_atlas.get_atlas(),
                self.transparent,
            )
        } else {
            self.texture_loader.create(
                &format!("{} texture atlas", self.name),
                self.texture_atlas.get_atlas(),
                self.transparent,
            )
        }
    }
}
