#language slang 2026

module picker;

public struct EntityInstanceData {
    public var world: float4x4;
    public var frame_part_transform: float4x4;
    public var texture_position: float2;
    public var texture_size: float2;
    public var texture_index: int;
    public var mirror: uint;
    public var identifier_high: uint;
    public var identifier_low: uint;
}

public struct EntityVertex {
    public var position: float3;
    public var texture_coordinates: float2;
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v
// 0            -1  2  0  0  0
// 1            -1  0  0  0  1
// 2             1  2  0  1  0
// 3             1  2  0  1  0
// 4            -1  0  0  0  1
// 5             1  0  0  1  1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
[ForceInline]
public func entity_vertex_data(vertex_index: uint) -> EntityVertex {
    let index = 1u << vertex_index;

    let case0 = int((index & 0x13u) != 0u);
    let case1 = int((index & 0x0Du) != 0u);

    let x = float(1 - 2 * case0);
    let y = float(2 * case1);
    let z = 0.0;
    let u = float(1 - case0);
    let v = float(1 - case1);

    var vertex: EntityVertex;
    vertex.position = float3(x, y, z);
    vertex.texture_coordinates = float2(u, v);
    return vertex;
}
