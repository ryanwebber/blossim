
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

fn warp_clamp(point: vec2<f32>) -> vec2<f32> {
    var p = point;
    let dimensions = textureDimensions(tex);
    p.x = select(p.x, 0.0, p.x >= f32(dimensions.x));
    p.x = select(p.x, f32(dimensions.x), p.x < 0.0);
    p.y = select(p.y, 0.0, p.y >= f32(dimensions.y));
    p.y = select(p.y, f32(dimensions.y), p.y < 0.0);
    return p;
}

fn sample(p: vec2<f32>) -> f32 {
    var p2 = warp_clamp(p);
    var uv = vec2<u32>(u32(p2.x), u32(p2.y));
    return textureLoad(tex, uv).w;
}

fn sample_area(p: vec2<f32>, radius: f32) -> f32 {
    let samples = i32(radius);
    var sum = 0.0;
    var num_samples = 0u;
    for (var i = -samples; i <= samples; i = i + 1) {
        for (var j = -samples; j <= samples; j = j + 1) {
            let offset = vec2<f32>(f32(i), f32(j));
            let sample_uv = p + offset;
            if i * i + j * j > samples * samples {
                continue;
            }

            sum += sample(sample_uv);
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

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let k = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, saturate(p - k.xxx), c.y);
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
        velocity = rotate(velocity, 0.01);
    } else {
        velocity = rotate(velocity, -0.01);
    }

    // Update agent position
    position += velocity * globals.dt * 4.0;
    position = warp_clamp(position);

    // Store new agent position and velocity
    (*agent).velocity = velocity;
    (*agent).position = position;
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
    let num_agents = globals.work_group_size * globals.work_group_size;
    let agent_idx = g_invocation_id.x + g_invocation_id.y * globals.work_group_size;

    // Update the agent
    update(agent_idx);

    let agent = &agents_buffer.agents[agent_idx];

    let pixel_position = vec2<u32>(
        u32((*agent).position.x),
        u32((*agent).position.y)
    );

    let hue = 0.2 * f32(agent_idx) / f32(num_agents) + 0.3;
    let hsv = vec3<f32>(hue, 1.0, 0.8);
    let color = hsv2rgb(hsv);

    // Write the agent to the texture
    textureStore(tex, pixel_position, vec4<f32>(color, 1.0));
}
