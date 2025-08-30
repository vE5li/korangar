mod color_span_iterator;
mod font_file;
mod font_map_descriptor;
mod layout_key;

use std::hash::Hash;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use cgmath::{Point2, Vector2};
use cosmic_text::fontdb::ID;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, fontdb};
use hashbrown::HashMap;
use image::{ImageBuffer, Rgba, RgbaImage, imageops};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
use korangar_interface::application::TextLayouter;
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::ElementDisplay;
use korangar_util::Rectangle;
use korangar_util::container::{Cacheable, SimpleCache};
use serde::{Deserialize, Serialize};

use self::color_span_iterator::ColorSpanIterator;
use super::{GameFileLoader, TextureLoader};
use crate::graphics::{Color, MAX_TEXTURE_SIZE, ScreenSize, Texture};
use crate::loaders::font::font_file::FontFile;
use crate::loaders::font::layout_key::{LayoutKey, LayoutKeyRef};
use crate::state::ClientState;

const MAX_CACHE_COUNT: u32 = 512;
// We cache layouts only by count.
const MAX_CACHE_SIZE: usize = usize::MAX;

struct CachedLayout {
    glyphs: Vec<GlyphInstruction>,
    size: Vector2<f32>,
}

impl Cacheable for CachedLayout {
    fn size(&self) -> usize {
        // We cache layouts only by count.
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontSize(pub f32);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowBehavior {
    Shrink,
    LineBreak,
}

impl korangar_interface::application::FontSize for FontSize {
    fn scaled(&self, scaling: f32) -> Self {
        Self(self.0 * scaling)
    }
}

impl ElementDisplay for FontSize {
    fn element_display(&self) -> String {
        format!("^000001F^000000{}", self.0.element_display())
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

impl Scaling {
    // TODO: Likely remove this function
    pub fn get_factor(&self) -> f32 {
        self.0
    }
}

impl DropDownItem<Scaling> for Scaling {
    fn text(&self) -> &str {
        match self.0 {
            0.5 => "50%",
            0.6 => "60%",
            0.7 => "70%",
            0.8 => "80%",
            0.9 => "90%",
            1.0 => "100%",
            1.1 => "110%",
            1.2 => "120%",
            1.3 => "130%",
            1.4 => "140%",
            1.5 => "150%",
            1.6 => "160%",
            1.7 => "170%",
            1.8 => "180%",
            1.9 => "190%",
            2.0 => "200%",
            _ => unimplemented!(),
        }
    }

    fn value(&self) -> Scaling {
        *self
    }
}

impl ElementDisplay for Scaling {
    fn element_display(&self) -> String {
        format!("^000001a^000000{}", self.0.element_display())
    }
}

impl Scaling {
    pub const fn new(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone)]
pub struct GlyphInstruction {
    pub position: Rectangle<f32>,
    pub texture_coordinate: Rectangle<f32>,
    pub color: Color,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct GlyphCoordinate {
    pub(crate) texture_coordinate: Rectangle<f32>,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) offset_top: f32,
    pub(crate) offset_left: f32,
}

pub struct FontLoader {
    font_system: Mutex<FontSystem>,
    primary_font_family: String,
    font_map: Arc<Texture>,
    glyph_cache: HashMap<ID, Arc<HashMap<u16, GlyphCoordinate>>>,
    layout_cache: Mutex<SimpleCache<LayoutKey, CachedLayout>>,
}

impl FontLoader {
    pub fn new(fonts: &[String], game_file_loader: &GameFileLoader, texture_loader: &TextureLoader) -> Self {
        assert_ne!(fonts.len(), 0, "no font defined");

        let mut font_system = FontSystem::new_with_locale_and_db(Self::system_locale(), fontdb::Database::new());
        let mut glyph_cache = HashMap::new();

        let fonts: Vec<FontFile> = fonts
            .iter()
            .filter_map(|font_name| FontFile::new(font_name, game_file_loader, &mut font_system))
            .collect();

        let primary_font_family = Self::extract_primary_font_family(&font_system, &fonts);
        let font_map_image_data = Self::merge_font_maps(&mut glyph_cache, &mut font_system, fonts);

        let font_map = texture_loader.create_msdf("font map", font_map_image_data);

        let layout_cache = SimpleCache::new(
            NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
            NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
        );

        Self {
            font_system: Mutex::new(font_system),
            primary_font_family,
            font_map,
            glyph_cache,
            layout_cache: Mutex::new(layout_cache),
        }
    }

    fn system_locale() -> String {
        sys_locale::get_locale().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to get system locale, falling back to en-US", "warning".yellow());
            "en-US".to_string()
        })
    }

    fn extract_primary_font_family(font_system: &FontSystem, fonts: &[FontFile]) -> String {
        let primary_font_id = fonts
            .first()
            .and_then(|font| font.ids.first())
            .copied()
            .expect("no primary font ID found");

        font_system
            .db()
            .face(primary_font_id)
            .and_then(|face| face.families.first().map(|(family, _)| family.clone()))
            .expect("primary font has no family name")
    }

    fn merge_font_maps(
        glyph_cache: &mut HashMap<ID, Arc<HashMap<u16, GlyphCoordinate>>>,
        font_system: &mut FontSystem,
        mut fonts: Vec<FontFile>,
    ) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        if fonts.len() == 1 {
            let FontFile { ids, font_map, glyphs } = fonts.drain(..).take(1).next().unwrap();

            for &id in &ids {
                glyph_cache.insert(id, glyphs.clone());
                let _ = font_system.get_font(id);
            }

            font_map
        } else {
            let overall_height: u32 = fonts.iter().map(|font| font.font_map.height()).sum();

            assert!(
                overall_height <= MAX_TEXTURE_SIZE,
                "aggregated font map is higher than max texture size"
            );
            assert_ne!(overall_height, 0, "aggregated font map height is zero");

            let mut font_map_image_data = RgbaImage::new(MAX_TEXTURE_SIZE, overall_height);
            let mut start_height = 0;

            for font in fonts {
                let FontFile { ids, font_map, glyphs } = font;

                let font_map_height = font_map.height() as f32;

                let adjusted_glyphs: Arc<HashMap<u16, GlyphCoordinate>> = Arc::new(
                    glyphs
                        .iter()
                        .map(|(&index, &coordinate)| {
                            let mut new_coordinate = coordinate;

                            let y_offset = start_height as f32 / overall_height as f32;
                            let scale_factor = font_map_height / overall_height as f32;

                            new_coordinate.texture_coordinate = Rectangle::new(
                                Point2::new(
                                    coordinate.texture_coordinate.min.x,
                                    coordinate.texture_coordinate.min.y * scale_factor + y_offset,
                                ),
                                Point2::new(
                                    coordinate.texture_coordinate.max.x,
                                    coordinate.texture_coordinate.max.y * scale_factor + y_offset,
                                ),
                            );

                            (index, new_coordinate)
                        })
                        .collect(),
                );

                for &id in &ids {
                    glyph_cache.insert(id, adjusted_glyphs.clone());
                    let _ = font_system.get_font(id);
                }

                imageops::replace(&mut font_map_image_data, &font_map, 0, start_height);
                start_height += font_map_height as i64;
            }

            font_map_image_data
        }
    }

    pub fn get_text_dimensions(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
        line_height_scale: f32,
        available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        // Avoid floating point inaccuracy leading to a line break when shrinking.
        const THRESHOLD: f32 = 1.0;

        let layout_width = match overflow_behavior {
            OverflowBehavior::Shrink => None,
            OverflowBehavior::LineBreak => Some(available_width),
        };

        let mut size = self.layout_text(
            text,
            default_color,
            highlight_color,
            font_size,
            line_height_scale,
            layout_width,
            None,
        );

        if let OverflowBehavior::Shrink = overflow_behavior {
            let scaling_factor = (available_width / (size.x + THRESHOLD)).min(1.0);

            font_size = FontSize(font_size.0 * scaling_factor);
            size *= scaling_factor;
        }

        (
            ScreenSize {
                width: size.x,
                height: size.y,
            },
            font_size,
        )
    }

    /// Writes the text layout for the given text into the `glyphs` buffer and
    /// returns the size of the text in pixels.
    ///
    /// Does not clear the glyph buffer before writing into it.
    pub fn layout_text(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: Option<f32>,
        glyphs: Option<&mut Vec<GlyphInstruction>>,
    ) -> Vector2<f32> {
        // TODO: NHA We currently call get_text_dimensions() with different
        //       "available_width" and "overflow_behavior" then when we call
        //       layout_text() for with the resulting available_width.
        //       This results us caching two instead of one layout.
        let key = LayoutKeyRef {
            text,
            default_color: default_color.into(),
            highlight_color: highlight_color.into(),
            font_size,
            line_height_scale,
            layout_width: available_width.unwrap_or(f32::INFINITY),
        };

        if let Some(layout) = self.layout_cache.lock().unwrap().get_with(&key, |k| k == &key) {
            if let Some(glyphs) = glyphs {
                glyphs.extend(layout.glyphs.iter().copied())
            }
            return layout.size;
        }

        let (size, rendered_glyphs) = self.render_layout(
            text,
            default_color,
            highlight_color,
            font_size,
            line_height_scale,
            available_width,
        );

        if let Some(glyphs) = glyphs {
            glyphs.extend(rendered_glyphs.iter().copied());
        }

        let _result = self.layout_cache.lock().unwrap().insert(key.to_owned(), CachedLayout {
            glyphs: rendered_glyphs,
            size,
        });

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] layout could not be added to cache. Text: '{}': {:?}",
                "error".red(),
                text,
                error
            );
        }

        size
    }

    // TODO: NHA cosmic_text could help us to render text in boxes.
    //       But that would need us to re-evaluate on how we render test in general.
    //       We also don't really use the "line_height_scale", which would provide
    //       an easy way to handle "line height".
    fn render_layout(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: Option<f32>,
    ) -> (Vector2<f32>, Vec<GlyphInstruction>) {
        let mut text_width = 0f32;
        let mut text_height = 0f32;

        let metrics = Metrics::relative(font_size.0, line_height_scale);
        let attributes = Attrs::new().family(Family::Name(&self.primary_font_family));

        // We try to hold the mutex lock as short as possible.
        let buffer = {
            let mut font_system = self.font_system.lock().unwrap();
            let mut buffer = Buffer::new(&mut font_system, metrics);

            buffer.set_size(&mut font_system, available_width, None);
            buffer.set_rich_text(
                &mut font_system,
                ColorSpanIterator::new(text, default_color, highlight_color, attributes.clone()),
                &attributes,
                Shaping::Advanced,
                None,
            );

            buffer
        };

        let mut rendered_glyphs = Vec::new();

        for run in buffer.layout_runs() {
            text_width = text_width.max(run.line_w);
            text_height += run.line_height;

            for layout_glyph in run.glyphs.iter() {
                let physical_glyph = layout_glyph.physical((0.0, 0.0), 1.0);

                let Some(glyph_coordinate) = self.glyph_cache.get(&layout_glyph.font_id).and_then(|font| {
                    font.get(&layout_glyph.glyph_id).copied().map(|mut glyph| {
                        glyph.width *= font_size.0;
                        glyph.height *= font_size.0;
                        glyph.offset_left *= font_size.0;
                        glyph.offset_top *= font_size.0;
                        glyph
                    })
                }) else {
                    continue;
                };

                let x = physical_glyph.x as f32 + glyph_coordinate.offset_left;
                let y = run.line_y + physical_glyph.y as f32 + glyph_coordinate.offset_top;
                let width = glyph_coordinate.width;
                let height = glyph_coordinate.height;

                let position = Rectangle::new(Point2::new(x, y), Point2::new(x + width, y + height));
                let color = layout_glyph.color_opt.map(|color| color.into()).unwrap_or(default_color);

                rendered_glyphs.push(GlyphInstruction {
                    position,
                    texture_coordinate: glyph_coordinate.texture_coordinate,
                    color,
                });
            }
        }

        (Vector2::new(text_width, text_height), rendered_glyphs)
    }

    /// The texture of the static font map.
    pub fn get_font_map(&self) -> &Texture {
        &self.font_map
    }
}

impl TextLayouter<ClientState> for Arc<FontLoader> {
    fn get_text_dimensions(
        &self,
        text: &str,
        default_color: Color,
        highlight_color: Color,
        font_size: FontSize,
        available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        let (size, font_size) = self.as_ref().get_text_dimensions(
            text,
            default_color,
            highlight_color,
            font_size,
            1.0,
            available_width,
            overflow_behavior,
        );

        (size, font_size)
    }
}
