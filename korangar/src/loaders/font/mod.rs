mod color_span_iterator;

use std::sync::Arc;

use cgmath::{Array, Point2, Vector2};
use cosmic_text::{Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, PhysicalGlyph, Shaping, SwashCache, SwashContent};
use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::ElementDisplay;
use korangar_util::texture_atlas::OnlineTextureAtlas;
use korangar_util::{FileLoader, Rectangle};
use serde::{Deserialize, Serialize};
use wgpu::{
    Device, Extent3d, ImageCopyTexture, Origin3d, Queue, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use self::color_span_iterator::ColorSpanIterator;
use super::GameFileLoader;
use crate::graphics::{Color, Texture, MAX_TEXTURE_SIZE};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, ScreenSize};

const FONT_FILE_PATH: &str = "data\\WenQuanYiMicroHei.ttf";
const FONT_FAMILY_NAME: &str = "WenQuanYi Micro Hei";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontSize(f32);

impl ArrayType for FontSize {
    type Element = f32;

    const ELEMENT_COUNT: usize = 1;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT] {
        [("size".to_owned(), &self.0)]
    }

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT] {
        [self.0]
    }
}

impl ElementDisplay for FontSize {
    fn display(&self) -> String {
        format!("^FFBB00F^000000{}", self.0.display())
    }
}

impl FontSizeTrait for FontSize {
    fn new(value: f32) -> Self {
        Self(value)
    }

    fn get_value(&self) -> f32 {
        self.0
    }
}

impl std::ops::Mul<f32> for FontSize {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self(self.0 * rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scaling(f32);

impl ArrayType for Scaling {
    type Element = f32;

    const ELEMENT_COUNT: usize = 1;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT] {
        [("scale".to_owned(), &self.0)]
    }

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT] {
        [self.0]
    }
}

impl ElementDisplay for Scaling {
    fn display(&self) -> String {
        format!("^FFBB00a^000000{}", self.0.display())
    }
}

impl Scaling {
    pub fn new(value: f32) -> Self {
        Self(value)
    }
}

impl korangar_interface::application::ScalingTrait for Scaling {
    fn get_factor(&self) -> f32 {
        self.0
    }
}

pub struct TextLayout {
    pub glyphs: Vec<GlyphInstruction>,
    pub size: Vector2<i32>,
}

pub struct GlyphInstruction {
    pub position: Rectangle<i32>,
    pub texture_coordinate: Rectangle<f32>,
    pub color: Color,
}

#[derive(Copy, Clone)]
struct GlyphCoordinate {
    texture_coordinate: Rectangle<f32>,
    width: i32,
    height: i32,
    offset_top: i32,
    offset_left: i32,
}

pub struct FontLoader {
    queue: Arc<Queue>,
    font_atlas: Texture,
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyph_cache: HashMap<CacheKey, GlyphCoordinate>,
    texture_atlas: OnlineTextureAtlas,
    atlas_is_full: bool,
}

impl FontLoader {
    pub fn new(device: &Device, queue: Arc<Queue>, game_file_loader: &GameFileLoader) -> Self {
        let cache_size = Vector2::from_value(2048);

        let online_texture_atlas = OnlineTextureAtlas::new(cache_size.x, cache_size.y, true);
        let font_atlas = Self::create_texture_atlas_texture(device, cache_size);
        let font_data = game_file_loader.get(FONT_FILE_PATH).unwrap();

        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(font_data);

        Self {
            queue,
            font_atlas,
            font_system,
            swash_cache: SwashCache::new(),
            glyph_cache: HashMap::new(),
            texture_atlas: online_texture_atlas,
            atlas_is_full: false,
        }
    }

