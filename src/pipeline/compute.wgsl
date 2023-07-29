
// ======================== Structs =======================

struct Globals {
    time: f32,
}

// ========================= Main =========================

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(0) @binding(2)
var tex: texture_storage_2d<rgba32float, read_write>;

@compute
@workgroup_size(1, 1, 1)
fn main(
    @builtin(global_invocation_id) g_invocation_id: vec3<u32>
) {
    let dimensions = textureDimensions(tex);
    let aspect_ratio = vec2<f32>(f32(dimensions.x) / f32(dimensions.y), 1.0);
    
    let uv = vec2<f32>(
        f32(g_invocation_id.x) / f32(dimensions.x),
        f32(g_invocation_id.y) / f32(dimensions.y)
    );

    textureStore(tex, g_invocation_id.xy, vec4<f32>(uv, 1.0, 1.0));
}
