//! Builds the GPU data for the Slug font rendering algorithm from the glyph
//! outlines of a font.
//!
//! The data is split into three buffers:
//!
//! - The curve buffer holds the control points of quadratic Bézier curves. One
//!   entry holds two points `(x1, y1, x2, y2)`. A curve starts at entry `i` and
//!   reads its third point from the first two components of entry `i + 1`.
//!   Connected curves of a contour share that entry.
//! - The band buffer holds the band headers and the curve lists of the bands.
//!   Every glyph starts with one header per horizontal band, followed by one
//!   header per vertical band. A header holds the curve count of the band and
//!   the offset of its curve list relative to the first header of the glyph. A
//!   curve list holds the curve buffer indices of all curves that intersect the
//!   band.
//! - The glyph buffer holds one [`SlugGlyph`] per glyph.
//!
//! All coordinates are in em space, with the y-axis pointing up.

use rayon::prelude::*;
use ttf_parser::{Face, GlyphId, OutlineBuilder};

use crate::graphics::SlugGlyph;

/// The maximum number of bands per direction.
const MAX_BAND_COUNT: usize = 16;
/// The bands overlap by this distance in em space to avoid gaps from rounding.
const BAND_EPSILON: f32 = 1.0 / 1024.0;
/// The number of quadratic curves that approximate one cubic curve.
const CUBIC_SUBDIVISIONS: usize = 4;

/// The em space bounding box of a glyph and its index in the glyph buffer.
#[derive(Copy, Clone, Debug)]
pub struct SlugGlyphBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub glyph_index: u32,
}

/// The CPU side Slug data of all loaded fonts.
#[derive(Default)]
pub struct SlugFontData {
    pub curves: Vec<[f32; 4]>,
    pub bands: Vec<u32>,
    pub glyphs: Vec<SlugGlyph>,
}

#[derive(Copy, Clone)]
struct Curve {
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
}

impl Curve {
    fn min(&self, axis: usize) -> f32 {
        self.p1[axis].min(self.p2[axis]).min(self.p3[axis])
    }

    fn max(&self, axis: usize) -> f32 {
        self.p1[axis].max(self.p2[axis]).max(self.p3[axis])
    }

    /// A straight line parallel to the given axis never crosses a ray that
    /// runs along that axis.
    fn is_parallel_to(&self, axis: usize) -> bool {
        let other = 1 - axis;
        self.p1[other] == self.p2[other] && self.p2[other] == self.p3[other]
    }
}

/// Collects the contours of a glyph outline as quadratic curves in em space.
struct ContourBuilder {
    scale: f32,
    contours: Vec<Vec<Curve>>,
    current: Vec<Curve>,
    start: [f32; 2],
    position: [f32; 2],
}

impl ContourBuilder {
    fn new(units_per_em: f32) -> Self {
        Self {
            scale: 1.0 / units_per_em,
            contours: Vec::new(),
            current: Vec::new(),
            start: [0.0, 0.0],
            position: [0.0, 0.0],
        }
    }

    fn point(&self, x: f32, y: f32) -> [f32; 2] {
        [x * self.scale, y * self.scale]
    }

    fn push_line(&mut self, to: [f32; 2]) {
        if to != self.position {
            // A line is a quadratic curve with the control point duplicated at the end
            // point. This is the most robust encoding for the root solver.
            self.current.push(Curve {
                p1: self.position,
                p2: to,
                p3: to,
            });
            self.position = to;
        }
    }

    fn push_quadratic(&mut self, control: [f32; 2], to: [f32; 2]) {
        if control == self.position || control == to {
            self.push_line(to);
        } else {
            self.current.push(Curve {
                p1: self.position,
                p2: control,
                p3: to,
            });
            self.position = to;
        }
    }

