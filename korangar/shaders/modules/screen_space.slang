#language slang 2026

module screen_space;

public struct FullscreenVertex {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var uv: float2;

    [ForceInline]
    public static func new(vertex_index: uint) -> FullscreenVertex {
        let uv = float2(float((vertex_index << 1) & 2), float(vertex_index & 2));
        let position = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);

        var vertex: FullscreenVertex;
        vertex.position = position;
        vertex.uv = uv;
        return vertex;
    }
};
