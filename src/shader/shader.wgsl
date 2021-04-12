struct VertexOut {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] f_tex_pos: vec2<f32>;
    [[location(1)]] f_color: vec4<f32>;
};

[[block]]
struct Transform {
    data: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> transform: Transform;

[[stage(vertex)]]
fn vs_main(
    [[builtin(vertex_index)]] vid: u32,
    [[location(0)]] left_top: vec3<f32>,
    [[location(1)]] right_bottom: vec2<f32>,
    [[location(2)]] tex_left_top: vec2<f32>,
    [[location(3)]] tex_right_bottom: vec2<f32>,
    [[location(4)]] color: vec4<f32>,
) -> VertexOut {

    var out: VertexOut;

    const left = left_top.x;
    const right = right_bottom.x;
    const top = left_top.y;
    const bottom = right_bottom.y;

    var pos: vec2<f32>;

    if (vid == 0u32) {
        pos = vec2<f32>(left, top);
        out.f_tex_pos = tex_left_top;
    }
    elseif (vid == 1u32) {
        pos = vec2<f32>(right, top);
        out.f_tex_pos = vec2<f32>(tex_right_bottom.x, tex_left_top.y);
    }
    elseif (vid == 2u32) {
        pos = vec2<f32>(left, bottom);
        out.f_tex_pos = vec2<f32>(tex_left_top.x, tex_right_bottom.y);
    }
    elseif (vid == 3u32) {
        pos = vec2<f32>(right, bottom);
        out.f_tex_pos = tex_right_bottom;
    }

    out.f_color = color;
    out.pos = transform.data * vec4<f32>(pos.x, pos.y, left_top.z, 1.0);
    return out;

}

[[group(0), binding(1)]]
var font_sampler: sampler;
[[group(0), binding(2)]]
var font_tex: texture_2d<f32>;

[[stage(fragment)]]
fn fs_main(
    in: VertexOut
) -> [[location(0)]] vec4<f32> {
    const alpha = textureSample(font_tex, font_sampler, in.f_tex_pos).r;
    if (alpha <= 0.0) {
        discard;
    }

    return in.f_color * vec4<f32>(1.0, 1.0, 1.0, alpha);
}