    fn finish_contour(&mut self) {
        let start = self.start;
        self.push_line(start);

        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}

impl OutlineBuilder for ContourBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish_contour();
        self.start = self.point(x, y);
        self.position = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.point(x, y);
        self.push_line(to);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let control = self.point(x1, y1);
        let to = self.point(x, y);
        self.push_quadratic(control, to);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = self.position;
        let c1 = self.point(x1, y1);
        let c2 = self.point(x2, y2);
        let p3 = self.point(x, y);

        let cubic = |t: f32| -> [f32; 2] {
            let u = 1.0 - t;
            let a = u * u * u;
            let b = 3.0 * u * u * t;
            let c = 3.0 * u * t * t;
            let d = t * t * t;
            [
                a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0],
                a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1],
            ]
        };

        let derivative = |t: f32| -> [f32; 2] {
            let u = 1.0 - t;
            let a = 3.0 * u * u;
            let b = 6.0 * u * t;
            let c = 3.0 * t * t;
            [
                a * (c1[0] - p0[0]) + b * (c2[0] - c1[0]) + c * (p3[0] - c2[0]),
                a * (c1[1] - p0[1]) + b * (c2[1] - c1[1]) + c * (p3[1] - c2[1]),
            ]
        };

        // Each segment of the cubic gets approximated by a quadratic curve with the
        // same end points. The control point is the average of the two points that
        // the end tangents of the segment point to.
        for segment in 0..CUBIC_SUBDIVISIONS {
            let t0 = segment as f32 / CUBIC_SUBDIVISIONS as f32;
            let t1 = (segment + 1) as f32 / CUBIC_SUBDIVISIONS as f32;
            let step = (t1 - t0) / 3.0;

            let start = cubic(t0);
            let end = if segment + 1 == CUBIC_SUBDIVISIONS { p3 } else { cubic(t1) };
            let d0 = derivative(t0);
            let d1 = derivative(t1);

            let control = [
                ((start[0] + d0[0] * step * 1.5) + (end[0] - d1[0] * step * 1.5)) * 0.5,
                ((start[1] + d0[1] * step * 1.5) + (end[1] - d1[1] * step * 1.5)) * 0.5,
            ];

            self.push_quadratic(control, end);
        }
    }

    fn close(&mut self) {
        self.finish_contour();
    }
}

/// The Slug data of a single glyph with indices local to the glyph.
struct GlyphBuild {
    bounds: [f32; 4],
    curves: Vec<[f32; 4]>,
    bands: Vec<u32>,
    /// Positions in `bands` that hold curve indices.
    curve_references: Vec<usize>,
    band_transform: [f32; 4],
    horizontal_band_count: usize,
    vertical_band_count: usize,
}

impl SlugFontData {
    /// Converts all glyphs of the face and returns the bounds of every glyph
    /// that has an outline, indexed by glyph ID.
    pub fn add_face(&mut self, face: &Face) -> Vec<Option<SlugGlyphBounds>> {
        let units_per_em = face.units_per_em() as f32;

        let builds: Vec<Option<GlyphBuild>> = (0..face.number_of_glyphs())
            .into_par_iter()
            .map(|glyph_id| build_glyph(face, GlyphId(glyph_id), units_per_em))
            .collect();

        builds
            .into_iter()
            .map(|build| {
                let build = build?;

                let curve_base = self.curves.len() as u32;
                let band_base = self.bands.len();

                self.curves.extend_from_slice(&build.curves);
                self.bands.extend_from_slice(&build.bands);

                for &reference in &build.curve_references {
                    self.bands[band_base + reference] += curve_base;
                }

                let glyph_index = self.glyphs.len() as u32;

                self.glyphs.push(SlugGlyph {
                    band_transform: build.band_transform,
                    band_base: band_base as u32,
                    band_max_x: build.vertical_band_count as u32 - 1,
                    band_max_y: build.horizontal_band_count as u32 - 1,
                    padding: 0,
                });

                let [x_min, y_min, x_max, y_max] = build.bounds;

                Some(SlugGlyphBounds {
                    x_min,
                    y_min,
                    x_max,
                    y_max,
                    glyph_index,
                })
            })
            .collect()
    }

