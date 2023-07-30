
// ======================== Structs =======================

struct Globals {
    dt: f32,
}

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
}

struct AgentBuffer {
    count: u32,
    agents: array<Agent>,
}

// ========================= Main =========================

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(0) @binding(1)
var<storage, read_write> agents_buffer: AgentBuffer;

@group(0) @binding(2)
var tex: texture_storage_2d<rgba32float, read_write>;

@compute
@workgroup_size(1, 1, 1)
fn main(
    @builtin(global_invocation_id) g_invocation_id: vec3<u32>
) {
    let dimensions = textureDimensions(tex);

    let agent = &agents_buffer.agents[g_invocation_id.x];
    let position = (*agent).position;
    let velocity = (*agent).velocity;

    let pixel_position = vec2<u32>(
        u32(position.x),
        u32(position.y)
    );

    textureStore(tex, pixel_position, vec4<f32>(1.0, 0.0, 0.0, 1.0));

    // Update agent position
    let new_position = position + velocity * globals.dt * 120.0;

    // Reverse velocity if agent hits a wall
    if new_position.x <= 0.0 || new_position.x >= f32(dimensions.x) {
        (*agent).velocity.x = -velocity.x;
    }

    if new_position.y <= 0.0 || new_position.y >= f32(dimensions.y) {
        (*agent).velocity.y = -velocity.y;
    }

    // Store new agent position
    (*agent).position = vec2<f32>(
        clamp(new_position.x, 0.0, f32(dimensions.x)),
        clamp(new_position.y, 0.0, f32(dimensions.y))
    );
}
