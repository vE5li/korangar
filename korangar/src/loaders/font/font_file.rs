use std::sync::Arc;

use cgmath::Point2;
use cosmic_text::FontSystem;
use cosmic_text::fontdb::{ID, Source};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_loaders::FileLoader;
use ttf_parser::Face;

use crate::loaders::GameFileLoader;
use crate::loaders::font::GlyphCoordinate;
use crate::loaders::font::slug::{SlugFontData, SlugGlyphBounds};
use crate::loaders::rectangle::Rectangle;

const FONT_FOLDER_PATH: &str = "data\\font";

pub(crate) type FaceGlyphs = Arc<Vec<Option<GlyphCoordinate>>>;

pub(crate) struct FontFile {
    pub(crate) faces: Vec<(ID, FaceGlyphs)>,
}

impl FontFile {
    pub(crate) fn new(
        name: &str,
        game_file_loader: &GameFileLoader,
        font_system: &mut FontSystem,
        slug_data: &mut SlugFontData,
    ) -> Option<Self> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load font: {}", name.magenta()));

        let ttf_file_path = format!("{}\\{}.ttf", FONT_FOLDER_PATH, name);

        let Ok(font_data) = game_file_loader.get(&ttf_file_path) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to load font file '{}'", "error".red(), ttf_file_path.magenta());
            return None;
        };

        let ids = font_system.db_mut().load_font_source(Source::Binary(Arc::new(font_data)));

        let faces: Vec<(ID, FaceGlyphs)> = ids
            .iter()
            .filter_map(|&id| {
                let glyphs = font_system.db().with_face_data(id, |data, index| {
                    let face = Face::parse(data, index).ok()?;
                    Some(slug_data.add_face(&face))
                })??;

                let glyphs = glyphs.into_iter().map(|bounds| bounds.map(glyph_coordinate)).collect();

                Some((id, Arc::new(glyphs)))
            })
            .collect();

        if faces.is_empty() {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to parse font file '{}'", "error".red(), ttf_file_path.magenta());
            return None;
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Some(Self { faces })
    }
}

/// Converts the em space bounds of a glyph with the y-axis pointing up into
/// the placement of the glyph relative to its origin with the y-axis pointing
/// down.
fn glyph_coordinate(bounds: SlugGlyphBounds) -> GlyphCoordinate {
    GlyphCoordinate {
        em_coordinate: Rectangle::new(Point2::new(bounds.x_min, bounds.y_max), Point2::new(bounds.x_max, bounds.y_min)),
        glyph_index: bounds.glyph_index,
        width: bounds.x_max - bounds.x_min,
        height: bounds.y_max - bounds.y_min,
        offset_top: -bounds.y_max,
        offset_left: bounds.x_min,
    }
}
