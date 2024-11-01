// NVIDIA FXAA 3.11 by TIMOTHY LOTTES
//
// ------------------------------------------------------------------------------
// COPYRIGHT (C) 2010, 2011 NVIDIA CORPORATION. ALL RIGHTS RESERVED.
// ------------------------------------------------------------------------------
// TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, THIS SOFTWARE IS PROVIDED
// *AS IS* AND NVIDIA AND ITS SUPPLIERS DISCLAIM ALL WARRANTIES, EITHER EXPRESS
// OR IMPLIED, INCLUDING, BUT NOT LIMITED TO, IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE. IN NO EVENT SHALL NVIDIA
// OR ITS SUPPLIERS BE LIABLE FOR ANY SPECIAL, INCIDENTAL, INDIRECT, OR
// CONSEQUENTIAL DAMAGES WHATSOEVER (INCLUDING, WITHOUT LIMITATION, DAMAGES FOR
// LOSS OF BUSINESS PROFITS, BUSINESS INTERRUPTION, LOSS OF BUSINESS INFORMATION,
// OR ANY OTHER PECUNIARY LOSS) ARISING OUT OF THE USE OF OR INABILITY TO USE
// THIS SOFTWARE, EVEN IF NVIDIA HAS BEEN ADVISED OF THE POSSIBILITY OF SUCH
// DAMAGES.

// Choose the amount of sub-pixel aliasing removal. This can effect sharpness.
//   1.00 - upper limit (softer)
//   0.75 - default amount of filtering
//   0.50 - lower limit (sharper, less sub-pixel aliasing removal)
//   0.25 - almost off
//   0.00 - completely off
const QUALITY_SUBPIX: f32 = 0.50;

// Trims the algorithm from processing darks.
//   0.0833 - upper limit (default, the start of visible unfiltered edges)
//   0.0625 - high quality (faster)
//   0.0312 - visible limit (slower)
const QUALITY_EDGE_THRESHOLD_MIN: f32 = 0.0833;

// The minimum amount of local contrast required to apply algorithm.
//   0.333 - too little (faster)
//   0.250 - low quality
//   0.166 - default
//   0.125 - high quality
//   0.063 - overkill (slower)
const QUALITY_EDGE_THRESHOLD_MAX: f32 = 0.125;

// FXAA_QUALITY__PRESET == 15
const QUALITY_STEPS_LENGTH: i32 = 8;
const QUALITY_STEPS = array<f32, QUALITY_STEPS_LENGTH>(1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 12.0);

/// Input texture must be:
///   - Luma must be stored in alpha channel from a previous pass
///   - Luma should be calculates as: sqrt(dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114)))
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

