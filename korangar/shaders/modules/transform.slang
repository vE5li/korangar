#language slang 2026

module transform;

public static const var NEAR_PLANE: float = 0.1;
public static const var DEPTH_EPSILON: float = 1.0e-7;

public namespace coordinate_space {
    [ForceInline]
    public func screen_to_clip_space(screen_coords: float2) -> float2 {
        let x = (screen_coords.x * 2.0) - 1.0;
        let y = -(screen_coords.y * 2.0) + 1.0;
        return float2(x, y);
    }

    [ForceInline]
    public func clip_to_screen_space(ndc: float2) -> float2 {
        let u = (ndc.x + 1.0) / 2.0;
        let v = (1.0 - ndc.y) / 2.0;
        return float2(u, v);
    }
}

public namespace depth {
    [ForceInline]
    public func nonLinearToLinear(non_linear_depth: float) -> float {
        return NEAR_PLANE / (non_linear_depth + DEPTH_EPSILON);
    }

    [ForceInline]
    public func linearToNonLinear(linear_depth: float) -> float {
        return NEAR_PLANE / (linear_depth + DEPTH_EPSILON);
    }
}

public namespace color {
    [ForceInline]
    public func linear_to_srgb(color: float3) -> float3 {
        return float3(
            linear_to_srgb_component(color.r),
            linear_to_srgb_component(color.g),
            linear_to_srgb_component(color.b)
        );
    }

    [ForceInline]
    func linear_to_srgb_component(value: float) -> float {
        let is_linear = value < 0.0031308;
        let linear_result = value * 12.92;
        let power_result = 1.055 * pow(abs(value), 1.0 / 2.4) - 0.055;
        return is_linear ? linear_result : power_result;
    }
}
