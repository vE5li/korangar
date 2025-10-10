#language slang 2026

module point_shadow;

public static const var SPRITE_MAX_SIZE_X: float = 400.0;
public static const var SPRITE_MAX_SIZE_Y: float = 400.0;

public struct PassUniforms {
    public var view_projection: float4x4;
    public var view: float4x4;
    public var inverse_view: float4x4;
    public var light_position: float4;
    public var animation_timer: float;
};

public struct EntityInstanceData {
    public var world: float4x4;
    public var frame_part_transform: float4x4;
    public var texture_position: float2;
    public var texture_size: float2;
    public var frame_size: float2;
    public var extra_depth_offset: float;
    public var depth_offset: float;
    public var curvature: float;
    public var mirror: uint;
    public var texture_index: int;
    public var alpha: float;
};

public struct ModelInstanceData {
    public var world: float4x4;
};

public struct ModelFragmentOutput {
    public float depth: SV_Depth;
};

public struct EntityVertexInput {
    public uint vertex_index : SV_VulkanVertexID;
    public uint instance_index : SV_VulkanInstanceID;
};

public struct EntityVertex {
    public var position: float3;
    public var texture_coordinates: float2;
};

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v
// 0            -1  2  1  0  0
// 1            -1  0  1  0  1
// 2             1  2  1  1  0
// 3             1  2  1  1  0
// 4            -1  0  1  0  1
// 5             1  0  1  1  1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
[ForceInline]
public func entity_vertex_data(vertex_index: uint) -> EntityVertex {
    let index = 1 << vertex_index;

    let case0 = int((index & 0x13) != 0);
    let case1 = int((index & 0x0D) != 0);

    let x = float(1 - 2 * case0);
    let y = float(2 * case1);
    let z = 1.0;
    let u = float(1 - case0);
    let v = float(1 - case1);

    var vertex: EntityVertex;
    vertex.position = float3(x, y, z);
    vertex.texture_coordinates = float2(u, v);
    return vertex;
}
