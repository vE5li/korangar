use std::io::Cursor;
use std::sync::{Arc, Mutex};

use derive_new::new;
use hashbrown::HashMap;
use image::{EncodableLayout, ImageFormat, ImageReader, Rgba, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation, TextureAtlas};
use korangar_util::FileLoader;
use wgpu::{Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::error::LoadError;
use super::{FALLBACK_BMP_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE};
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    #[new(default)]
    cache: Mutex<HashMap<String, Arc<Texture>>>,
}

impl TextureLoader {
    fn create(&self, name: &str, image: RgbaImage) -> Arc<Texture> {
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
            image.as_bytes(),
        );
        Arc::new(texture)
    }

    fn load(&self, path: &str) -> Result<Arc<Texture>, LoadError> {
        let texture_data = self.load_texture_data(path)?;
        let texture = self.create(path, texture_data);
        self.cache.lock().as_mut().unwrap().insert(path.to_string(), texture.clone());
        Ok(texture)
    }

    pub fn load_texture_data(&self, path: &str) -> Result<RgbaImage, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture data from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            _ => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("File with unknown image format found: {:?}", path);
                    print_debug!("Replacing with fallback");
                }

                return self.load_texture_data(FALLBACK_PNG_FILE);
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

                return self.load_texture_data(FALLBACK_PNG_FILE);
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
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Bmp => FALLBACK_BMP_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.load_texture_data(fallback_path);
            }
        };

        if image_format == ImageFormat::Bmp {
            // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
            image_buffer
                .pixels_mut()
                .filter(|pixel| pixel.0[0] > 0xF0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0F)
                .for_each(|pixel| *pixel = Rgba([0; 4]));
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(image_buffer)
    }

    pub fn get(&self, path: &str) -> Result<Arc<Texture>, LoadError> {
        let lock = self.cache.lock();
        match lock.as_ref().unwrap().get(path) {
            Some(texture) => Ok(texture.clone()),
            None => {
                // We need to drop to avoid a deadlock here.
                drop(lock);
                self.load(path)
            }
        }
    }
}

pub struct TextureAtlasFactory {
    name: String,
    texture_loader: Arc<TextureLoader>,
    texture_atlas: TextureAtlas,
    lookup: HashMap<String, AllocationId>,
}

impl TextureAtlasFactory {
    #[cfg(feature = "debug")]
    pub fn create_from_group(
        texture_loader: Arc<TextureLoader>,
        name: impl Into<String>,
        add_padding: bool,
        paths: &[&str],
    ) -> (Vec<AtlasAllocation>, Arc<Texture>) {
        let mut factory = Self::new(texture_loader, name, add_padding);

        let mut ids: Vec<AllocationId> = paths.iter().map(|path| factory.register(path)).collect();
        factory.build_atlas();

        let mapping = ids.drain(..).map(|id| factory.get_allocation(id).unwrap()).collect();
        let texture = factory.upload_texture_atlas_texture();

        (mapping, texture)
    }

    pub fn new(texture_loader: Arc<TextureLoader>, name: impl Into<String>, add_padding: bool) -> Self {
        Self {
            name: name.into(),
            texture_loader,
            texture_atlas: TextureAtlas::new(add_padding),
            lookup: HashMap::default(),
        }
    }

    /// Registers the given texture by its path. Will return an allocation ID
    /// which can later be used to get the actual allocation.
    pub fn register(&mut self, path: &str) -> AllocationId {
        if let Some(allocation_id) = self.lookup.get(path).copied() {
            return allocation_id;
        }

        let data = self.texture_loader.load_texture_data(path).expect("can't load texture data");
        let allocation_id = self.texture_atlas.register_image(data);
        self.lookup.insert(path.to_string(), allocation_id);

        allocation_id
    }

    pub fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        self.texture_atlas.get_allocation(allocation_id)
    }

    pub fn build_atlas(&mut self) {
        self.texture_atlas.build_atlas();
    }

    pub fn upload_texture_atlas_texture(self) -> Arc<Texture> {
        self.texture_loader
            .create(&format!("{} texture atlas", self.name), self.texture_atlas.get_atlas())
    }
}
