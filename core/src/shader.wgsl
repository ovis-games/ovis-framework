@group(0) @binding(0)
var<uniform> entity_index: u32;

@group(1) @binding(0)
var<storage, read> positions: array<vec2<f32>>;
@group(1) @binding(1)
var<storage, read> positions_index: array<u32>;

@group(1) @binding(4)
var<storage, read> colors: array<vec4<f32>>;
@group(1) @binding(5)
var<storage, read> colors_index: array<u32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) instance_index: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.instance_index = in_instance_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var position = positions[positions_index[in.instance_index] & 0xffffffu];

    return vec4<f32>(position, 0.1, 1.0);
}
