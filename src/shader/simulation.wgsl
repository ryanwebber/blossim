
// ======================== Structs =======================

struct Globals {
    dt: f32,
    time: f32,
    work_group_size: u32,
}

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
}

struct AgentBuffer {
    count: u32,
    agents: array<Agent>,
}

// ========================= Utils ========================

fn debug_point(p: vec2<f32>) {
    let dimensions = textureDimensions(tex);
    let ppos = vec2<u32>(
        u32(clamp(p.x, 0.0, f32(dimensions.x))),
        u32(clamp(p.y, 0.0, f32(dimensions.y))),
    );

    // textureStore(tex, ppos, vec4<f32>(0.0, 1.0, 0.0, 1.0));
}

fn sample(p: vec2<f32>) -> f32 {
    let dimensions = textureDimensions(tex);
    let x = p.x;
    let y = p.y;
    let uv = vec2<u32>(
        clamp(0u, dimensions.x, u32(x)),
        clamp(0u, dimensions.y, u32(y))
    );

    return textureLoad(tex, uv).w;
}

fn sample_area(p: vec2<f32>, radius: f32) -> f32 {
    let dimensions = textureDimensions(tex);
    let x = p.x;
    let y = p.y;
    let uv = vec2<i32>(
        clamp(0, i32(dimensions.x), i32(x)),
        clamp(0, i32(dimensions.y), i32(y))
    );

    let samples = i32(radius);
    var sum = 0.0;
    var num_samples = 0u;
    for (var i = -samples; i <= samples; i = i + 1) {
        for (var j = -samples; j <= samples; j = j + 1) {
            let sample_uv = vec2<u32>(
                u32(clamp(0, i32(dimensions.x), uv.x + i)),
                u32(clamp(0, i32(dimensions.y), uv.y + j))
            );

            if i * i + j * j > samples * samples {
                continue;
            }

            sum += sample(vec2<f32>(f32(sample_uv.x), f32(sample_uv.y)));
            num_samples = num_samples + 1u;
        }
    }

    if num_samples == 0u {
        return sample(p);
    }

    return sum / f32(num_samples);
}

fn rotate(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
}

// ======================== Update ========================

fn update(agent_idx: u32) {

    let dimensions = textureDimensions(tex);

    let agent = &agents_buffer.agents[agent_idx];
    var position = (*agent).position;
    var velocity = (*agent).velocity;

    let angle = 0.8;

    let left = sample_area(position + rotate(velocity, angle), 1.0);
    let right = sample_area(position + rotate(velocity, -angle), 1.0);
    let forward = sample_area(position + velocity, 1.0);

    if forward >= left && forward >= right {
        // Do nothing
    } else if left > right {
        velocity = rotate(velocity, 0.05);
    } else {
        velocity = rotate(velocity, -0.05);
    }

    // Update agent position
    position += velocity * globals.dt * 4.0;

    // Reverse velocity if agent hits a wall
    if position.x <= 0.0 || position.x >= f32(dimensions.x) {
        velocity.x = -velocity.x;
    }

    if position.y <= 0.0 || position.y >= f32(dimensions.y) {
        velocity.y = -velocity.y;
    }

    // Store new agent position and velocity
    (*agent).velocity = velocity;
    (*agent).position = vec2<f32>(
        clamp(position.x, 0.0, f32(dimensions.x)),
        clamp(position.y, 0.0, f32(dimensions.y))
    );
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
    let agent_idx = g_invocation_id.x + g_invocation_id.y * globals.work_group_size;
    let agent = &agents_buffer.agents[agent_idx];

    let pixel_position = vec2<u32>(
        u32((*agent).position.x),
        u32((*agent).position.y)
    );

    // Update the agent
    update(agent_idx);

    // Write the agent to the texture
    textureStore(tex, pixel_position, vec4<f32>(1.0, 0.0, 0.0, 1.0));
}
