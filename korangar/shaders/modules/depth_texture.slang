#language slang 2026

module depth_texture;

__generic<int sampleCount = 0, int format = 0>
public typealias DepthTexture2D = _Texture<
    float,
    __Shape2D,
    0, // isArray
    0, // isMS
    sampleCount,
    0, // access
    1, // isShadow
    0, // isCombined
    format
>;

__generic<int sampleCount = 0, int format = 0>
public typealias DepthTexture2DMS = _Texture<
    float,
    __Shape2D,
    0, // isArray
    1, // isMS
    sampleCount,
    0, // access
    1, // isShadow
    0, // isCombined
    format
>;

__generic<int sampleCount = 0, int format = 0>
public typealias DepthTexture2DArray = _Texture<
    float,
    __Shape2D,
    1, // isArray
    0, // isMS
    sampleCount,
    0, // access
    1, // isShadow
    0, // isCombined
    format
>;

__generic<int sampleCount = 0, int format = 0>
public typealias DepthTextureCubeArray = _Texture<
    float,
    __ShapeCube,
    1, // isArray
    0, // isMS
    sampleCount,
    0, // access
    1, // isShadow
    0, // isCombined
    format
>;