// This section in "as close as" the original code, so that it can be compared
// easier against the original implementation, in case there is a bug.
@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(texture));
    let rcpFrame = vec2<f32>(1.0) / resolution;
    let posM = position.xy * rcpFrame;

    let rgbyM = textureSample(texture, linear_sampler, posM);
    let lumaM = rgbyM.a;

    var lumaS = textureSample(texture, linear_sampler, posM, vec2<i32>( 0, 1)).a;
    let lumaE = textureSample(texture, linear_sampler, posM, vec2<i32>( 1, 0)).a;
    var lumaN = textureSample(texture, linear_sampler, posM, vec2<i32>( 0,-1)).a;
    let lumaW = textureSample(texture, linear_sampler, posM, vec2<i32>(-1, 0)).a;

    let maxSM = max(lumaS, lumaM);
    let minSM = min(lumaS, lumaM);
    let maxESM = max(lumaE, maxSM);
    let minESM = min(lumaE, minSM);
    let maxWN = max(lumaW, lumaN);
    let minWN = min(lumaW, lumaN);
    let rangeMax = max(maxWN, maxESM);
    let rangeMin = min(minWN, minESM);
    let rangeMaxScaled = rangeMax * QUALITY_EDGE_THRESHOLD_MAX;
    let range = rangeMax - rangeMin;
    let rangeMaxClamped = max(QUALITY_EDGE_THRESHOLD_MIN, rangeMaxScaled);

    if (range < rangeMaxClamped) {
        return vec4<f32>(rgbyM.rgb, 1.0);
    }

    let lumaNW = textureSample(texture, linear_sampler, posM, vec2<i32>(-1,-1)).a;
    let lumaSE = textureSample(texture, linear_sampler, posM, vec2<i32>( 1, 1)).a;
    let lumaNE = textureSample(texture, linear_sampler, posM, vec2<i32>( 1,-1)).a;
    let lumaSW = textureSample(texture, linear_sampler, posM, vec2<i32>(-1, 1)).a;

    let lumaNS = lumaN + lumaS;
    let lumaWE = lumaW + lumaE;
    let subpixRcpRange = 1.0 / range;
    let subpixNSWE = lumaNS + lumaWE;
    let edgeHorz1 = (-2.0 * lumaM) + lumaNS;
    let edgeVert1 = (-2.0 * lumaM) + lumaWE;

    let lumaNESE = lumaNE + lumaSE;
    let lumaNWNE = lumaNW + lumaNE;
    let edgeHorz2 = (-2.0 * lumaE) + lumaNESE;
    let edgeVert2 = (-2.0 * lumaN) + lumaNWNE;

    let lumaNWSW = lumaNW + lumaSW;
    let lumaSWSE = lumaSW + lumaSE;
    let edgeHorz4 = (abs(edgeHorz1) * 2.0) + abs(edgeHorz2);
    let edgeVert4 = (abs(edgeVert1) * 2.0) + abs(edgeVert2);
    let edgeHorz3 = (-2.0 * lumaW) + lumaNWSW;
    let edgeVert3 = (-2.0 * lumaS) + lumaSWSE;
    let edgeHorz = abs(edgeHorz3) + edgeHorz4;
    let edgeVert = abs(edgeVert3) + edgeVert4;

    let subpixNWSWNESE = lumaNWSW + lumaNESE;
    let isHorizontal = edgeHorz >= edgeVert;
    let subpixA = subpixNSWE * 2.0 + subpixNWSWNESE;
    let subpixB = (subpixA * (1.0/12.0)) - lumaM;

    lumaN = select(lumaN, lumaW, !isHorizontal);
    lumaS = select(lumaS, lumaE, !isHorizontal);
    var lengthSign = select(rcpFrame.x, rcpFrame.y, isHorizontal);

    let gradientN = lumaN - lumaM;
    let gradientS = lumaS - lumaM;
    var lumaNN = lumaN + lumaM;
    let lumaSS = lumaS + lumaM;
    let pairN = abs(gradientN) >= abs(gradientS);
    let gradient = max(abs(gradientN), abs(gradientS));
    lengthSign = select(lengthSign, -lengthSign, pairN);
    let subpixC = saturate(abs(subpixB) * subpixRcpRange);

    let posB = vec2<f32>(
        select(posM.x, posM.x + lengthSign * 0.5, !isHorizontal),
        select(posM.y, posM.y + lengthSign * 0.5, isHorizontal)
    );

    let offNP = vec2<f32>(
        select(rcpFrame.x, 0.0, !isHorizontal),
        select(rcpFrame.y, 0.0, isHorizontal)
    );

    var posN = posB - offNP * QUALITY_STEPS[0];
    var posP = posB + offNP * QUALITY_STEPS[0];

    let subpixD = ((-2.0) * subpixC) + 3.0;
    var lumaEndN = textureSample(texture, linear_sampler, posN).a;
    let subpixE = subpixC * subpixC;
    var lumaEndP = textureSample(texture, linear_sampler, posP).a;

    lumaNN = select(lumaNN, lumaSS, !pairN);
    let gradientScaled = gradient * 1.0 / 4.0;
    let lumaMM = lumaM - lumaNN * 0.5;
    let subpixF = subpixD * subpixE;
    let lumaMLTZero = lumaMM < 0.0;

    lumaEndN -= lumaNN * 0.5;
    lumaEndP -= lumaNN * 0.5;
    var doneN = abs(lumaEndN) >= gradientScaled;
    var doneP = abs(lumaEndP) >= gradientScaled;

    posN = select(posN, posN - offNP * QUALITY_STEPS[1], !doneN);
    posP = select(posP, posP + offNP * QUALITY_STEPS[1], !doneP);

    if (!(doneN && doneP)) {
        for (var i = 2; i < QUALITY_STEPS_LENGTH; i++) {
            if (!doneN) {
                lumaEndN = textureSample(texture, linear_sampler, posN).a;
            }

            if (!doneP) {
                lumaEndP = textureSample(texture, linear_sampler, posP).a;
            }

            if (!doneN) {
                lumaEndN -= lumaNN * 0.5;
            }

            if (!doneP) {
                lumaEndP -= lumaNN * 0.5;
            }

            doneN = abs(lumaEndN) >= gradientScaled;
            doneP = abs(lumaEndP) >= gradientScaled;

            if (!doneN) {
                posN -= offNP * QUALITY_STEPS[i];
            }

            if (!doneP) {
                posP += offNP * QUALITY_STEPS[i];
            }

            if (doneN && doneP) {
                break;
            }
        }
    }

    let dstN = select(posM.x - posN.x, posM.y - posN.y, !isHorizontal);
    let dstP = select(posP.x - posM.x, posP.y - posM.y, !isHorizontal);

    let goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
    let spanLength = dstP + dstN;
    let goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
    let spanLengthRcp = 1.0 / spanLength;

    let directionN = dstN < dstP;
    let dst = min(dstN, dstP);
    let goodSpan = select(goodSpanP, goodSpanN, directionN);
    let subpixG = subpixF * subpixF;
    let pixelOffset = (dst * (-spanLengthRcp)) + 0.5;
    let subpixH = subpixG * QUALITY_SUBPIX;

    let pixelOffsetGood = select(0.0, pixelOffset, goodSpan);
    let pixelOffsetSubpix = max(pixelOffsetGood, subpixH);

    let finalPos = vec2<f32>(
        select(posM.x, posM.x + pixelOffsetSubpix * lengthSign, !isHorizontal),
        select(posM.y, posM.y + pixelOffsetSubpix * lengthSign, isHorizontal)
    );

    let finalColor = textureSample(texture, linear_sampler, finalPos);
    return vec4<f32>(finalColor.rgb, 1.0);
}
