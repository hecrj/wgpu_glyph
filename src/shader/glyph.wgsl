struct Globals {
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var font_sampler: sampler;
@group(0) @binding(2) var font_tex: texture_2d<f32>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) left_top: vec3f,
    @location(1) right_bottom: vec2f,
    @location(2) tex_left_top: vec2f,
    @location(3) tex_right_bottom: vec2f,
    @location(4) color: vec4f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) f_tex_pos: vec2f,
    @location(1) f_color: vec4f,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    var pos = vec2f(0, 0);
    let left = input.left_top.x;
    let right = input.right_bottom.x;
    let top = input.left_top.y;
    let bottom = input.right_bottom.y;

    switch input.vertex_index {
        case 0u: {
            pos = vec2(left, top);
            out.f_tex_pos = input.tex_left_top;
        }
        case 1u: {
            pos = vec2(right, top);
            out.f_tex_pos = vec2(input.tex_right_bottom.x, input.tex_left_top.y);
        }
        case 2u: {
            pos = vec2(left, bottom);
            out.f_tex_pos = vec2(input.tex_left_top.x, input.tex_right_bottom.y);
        }
        case 3u: {
            pos = vec2(right, bottom);
            out.f_tex_pos = input.tex_right_bottom;
        }
        default: {}
    }

    out.f_color = input.color;
    out.position = globals.transform * vec4(pos, input.left_top.z, 1.0);

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    var alpha = textureSample(font_tex, font_sampler, input.f_tex_pos).r;

    if (alpha <= 0.0) {
        discard;
    }

    return input.f_color * vec4f(1.0, 1.0, 1.0, alpha);
}
