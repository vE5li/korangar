#language slang 2026

module globals;

public struct GlobalUniforms {
    public var view_projection: float4x4;
    public var view: float4x4;
    public var inverse_view: float4x4;
    public var inverse_projection: float4x4;
    public var inverse_view_projection: float4x4;
    public var indicator_positions: float4x4;
    public var indicator_color: float4;
    public var ambient_color: float4;
    public var camera_position: float4;
    public var forward_size: uint2;
    public var interface_size: uint2;
    public var pointer_position: uint2;
    public var animation_timer: float;
    public var point_light_count: uint;
    public var enhanced_lighting: uint;
    public var shadow_method: uint;
    public var shadow_detail: uint;
    public var use_sdsm: uint;
};
