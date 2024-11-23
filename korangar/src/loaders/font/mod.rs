use std::sync::Arc;

use cgmath::{Array, Vector2};
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::ElementDisplay;
use korangar_util::FileLoader;
use rusttype::gpu_cache::Cache;
use rusttype::*;
use serde::{Deserialize, Serialize};
use wgpu::{
    Device, Extent3d, ImageCopyTexture, Origin3d, Queue, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use super::GameFileLoader;
use crate::graphics::{Color, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, ScreenSize};

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

pub struct FontLoader {
    queue: Arc<Queue>,
    font_atlas: Texture,
    cache: Box<Cache<'static>>,
    font: Box<Font<'static>>,
}

struct GlyphData {
    glyph: PositionedGlyph<'static>,
    color: Color,
}

fn layout_paragraph(font: &Font<'static>, scale: Scale, width: f32, text: &str, default_color: Color) -> (Vec<GlyphData>, Vector2<f32>) {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    let mut color = default_color;
    let mut chars = text.chars();
    let text_address = text.as_ptr() as usize;

    while let Some(character) = chars.next() {
        if character.is_control() {
            match character {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }

        // Color code following.
        if character == '^' {
            let mut cloned_chars = chars.clone();

            // If the next 6 characters are Hex digits (0-9 A-F)
            if (0..6)
                .map(|_| cloned_chars.next())
                .all(|option| option.is_some_and(|character| character.is_ascii_hexdigit()))
            {
                let start_offset = chars.as_str().as_ptr() as usize - text_address;
                let (color_code, remaining) = text[start_offset..].split_at(6);
                chars = remaining.chars();

                color = match color_code {
                    "000000" => default_color,
                    code => Color::rgb_hex(code),
                };

                continue;
            }
        }

        let base_glyph = font.glyph(character);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }

        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);

        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x as f32 > width {
                caret = point(0.0, caret.y + advance_height);
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }

        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(GlyphData { glyph, color });
    }

    (result, Vector2::new(caret.x, caret.y))
}

impl FontLoader {
    pub fn new(device: &Device, queue: Arc<Queue>, game_file_loader: &GameFileLoader) -> Self {
        let cache_size = Vector2::from_value(2048);
        let cache = Cache::builder().dimensions(cache_size.x, cache_size.y).build();

        let font_atlas = Texture::new(device, &TextureDescriptor {
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
        });

        let font_path = "data\\WenQuanYiMicroHei.ttf";
        let data = game_file_loader.get(font_path).unwrap();
        let font = Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a font from data at {font_path:?}");
        });

        Self {
            queue,
            font_atlas,
            cache: Box::new(cache),
            font: Box::new(font),
        }
    }

    pub fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        let (_, size) = layout_paragraph(
            &self.font,
            Scale::uniform(font_size.get_value()),
            available_width,
            text,
            Color::BLACK,
        );

        ScreenSize {
            width: size.x,
            height: size.y,
        }
    }

    pub fn get(
        &mut self,
        text: &str,
        default_color: Color,
        font_size: FontSize,
        available_width: f32,
    ) -> (Vec<(Rect<f32>, Rect<i32>, Color)>, f32) {
        let (glyphs, size) = layout_paragraph(
            &self.font,
            Scale::uniform(font_size.get_value()),
            available_width,
            text,
            default_color,
        );

        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.glyph.clone());
        }

        self.cache
            .cache_queued(|rect, data| {
                self.queue.write_texture(
                    ImageCopyTexture {
                        texture: self.font_atlas.get_texture(),
                        mip_level: 0,
                        origin: Origin3d {
                            x: rect.min.x,
                            y: rect.min.y,
                            z: 0,
                        },
                        aspect: TextureAspect::All,
                    },
                    data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(rect.width()),
                        rows_per_image: Some(rect.height()),
                    },
                    Extent3d {
                        width: rect.width(),
                        height: rect.height(),
                        depth_or_array_layers: 1,
                    },
                );
            })
            .unwrap();

        (
            glyphs
                .into_iter()
                .filter_map(|glyph| {
                    self.cache
                        .rect_for(0, &glyph.glyph)
                        .unwrap()
                        .map(|tuple| (tuple.0, tuple.1, glyph.color))
                })
                .collect(),
            size.y,
        )
    }

    pub fn get_font_atlas(&self) -> &Texture {
        &self.font_atlas
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl korangar_interface::application::FontLoaderTrait<InterfaceSettings> for std::rc::Rc<std::cell::RefCell<FontLoader>> {
    fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        self.borrow().get_text_dimensions(text, font_size, available_width)
    }
}
