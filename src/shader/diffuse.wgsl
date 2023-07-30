
// ======================== Structs =======================

struct Globals {
    dt: f32,
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

    var color = textureLoad(tex, g_invocation_id.xy).xyz;

    // Diffuse by averaging nearby pixels

    var diffuse = vec3<f32>(0.0);
    for (var i = -1; i <= 1; i = i + 1) {
        for (var j = -1; j <= 1; j = j + 1) {
            var sample = vec2<i32>(g_invocation_id.xy) + vec2<i32>(i, j);
            if (sample.x < 0 || sample.x >= i32(dimensions.x) || sample.y < 0 || sample.y >= i32(dimensions.y)) {
                continue;
            }

            diffuse += textureLoad(tex, sample).xyz;
        }
    }

    diffuse /= 9.0;

    color = mix(color, diffuse, globals.dt * 12.0);

    // Apply dimming

    if globals.dt > 0.0 {
        color = max(vec3<f32>(0.0), color - globals.dt * 0.1);
    }

    textureStore(tex, g_invocation_id.xy, vec4<f32>(color, 1.0));
}
