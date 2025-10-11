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
#language slang 2026

import screen_space;

// Choose the amount of sub-pixel aliasing removal. This can effect sharpness.
//   1.00 - upper limit (softer)
//   0.75 - default amount of filtering
//   0.50 - lower limit (sharper, less sub-pixel aliasing removal)
//   0.25 - almost off
//   0.00 - completely off
static const var QUALITY_SUBPIX: float = 0.50;

// Trims the algorithm from processing darks.
//   0.0833 - upper limit (default, the start of visible unfiltered edges)
//   0.0625 - high quality (faster)
//   0.0312 - visible limit (slower)
static const var QUALITY_EDGE_THRESHOLD_MIN: float = 0.0833;

// The minimum amount of local contrast required to apply algorithm.
//   0.333 - too little (faster)
//   0.250 - low quality
//   0.166 - default
//   0.125 - high quality
//   0.063 - overkill (slower)
static const var QUALITY_EDGE_THRESHOLD_MAX: float = 0.125;

// FXAA_QUALITY__PRESET == 15
static const var QUALITY_STEPS_LENGTH: int = 8;
static const var QUALITY_STEPS: float[8] = {1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 12.0};

// Input texture must be:
//   - Luma must be stored in alpha channel from a previous pass
//   - Luma should be calculates as: sqrt(dot(color.rgb, float3(0.299, 0.587, 0.114)))
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

// This section in "as close as" the original code, so that it can be compared
// easier against the original implementation, in case there is a bug.
[[shader("pixel")]]
func fs_main(FullscreenVertex input) -> float4 {
    var texDims: uint2;
    texture.GetDimensions(texDims.x, texDims.y);
    let resolution = float2(texDims);
    let rcpFrame = float2(1.0) / resolution;
    let posM = input.position.xy * rcpFrame;

    let rgbyM = texture.Sample(linear_sampler, posM);
    let lumaM = rgbyM.a;

    var lumaS = texture.Sample(linear_sampler, posM, int2( 0, 1)).a;
    let lumaE = texture.Sample(linear_sampler, posM, int2( 1, 0)).a;
    var lumaN = texture.Sample(linear_sampler, posM, int2( 0,-1)).a;
    let lumaW = texture.Sample(linear_sampler, posM, int2(-1, 0)).a;

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
        return float4(rgbyM.rgb, 1.0);
    }

    let lumaNW = texture.Sample(linear_sampler, posM, int2(-1,-1)).a;
    let lumaSE = texture.Sample(linear_sampler, posM, int2( 1, 1)).a;
    let lumaNE = texture.Sample(linear_sampler, posM, int2( 1,-1)).a;
    let lumaSW = texture.Sample(linear_sampler, posM, int2(-1, 1)).a;

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

    lumaN = !isHorizontal ? lumaW : lumaN;
    lumaS = !isHorizontal ? lumaE : lumaS;
    var lengthSign = isHorizontal ? rcpFrame.y : rcpFrame.x;

    let gradientN = lumaN - lumaM;
    let gradientS = lumaS - lumaM;
    var lumaNN = lumaN + lumaM;
    let lumaSS = lumaS + lumaM;
    let pairN = abs(gradientN) >= abs(gradientS);
    let gradient = max(abs(gradientN), abs(gradientS));
    lengthSign = pairN ? -lengthSign : lengthSign;
    let subpixC = saturate(abs(subpixB) * subpixRcpRange);

    let posB = float2(
        !isHorizontal ? posM.x + lengthSign * 0.5 : posM.x,
        isHorizontal ? posM.y + lengthSign * 0.5 : posM.y
    );

    let offNP = float2(
        !isHorizontal ? 0.0 : rcpFrame.x,
        isHorizontal ? 0.0 : rcpFrame.y
    );

    var posN = posB - offNP * QUALITY_STEPS[0];
    var posP = posB + offNP * QUALITY_STEPS[0];

    let subpixD = ((-2.0) * subpixC) + 3.0;
    var lumaEndN = texture.Sample(linear_sampler, posN).a;
    let subpixE = subpixC * subpixC;
    var lumaEndP = texture.Sample(linear_sampler, posP).a;

    lumaNN = !pairN ? lumaSS : lumaNN;
    let gradientScaled = gradient * 1.0 / 4.0;
    let lumaMM = lumaM - lumaNN * 0.5;
    let subpixF = subpixD * subpixE;
    let lumaMLTZero = lumaMM < 0.0;

    lumaEndN -= lumaNN * 0.5;
    lumaEndP -= lumaNN * 0.5;
    var doneN = abs(lumaEndN) >= gradientScaled;
    var doneP = abs(lumaEndP) >= gradientScaled;

    posN = !doneN ? posN - offNP * QUALITY_STEPS[1] : posN;
    posP = !doneP ? posP + offNP * QUALITY_STEPS[1] : posP;

    if (!(doneN && doneP)) {
        [unroll]
        for (int i = 2; i < QUALITY_STEPS_LENGTH; i++) {
            if (!doneN) {
                lumaEndN = texture.Sample(linear_sampler, posN).a;
            }

            if (!doneP) {
                lumaEndP = texture.Sample(linear_sampler, posP).a;
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

    let dstN = !isHorizontal ? posM.y - posN.y : posM.x - posN.x;
    let dstP = !isHorizontal ? posP.y - posM.y : posP.x - posM.x;

    let goodSpanN = (lumaEndN < 0.0) != lumaMLTZero;
    let spanLength = dstP + dstN;
    let goodSpanP = (lumaEndP < 0.0) != lumaMLTZero;
    let spanLengthRcp = 1.0 / spanLength;

    let directionN = dstN < dstP;
    let dst = min(dstN, dstP);
    let goodSpan = directionN ? goodSpanN : goodSpanP;
    let subpixG = subpixF * subpixF;
    let pixelOffset = (dst * (-spanLengthRcp)) + 0.5;
    let subpixH = subpixG * QUALITY_SUBPIX;

    let pixelOffsetGood = goodSpan ? pixelOffset : 0.0;
    let pixelOffsetSubpix = max(pixelOffsetGood, subpixH);

    let finalPos = float2(
        !isHorizontal ? posM.x + pixelOffsetSubpix * lengthSign : posM.x,
        isHorizontal ? posM.y + pixelOffsetSubpix * lengthSign : posM.y
    );

    let finalColor = texture.Sample(linear_sampler, finalPos);
    return float4(finalColor.rgb, 1.0);
}
