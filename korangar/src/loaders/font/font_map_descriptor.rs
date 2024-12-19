use cgmath::Point2;
use hashbrown::HashMap;
use korangar_util::Rectangle;

use super::GlyphCoordinate;

/// The CSV file has the following columns:
///
/// 1. glyph identifier index
/// 2. horizontal advance
/// 3. plane bounds (l, b, r, t)
/// 4. atlas bounds (l, b, r, t)
#[allow(unused)]
struct Glyph {
    index: u16,
    advance: f64,
    plane_bounds: Bounds,
    atlas_bounds: Bounds,
}

struct Bounds {
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
}

pub(crate) fn parse_glyph_cache(
    font_description_content: String,
    font_map_width: u32,
    font_map_height: u32,
) -> HashMap<u16, GlyphCoordinate> {
    let font_map_width = font_map_width as f32;
    let font_map_height = font_map_height as f32;

    let glyphs: Vec<Glyph> = font_description_content
        .lines()
        .filter_map(|line| {
            if line.trim().is_empty() {
                return None;
            }

            let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            if values.len() < 10 {
                return None;
            }

            Some(Glyph {
                index: values[0].parse().expect("Failed to parse glyph index"),
                advance: values[1].parse().expect("Failed to parse advance"),
                plane_bounds: Bounds {
                    left: values[2].parse().expect("Failed to parse plane left"),
                    bottom: values[3].parse().expect("Failed to parse plane bottom"),
                    right: values[4].parse().expect("Failed to parse plane right"),
                    top: values[5].parse().expect("Failed to parse plane top"),
                },
                atlas_bounds: Bounds {
                    left: values[6].parse().expect("Failed to parse atlas left"),
                    bottom: values[7].parse().expect("Failed to parse atlas bottom"),
                    right: values[8].parse().expect("Failed to parse atlas right"),
                    top: values[9].parse().expect("Failed to parse atlas top"),
                },
            })
        })
        .collect();

    let mut glyph_map = HashMap::default();

    for glyph in &glyphs {
        let texture_coordinate = Rectangle::new(
            Point2::new(
                glyph.atlas_bounds.left as f32 / font_map_width,
                glyph.atlas_bounds.top as f32 / font_map_height,
            ),
            Point2::new(
                glyph.atlas_bounds.right as f32 / font_map_width,
                glyph.atlas_bounds.bottom as f32 / font_map_height,
            ),
        );

        let width = (glyph.plane_bounds.right - glyph.plane_bounds.left) as f32;
        let height = (glyph.plane_bounds.bottom - glyph.plane_bounds.top) as f32;

        let offset_left = glyph.plane_bounds.left as f32;
        let offset_top = glyph.plane_bounds.top as f32;

        let coordinate = GlyphCoordinate {
            texture_coordinate,
            width,
            height,
            offset_top,
            offset_left,
        };

        glyph_map.insert(glyph.index, coordinate);
    }

    glyph_map
}
