#language slang 2026

// Glyph rendering with the Slug algorithm by Eric Lengyel.
// Reference: https://github.com/EricLengyel/Slug/tree/main
// SPDX-License-Identifier: MIT OR Apache-2.0
// The patent has been dedicated to the public domain in 2026.

module slug;

public struct SlugGlyph {
    // Scale (xy) and offset (zw) that map an em space coordinate to a band index.
    public var band_transform: float4;
    // Index of the first band header of the glyph in the band buffer.
    public var band_base: uint;
    // Index of the last vertical band.
    public var band_max_x: uint;
    // Index of the last horizontal band.
    public var band_max_y: uint;
    public var padding: uint;
};

// Calculates the root eligibility code for a sample relative quadratic Bezier curve
// from the signs of the y coordinates of its three control points. Bit 0 tells if the
// first root contributes and bit 8 tells if the second root contributes.
[ForceInline]
func calculate_root_code(y1: float, y2: float, y3: float) -> uint {
    let i1 = asuint(y1) >> 31u;
    let i2 = asuint(y2) >> 30u;
    let i3 = asuint(y3) >> 29u;

    var shift = (i2 & 2u) | (i1 & ~2u);
    shift = (i3 & 4u) | (shift & ~4u);

    return (0x2E74u >> shift) & 0x0101u;
}

// Solves for the x coordinates where the curve crosses y = 0. The polynomial in t is
// a t^2 - 2b t + c with a = p1.y - 2 p2.y + p3.y, b = p1.y - p2.y and c = p1.y.
// Imaginary roots are treated as a double root at the minimum where t = b / a.
[ForceInline]
func solve_horizontal_polynomial(p12: float4, p3: float2) -> float2 {
    let a = p12.xy - p12.zw * 2.0 + p3;
    let b = p12.xy - p12.zw;
    let ra = 1.0 / a.y;
    let rb = 0.5 / b.y;

    let d = sqrt(max(b.y * b.y - a.y * p12.y, 0.0));
    var t1 = (b.y - d) * ra;
    var t2 = (b.y + d) * ra;

    // A nearly linear polynomial is solved as -2b t + c = 0.
    if (abs(a.y) < 1.0 / 65536.0) {
        t1 = p12.y * rb;
        t2 = t1;
    }

    return float2((a.x * t1 - b.x * 2.0) * t1 + p12.x, (a.x * t2 - b.x * 2.0) * t2 + p12.x);
}

// Solves for the y coordinates where the curve crosses x = 0.
[ForceInline]
func solve_vertical_polynomial(p12: float4, p3: float2) -> float2 {
    let a = p12.xy - p12.zw * 2.0 + p3;
    let b = p12.xy - p12.zw;
    let ra = 1.0 / a.x;
    let rb = 0.5 / b.x;

    let d = sqrt(max(b.x * b.x - a.x * p12.x, 0.0));
    var t1 = (b.x - d) * ra;
    var t2 = (b.x + d) * ra;

    if (abs(a.x) < 1.0 / 65536.0) {
        t1 = p12.x * rb;
        t2 = t1;
    }

    return float2((a.y * t1 - b.y * 2.0) * t1 + p12.y, (a.y * t2 - b.y * 2.0) * t2 + p12.y);
}

// The number of pixels per em along both axes. Must be called in uniform control flow.
[ForceInline]
public func slug_pixels_per_em(render_coordinate: float2) -> float2 {
    return 1.0 / fwidth(render_coordinate);
}

// Calculates the coverage of the pixel at the em space coordinate by the glyph, using the
// nonzero fill rule.
public func slug_coverage(
    curves: StructuredBuffer<float4>,
    bands: StructuredBuffer<uint>,
    glyph: SlugGlyph,
    render_coordinate: float2,
    pixels_per_em: float2,
) -> float {
    let band_max = int2(int(glyph.band_max_x), int(glyph.band_max_y));
    let band_index = clamp(int2(render_coordinate * glyph.band_transform.xy + glyph.band_transform.zw), int2(0, 0), band_max);
    let base = glyph.band_base;

    var x_coverage = 0.0;
    var x_weight = 0.0;

    // The header of a band holds the number of curves in the band and the offset of its
    // curve list relative to the first header of the glyph.
    let horizontal_header = base + uint(band_index.y) * 2u;
    let horizontal_count = bands[horizontal_header];
    let horizontal_list = base + bands[horizontal_header + 1u];

    for (var curve_index = 0u; curve_index < horizontal_count; curve_index++) {
        let curve_location = bands[horizontal_list + curve_index];

        // The first entry holds p1 and p2, the next entry holds p3 in its first two
        // components. Subtracting the render coordinate places the sample at the origin.
        let p12 = curves[curve_location] - float4(render_coordinate, render_coordinate);
        let p3 = curves[curve_location + 1u].xy - render_coordinate;

        // The curves are sorted in descending order of their maximum x coordinate. Once a
        // curve lies left of the pixel, no further curve can contribute.
        if (max(max(p12.x, p12.z), p3.x) * pixels_per_em.x < -0.5) {
            break;
        }

        let code = calculate_root_code(p12.y, p12.w, p3.y);

        if (code != 0u) {
            let roots = solve_horizontal_polynomial(p12, p3) * pixels_per_em.x;

            if ((code & 1u) != 0u) {
                x_coverage += saturate(roots.x + 0.5);
                x_weight = max(x_weight, saturate(1.0 - abs(roots.x) * 2.0));
            }

            if (code > 1u) {
                x_coverage -= saturate(roots.y + 0.5);
                x_weight = max(x_weight, saturate(1.0 - abs(roots.y) * 2.0));
            }
        }
    }

    var y_coverage = 0.0;
    var y_weight = 0.0;

    // The vertical band headers follow the horizontal band headers.
    let vertical_header = base + uint(band_max.y + 1 + band_index.x) * 2u;
    let vertical_count = bands[vertical_header];
    let vertical_list = base + bands[vertical_header + 1u];

    for (var curve_index = 0u; curve_index < vertical_count; curve_index++) {
        let curve_location = bands[vertical_list + curve_index];
        let p12 = curves[curve_location] - float4(render_coordinate, render_coordinate);
        let p3 = curves[curve_location + 1u].xy - render_coordinate;

        // The curves are sorted in descending order of their maximum y coordinate.
        if (max(max(p12.y, p12.w), p3.y) * pixels_per_em.y < -0.5) {
            break;
        }

        let code = calculate_root_code(p12.x, p12.z, p3.x);

        if (code != 0u) {
            let roots = solve_vertical_polynomial(p12, p3) * pixels_per_em.y;

            if ((code & 1u) != 0u) {
                y_coverage -= saturate(roots.x + 0.5);
                y_weight = max(y_weight, saturate(1.0 - abs(roots.x) * 2.0));
            }

            if (code > 1u) {
                y_coverage += saturate(roots.y + 0.5);
                y_weight = max(y_weight, saturate(1.0 - abs(roots.y) * 2.0));
            }
        }
    }

    // Combine the coverage of both rays by their weights. The absolute values make both
    // winding directions work.
    let coverage = max(
        abs(x_coverage * x_weight + y_coverage * y_weight) / max(x_weight + y_weight, 1.0 / 65536.0),
        min(abs(x_coverage), abs(y_coverage))
    );

    return saturate(coverage);
}