    /// Bindings of empty buffers are invalid, so every buffer gets at least
    /// one entry.
    pub fn ensure_not_empty(&mut self) {
        if self.curves.is_empty() {
            self.curves.push([0.0; 4]);
        }
        if self.bands.is_empty() {
            self.bands.push(0);
        }
        if self.glyphs.is_empty() {
            self.glyphs.push(SlugGlyph::default());
        }
    }
}

fn build_glyph(face: &Face, glyph_id: GlyphId, units_per_em: f32) -> Option<GlyphBuild> {
    let mut builder = ContourBuilder::new(units_per_em);
    face.outline_glyph(glyph_id, &mut builder)?;
    builder.finish_contour();

    let contours = builder.contours;

    if contours.is_empty() {
        return None;
    }

    // Write the contours into the curve list. Curve `k` of a contour starts at
    // entry `start + k`, and the contour ends with one entry for the last point.
    let curve_count: usize = contours.iter().map(Vec::len).sum();
    let mut curves = Vec::with_capacity(curve_count + contours.len());
    let mut indexed_curves = Vec::with_capacity(curve_count);

    for contour in &contours {
        for curve in contour {
            indexed_curves.push((curves.len() as u32, *curve));
            curves.push([curve.p1[0], curve.p1[1], curve.p2[0], curve.p2[1]]);
        }

        let last = contour.last().unwrap();
        curves.push([last.p3[0], last.p3[1], 0.0, 0.0]);
    }

    let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];

    for (_, curve) in &indexed_curves {
        bounds[0] = bounds[0].min(curve.min(0));
        bounds[1] = bounds[1].min(curve.min(1));
        bounds[2] = bounds[2].max(curve.max(0));
        bounds[3] = bounds[3].max(curve.max(1));
    }

    // Horizontal bands slice the glyph along the y axis and vertical bands along
    // the x axis.
    let (horizontal_band_count, horizontal_scale, horizontal_offset) = band_layout(&indexed_curves, 1, bounds[1], bounds[3]);
    let (vertical_band_count, vertical_scale, vertical_offset) = band_layout(&indexed_curves, 0, bounds[0], bounds[2]);

    let horizontal_bands = collect_bands(&indexed_curves, 1, horizontal_band_count, horizontal_scale, horizontal_offset);
    let vertical_bands = collect_bands(&indexed_curves, 0, vertical_band_count, vertical_scale, vertical_offset);

    let header_count = horizontal_band_count + vertical_band_count;
    let mut bands = vec![0; header_count * 2];
    let mut curve_references = Vec::new();

    let mut previous: Option<(&Vec<u32>, u32)> = None;

    for (band_index, band) in horizontal_bands.iter().chain(vertical_bands.iter()).enumerate() {
        // The first vertical band never shares the list of the last horizontal band,
        // because the lists are sorted along different axes.
        if band_index == horizontal_band_count {
            previous = None;
        }

        let offset = match previous {
            Some((previous_band, previous_offset)) if previous_band == band => previous_offset,
            _ => {
                let offset = bands.len() as u32;
                curve_references.extend(bands.len()..bands.len() + band.len());
                bands.extend_from_slice(band);
                offset
            }
        };

        bands[band_index * 2] = band.len() as u32;
        bands[band_index * 2 + 1] = offset;
        previous = Some((band, offset));
    }

    Some(GlyphBuild {
        bounds,
        curves,
        bands,
        curve_references,
        band_transform: [vertical_scale, horizontal_scale, vertical_offset, horizontal_offset],
        horizontal_band_count,
        vertical_band_count,
    })
}

/// The range of band indices that a curve covers along the axis.
fn band_range(curve: &Curve, axis: usize, band_count: usize, scale: f32, offset: f32) -> (usize, usize) {
    let last = band_count as i32 - 1;
    let low = (((curve.min(axis) - BAND_EPSILON) * scale + offset).floor() as i32).clamp(0, last);
    let high = (((curve.max(axis) + BAND_EPSILON) * scale + offset).floor() as i32).clamp(0, last);
    (low as usize, high as usize)
}

