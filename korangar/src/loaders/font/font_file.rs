use std::io::Cursor;
use std::sync::Arc;

use cosmic_text::FontSystem;
use cosmic_text::fontdb::{ID, Source};
use hashbrown::HashMap;
use image::{ImageFormat, ImageReader, RgbaImage};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;

use crate::loaders::GameFileLoader;
use crate::loaders::font::GlyphCoordinate;
use crate::loaders::font::font_map_descriptor::parse_glyphs;

const FONT_FOLDER_PATH: &str = "data\\font";

pub(crate) struct FontFile {
    pub(crate) ids: Vec<ID>,
    pub(crate) font_map: RgbaImage,
    pub(crate) glyphs: Arc<HashMap<u16, GlyphCoordinate>>,
}

impl FontFile {
    pub(crate) fn new(name: &str, game_file_loader: &GameFileLoader, font_system: &mut FontSystem) -> Option<Self> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load font: {}", name.magenta()));

        let font_base_path = format!("{}\\{}", FONT_FOLDER_PATH, name);
        let ttf_file_path = format!("{}.ttf", font_base_path);
        let map_file_path = format!("{}.png", font_base_path);
        let map_description_file_path = format!("{}.csv", font_base_path);

        let Ok(font_data) = game_file_loader.get(&ttf_file_path) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to load font file '{}'", "error".red(), ttf_file_path.magenta());
            return None;
        };

        let ids = font_system.db_mut().load_font_source(Source::Binary(Arc::new(font_data)));

        let Ok(font_map_data) = game_file_loader.get(&map_file_path) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to load font map file '{}'", "error".red(), map_file_path.magenta());
            return None;
        };

        let font_map_reader = ImageReader::with_format(Cursor::new(font_map_data), ImageFormat::Png);

        let Ok(font_map_decoder) = font_map_reader.decode() else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to decode font map '{}'", "error".red(), map_file_path.magenta());
            return None;
        };

        let font_map_rgba_image = font_map_decoder.into_rgba8();
        let font_map_width = font_map_rgba_image.width();
        let font_map_height = font_map_rgba_image.height();

        let Ok(font_description_data) = game_file_loader.get(&map_description_file_path) else {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to load font map description file '{}'",
                "error".red(),
                map_description_file_path.magenta()
            );
            return None;
        };

        let Ok(font_description_content) = String::from_utf8(font_description_data) else {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] invalid UTF-8 text data found in font map description file '{}'",
                "error".red(),
                map_description_file_path.magenta()
            );
            return None;
        };

        let glyphs = parse_glyphs(font_description_content, font_map_width, font_map_height);

        #[cfg(feature = "debug")]
        timer.stop();

        Some(Self {
            ids: Vec::from_iter(ids),
            font_map: font_map_rgba_image,
            glyphs: Arc::new(glyphs),
        })
    }
}