    fn create_texture_atlas_texture(device: &Device, cache_size: Vector2<u32>) -> Texture {
        Texture::new(
            device,
            &TextureDescriptor {
                label: Some("Texture Atlas"),
                size: Extent3d {
                    width: cache_size.x,
                    height: cache_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            true,
        )
    }

    pub fn is_full(&self) -> bool {
        self.atlas_is_full
    }

    pub fn resize_or_clear(&mut self, device: &Device) {
        self.atlas_is_full = false;

        let current_size = self.font_atlas.get_size();
        let mut new_width = current_size.width;
        let mut new_height = current_size.height;

        if new_width <= new_height && new_width * 2 <= MAX_TEXTURE_SIZE {
            new_width *= 2;
        } else if new_height < new_width && new_height * 2 <= MAX_TEXTURE_SIZE {
            new_height *= 2;
        }

        if new_width != current_size.width || new_height != current_size.height {
            self.texture_atlas.clear();
            self.glyph_cache.clear();

            self.texture_atlas = OnlineTextureAtlas::new(new_width, new_height, true);
            self.font_atlas = Self::create_texture_atlas_texture(device, Vector2::new(new_width, new_height));

            #[cfg(feature = "debug")]
            print_debug!("increased font atlas size to {}x{}", new_width, new_height);
        } else {
            self.clear(device);
        }
    }

    // TODO: NHA Call this when we change the scale factor of the application.
    pub fn clear(&mut self, device: &Device) {
        self.texture_atlas.clear();
        self.glyph_cache.clear();
        let size = self.font_atlas.get_size();
        self.font_atlas = Self::create_texture_atlas_texture(device, Vector2::new(size.width, size.height));
    }

    pub fn get_text_dimensions(&mut self, text: &str, font_size: FontSize, line_height_scale: f32, available_width: f32) -> ScreenSize {
        let TextLayout { size, .. } = self.get(text, Color::BLACK, font_size, line_height_scale, available_width);

        ScreenSize {
            width: size.x as f32,
            height: size.y as f32,
        }
    }

    // TODO: NHA cosmic_text could help us to render text in boxes.
    //       But that would need us to re-evaluate on how we render test in general.
    //       We also don't really use the "line_height_scale", which would provide
    //       an easy way to handle "line height".
    pub fn get(
        &mut self,
        text: &str,
        default_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: f32,
    ) -> TextLayout {
        let mut glyphs = Vec::with_capacity(text.len());
        let mut text_width = 0;
        let mut text_height = 0;

        let metrics = Metrics::relative(font_size.0, line_height_scale);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attributes = Attrs::new().family(Family::Name(FONT_FAMILY_NAME));

        buffer.set_size(&mut self.font_system, Some(available_width), None);

        buffer.set_rich_text(
            &mut self.font_system,
            ColorSpanIterator::new(text, default_color, attributes),
            attributes,
            Shaping::Advanced,
        );

        for run in buffer.layout_runs() {
            text_width = text_height.max(run.line_w.round() as i32);

            for layout_glyph in run.glyphs.iter() {
                let physical_glyph = layout_glyph.physical((0.0, 0.0), 1.0);

                if !self.glyph_cache.contains_key(&physical_glyph.cache_key) && self.add_glyph_to_cache(&physical_glyph).is_err() {
                    continue;
                }

                let glyph_coordinate = *self.glyph_cache.get(&physical_glyph.cache_key).unwrap();

                let x = physical_glyph.x + glyph_coordinate.offset_left;
                let y = run.line_y.round() as i32 + physical_glyph.y - glyph_coordinate.offset_top;
                let width = glyph_coordinate.width;
                let height = glyph_coordinate.height;

                text_height = text_height.max(y + height);

                let position = Rectangle::new(Point2::new(x, y), Point2::new(x + width, y + height));
                let color = layout_glyph.color_opt.map(|color| color.into()).unwrap_or(default_color);

                glyphs.push(GlyphInstruction {
                    position,
                    texture_coordinate: glyph_coordinate.texture_coordinate,
                    color,
                });
            }
        }

        TextLayout {
            glyphs,
            size: Vector2::new(text_width, text_height),
        }
    }

    fn add_glyph_to_cache(&mut self, physical_glyph: &PhysicalGlyph) -> Result<(), ()> {
        let image = self
            .swash_cache
            .get_image_uncached(&mut self.font_system, physical_glyph.cache_key)
            .expect("can't create glyph image");

        let width = image.placement.width;
        let height = image.placement.height;

        // We only support rendering of glyphs that use an alpha mask.
        // The other types are used for sub-pixel rendering, which cosmic-text
        // currently doesn't expose and is also hard to do properly (since you need to
        // know the sub-pixel layout of the monitor used).
        if image.content != SwashContent::Mask || width == 0 || height == 0 {
            return Err(());
        }

        let Some(allocation) = self.texture_atlas.allocate(Vector2::new(width, height)) else {
            if !self.atlas_is_full {
                self.atlas_is_full = true;

                #[cfg(feature = "debug")]
                print_debug!("[{}] texture atlas is full", "warning".yellow());
            }

            return Err(());
        };

        let glyph_coordinate = GlyphCoordinate {
            texture_coordinate: Rectangle::new(
                allocation.map_to_atlas(Point2::from_value(0.0)),
                allocation.map_to_atlas(Point2::from_value(1.0)),
            ),
            width: width as i32,
            height: height as i32,
            offset_top: image.placement.top,
            offset_left: image.placement.left,
        };

        self.glyph_cache.insert(physical_glyph.cache_key, glyph_coordinate);

        self.queue.write_texture(
            ImageCopyTexture {
                texture: self.font_atlas.get_texture(),
                mip_level: 0,
                origin: Origin3d {
                    x: allocation.rectangle.min.x,
                    y: allocation.rectangle.min.y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            &image.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    pub fn get_font_atlas(&self) -> &Texture {
        &self.font_atlas
    }
}

impl korangar_interface::application::FontLoaderTrait<InterfaceSettings> for std::rc::Rc<std::cell::RefCell<FontLoader>> {
    fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        self.borrow_mut().get_text_dimensions(text, font_size, 1.0, available_width)
    }
}