/// Picks the band count that minimizes the largest number of curves in any
/// band. Ties go to the smaller total number of band entries.
fn band_layout(curves: &[(u32, Curve)], axis: usize, minimum: f32, maximum: f32) -> (usize, f32, f32) {
    let extent = maximum - minimum;

    if extent <= 0.0 {
        return (1, 0.0, 0.0);
    }

    let ray_axis = 1 - axis;
    let maximum_band_count = curves.len().clamp(1, MAX_BAND_COUNT);
    let mut best = (i32::MAX, i32::MAX, 1);
    let mut counts = [0i32; MAX_BAND_COUNT + 1];

    for band_count in 1..=maximum_band_count {
        let scale = band_count as f32 / extent;
        let offset = -minimum * scale;

        counts.fill(0);

        for (_, curve) in curves.iter().filter(|(_, curve)| !curve.is_parallel_to(ray_axis)) {
            let (low, high) = band_range(curve, axis, band_count, scale, offset);
            counts[low] += 1;
            counts[high + 1] -= 1;
        }

        let mut running = 0;
        let mut largest = 0;
        let mut total = 0;

        for count in counts.iter().take(band_count) {
            running += count;
            largest = largest.max(running);
            total += running;
        }

        if (largest, total) < (best.0, best.1) {
            best = (largest, total, band_count);
        }
    }

    let band_count = best.2;
    let scale = band_count as f32 / extent;
    (band_count, scale, -minimum * scale)
}

