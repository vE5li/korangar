mod color_span_iterator;
mod font_map_descriptor;

use std::io::{Cursor, Read};
use std::sync::Arc;

use cgmath::{Point2, Vector2};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use flate2::bufread::GzDecoder;
use hashbrown::HashMap;
use image::{ImageFormat, ImageReader};
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::ElementDisplay;
use korangar_util::{FileLoader, Rectangle};
use serde::{Deserialize, Serialize};

use self::color_span_iterator::ColorSpanIterator;
use self::font_map_descriptor::parse_glyph_cache;
use super::{GameFileLoader, TextureLoader};
use crate::graphics::{Color, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, ScreenSize};

const FONT_FILE_PATH: &str = "data\\font\\NotoSans.ttf";
const FONT_MAP_DESCRIPTION_FILE_PATH: &str = "data\\font\\NotoSans.csv.gz";
const FONT_MAP_FILE_PATH: &str = "data\\font\\NotoSans.png";
const FONT_FAMILY_NAME: &str = "Noto Sans";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontSize(pub f32);

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
    pub const fn new(value: f32) -> Self {
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
    pub size: Vector2<f32>,
}

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
    font_system: FontSystem,
    font_map: Arc<Texture>,
    glyph_cache: HashMap<u16, GlyphCoordinate>,
}

impl FontLoader {
    pub fn new(game_file_loader: &GameFileLoader, texture_loader: &TextureLoader) -> Self {
        let font_data = game_file_loader.get(FONT_FILE_PATH).unwrap();
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(font_data);

        let font_map_data = game_file_loader.get(FONT_MAP_FILE_PATH).unwrap();
        let font_map_reader = ImageReader::with_format(Cursor::new(font_map_data), ImageFormat::Png);
        let font_map_decoder = font_map_reader.decode().unwrap();
        let font_map_rgba_image = font_map_decoder.into_rgba8();
        let font_map = texture_loader.create_msdf("font map", font_map_rgba_image);
        let font_map_size = font_map.get_size();

        let mut font_description_data = game_file_loader.get(FONT_MAP_DESCRIPTION_FILE_PATH).unwrap();

        if FONT_MAP_DESCRIPTION_FILE_PATH.ends_with(".gz") {
            let mut decoder = GzDecoder::new(&font_description_data[..]);
            let mut data = Vec::with_capacity(font_description_data.len() * 2);
            decoder.read_to_end(&mut data).unwrap();
            font_description_data = data;
        }

        let font_description_content = String::from_utf8(font_description_data).unwrap();
        let glyph_cache = parse_glyph_cache(font_description_content, font_map_size.width, font_map_size.height);

        Self {
            font_system,
            font_map,
            glyph_cache,
        }
    }

    pub fn get_text_dimensions(&mut self, text: &str, font_size: FontSize, line_height_scale: f32, available_width: f32) -> ScreenSize {
        let TextLayout { size, .. } = self.get_text_layout(text, Color::BLACK, font_size, line_height_scale, available_width);

        ScreenSize {
            width: size.x,
            height: size.y,
        }
    }

    // TODO: NHA cosmic_text could help us to render text in boxes.
    //       But that would need us to re-evaluate on how we render test in general.
    //       We also don't really use the "line_height_scale", which would provide
    //       an easy way to handle "line height".
    pub fn get_text_layout(
        &mut self,
        text: &str,
        default_color: Color,
        font_size: FontSize,
        line_height_scale: f32,
        available_width: f32,
    ) -> TextLayout {
        let mut glyphs = Vec::with_capacity(text.len());
        let mut text_width = 0f32;
        let mut text_height = 0f32;

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
            text_width = text_width.max(run.line_w);
            text_height += run.line_height;

            for layout_glyph in run.glyphs.iter() {
                let physical_glyph = layout_glyph.physical((0.0, 0.0), 1.0);

                let Some(glyph_coordinate) = self.glyph_cache.get(&layout_glyph.glyph_id).copied().map(|mut glyph| {
                    glyph.width *= font_size.0;
                    glyph.height *= font_size.0;
                    glyph.offset_left *= font_size.0;
                    glyph.offset_top *= font_size.0;
                    glyph
                }) else {
                    continue;
                };

                let x = physical_glyph.x as f32 + glyph_coordinate.offset_left;
                let y = run.line_y + physical_glyph.y as f32 + glyph_coordinate.offset_top;
                let width = glyph_coordinate.width;
                let height = glyph_coordinate.height;

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

    /// The texture of the static font map.
    pub fn get_font_map(&self) -> &Texture {
        &self.font_map
    }
}

impl korangar_interface::application::FontLoaderTrait<InterfaceSettings> for std::rc::Rc<std::cell::RefCell<FontLoader>> {
    fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        self.borrow_mut().get_text_dimensions(text, font_size, 1.0, available_width)
    }
}