/// Collects the curves of every band. A band along the y-axis is crossed by
/// rays along the x-axis and vice versa. The curves of a band are sorted in
/// descending order of their maximum coordinate along the ray axis, which
/// lets the pixel shader stop early.
fn collect_bands(curves: &[(u32, Curve)], axis: usize, band_count: usize, scale: f32, offset: f32) -> Vec<Vec<u32>> {
    let ray_axis = 1 - axis;
    let mut bands: Vec<Vec<(u32, f32)>> = vec![Vec::new(); band_count];

    for (index, curve) in curves.iter().filter(|(_, curve)| !curve.is_parallel_to(ray_axis)) {
        let (low, high) = band_range(curve, axis, band_count, scale, offset);

        for band in &mut bands[low..=high] {
            band.push((*index, curve.max(ray_axis)));
        }
    }

    bands
        .into_iter()
        .map(|mut band| {
            band.sort_by(|a, b| b.1.total_cmp(&a.1));
            band.into_iter().map(|(index, _)| index).collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ttf_parser::Face;

    use super::SlugFontData;
    use crate::graphics::SlugGlyph;

    static FONT_DATA: &[u8] = include_bytes!("../../../archive/data/font/NotoSans.ttf");

    const PIXELS_PER_EM: f32 = 1.0e5;
    const GRID_SIZE: usize = 12;

    fn root_code(y1: f32, y2: f32, y3: f32) -> u32 {
        let i1 = y1.to_bits() >> 31;
        let i2 = y2.to_bits() >> 30;
        let i3 = y3.to_bits() >> 29;
        let shift = (i2 & 2) | (i1 & !2);
        let shift = (i3 & 4) | (shift & !4);
        (0x2E74 >> shift) & 0x0101
    }

    fn solve(p12: [f32; 4], p3: [f32; 2], axis: usize) -> [f32; 2] {
        let other = 1 - axis;
        let a = [p12[0] - p12[2] * 2.0 + p3[0], p12[1] - p12[3] * 2.0 + p3[1]];
        let b = [p12[0] - p12[2], p12[1] - p12[3]];
        let c = [p12[0], p12[1]];

        let d = (b[other] * b[other] - a[other] * c[other]).max(0.0).sqrt();
        let (mut t1, mut t2) = ((b[other] - d) / a[other], (b[other] + d) / a[other]);

        if a[other].abs() < 1.0 / 65536.0 {
            t1 = c[other] * 0.5 / b[other];
            t2 = t1;
        }

        [
            (a[axis] * t1 - b[axis] * 2.0) * t1 + c[axis],
            (a[axis] * t2 - b[axis] * 2.0) * t2 + c[axis],
        ]
    }

    /// CPU port of the coverage calculation of the Slug pixel shader.
    fn coverage(data: &SlugFontData, glyph: &SlugGlyph, point: [f32; 2]) -> f32 {
        let band_max = [glyph.band_max_x as i32, glyph.band_max_y as i32];
        let band_index = [
            ((point[0] * glyph.band_transform[0] + glyph.band_transform[2]) as i32).clamp(0, band_max[0]),
            ((point[1] * glyph.band_transform[1] + glyph.band_transform[3]) as i32).clamp(0, band_max[1]),
        ];
        let base = glyph.band_base as usize;

        let mut result = [(0.0f32, 0.0f32); 2];

        // Rays along the x-axis use the horizontal bands, rays along the y-axis the
        // vertical bands.
        for axis in 0..2 {
            let header = match axis {
                0 => base + band_index[1] as usize * 2,
                _ => base + (band_max[1] + 1 + band_index[0]) as usize * 2,
            };
            let count = data.bands[header] as usize;
            let list = base + data.bands[header + 1] as usize;
            let other = 1 - axis;
            let sign = if axis == 0 { 1.0 } else { -1.0 };

            let (mut sum, mut weight) = (0.0f32, 0.0f32);

            for &location in &data.bands[list..list + count] {
                let entry = data.curves[location as usize];
                let next = data.curves[location as usize + 1];
                let p12 = [entry[0] - point[0], entry[1] - point[1], entry[2] - point[0], entry[3] - point[1]];
                let p3 = [next[0] - point[0], next[1] - point[1]];

                if p12[axis].max(p12[axis + 2]).max(p3[axis]) * PIXELS_PER_EM < -0.5 {
                    break;
                }

                let code = root_code(p12[other], p12[other + 2], p3[other]);

                if code != 0 {
                    let roots = solve(p12, p3, axis).map(|root| root * PIXELS_PER_EM);

                    if code & 1 != 0 {
                        sum += sign * (roots[0] + 0.5).clamp(0.0, 1.0);
                        weight = weight.max((1.0 - roots[0].abs() * 2.0).clamp(0.0, 1.0));
                    }

                    if code > 1 {
                        sum -= sign * (roots[1] + 0.5).clamp(0.0, 1.0);
                        weight = weight.max((1.0 - roots[1].abs() * 2.0).clamp(0.0, 1.0));
                    }
                }
            }

            result[axis] = (sum, weight);
        }

        let [(x_coverage, x_weight), (y_coverage, y_weight)] = result;

        let coverage = ((x_coverage * x_weight + y_coverage * y_weight).abs() / (x_weight + y_weight).max(1.0 / 65536.0))
            .max(x_coverage.abs().min(y_coverage.abs()));

        coverage.clamp(0.0, 1.0)
    }

    /// The nonzero winding number of the outline around the point, computed
    /// from all curves of the glyph without the bands.
    fn winding_number(curves: &[(f32, f32, f32, f32, f32, f32)], point: [f32; 2]) -> i32 {
        let mut winding = 0;

        for &(x1, y1, x2, y2, x3, y3) in curves {
            let [x1, y1, x2, y2, x3, y3] = [x1, y1, x2, y2, x3, y3].map(f64::from);
            let a = y1 - 2.0 * y2 + y3;
            let b = 2.0 * (y2 - y1);
            let c = y1 - f64::from(point[1]);

            let mut roots = Vec::new();

            if a.abs() < 1e-12 {
                if b != 0.0 {
                    roots.push(-c / b);
                }
            } else {
                let discriminant = b * b - 4.0 * a * c;
                if discriminant >= 0.0 {
                    let q = -0.5 * (b + b.signum() * discriminant.sqrt());
                    roots.push(q / a);
                    if q != 0.0 {
                        roots.push(c / q);
                    }
                }
            }

            for t in roots.into_iter().filter(|t| (0.0..1.0).contains(t)) {
                let u = 1.0 - t;
                let x = u * u * x1 + 2.0 * u * t * x2 + t * t * x3;
                let dy = 2.0 * a * t + b;

                if x > f64::from(point[0]) && dy != 0.0 {
                    winding += if dy > 0.0 { 1 } else { -1 };
                }
            }
        }

        winding
    }

    #[test]
    fn coverage_matches_winding_number() {
        let face = Face::parse(FONT_DATA, 0).unwrap();
        let mut data = SlugFontData::default();
        let bounds = data.add_face(&face);

        let mut samples = 0;
        let mut mismatches = 0;

        for glyph_bounds in bounds.iter().flatten() {
            let glyph = data.glyphs[glyph_bounds.glyph_index as usize];

            // Collect all curves of the glyph by walking its band lists.
            let mut locations: Vec<u32> = Vec::new();
            let band_count = (glyph.band_max_x + glyph.band_max_y + 2) as usize;
            for band in 0..band_count {
                let header = glyph.band_base as usize + band * 2;
                let count = data.bands[header] as usize;
                let list = glyph.band_base as usize + data.bands[header + 1] as usize;
                locations.extend_from_slice(&data.bands[list..list + count]);
            }
            locations.sort_unstable();
            locations.dedup();

            let curves: Vec<_> = locations
                .iter()
                .map(|&location| {
                    let entry = data.curves[location as usize];
                    let next = data.curves[location as usize + 1];
                    (entry[0], entry[1], entry[2], entry[3], next[0], next[1])
                })
                .collect();

            let width = glyph_bounds.x_max - glyph_bounds.x_min;
            let height = glyph_bounds.y_max - glyph_bounds.y_min;

            for row in 0..GRID_SIZE {
                for column in 0..GRID_SIZE {
                    // Offsets with irrational ratios keep the samples away from the exact
                    // coordinates of the control points.
                    let point = [
                        glyph_bounds.x_min + width * ((column as f32 + 0.5 + 0.0731) / GRID_SIZE as f32),
                        glyph_bounds.y_min + height * ((row as f32 + 0.5 + 0.0377) / GRID_SIZE as f32),
                    ];

                    let expected = winding_number(&curves, point) != 0;
                    let actual = coverage(&data, &glyph, point) > 0.5;

                    samples += 1;
                    if expected != actual {
                        mismatches += 1;
                    }
                }
            }
        }

        assert!(samples > 0);
        assert!(
            mismatches * 10_000 < samples,
            "{mismatches} of {samples} samples have the wrong coverage"
        );
    }

    #[test]
    fn band_data_is_consistent() {
        let face = Face::parse(FONT_DATA, 0).unwrap();
        let mut data = SlugFontData::default();
        let bounds = data.add_face(&face);

        for glyph_bounds in bounds.iter().flatten() {
            let glyph = data.glyphs[glyph_bounds.glyph_index as usize];
            let band_count = (glyph.band_max_x + glyph.band_max_y + 2) as usize;

            for band in 0..band_count {
                let header = glyph.band_base as usize + band * 2;
                let count = data.bands[header] as usize;
                let list = glyph.band_base as usize + data.bands[header + 1] as usize;
                let axis = if band <= glyph.band_max_y as usize { 0 } else { 1 };

                let maxima: Vec<f32> = data.bands[list..list + count]
                    .iter()
                    .map(|&location| {
                        let entry = data.curves[location as usize];
                        let next = data.curves[location as usize + 1];
                        entry[axis].max(entry[axis + 2]).max(next[axis])
                    })
                    .collect();

                assert!(maxima.windows(2).all(|pair| pair[0] >= pair[1]), "band curves are not sorted");
            }
        }
    }
}
