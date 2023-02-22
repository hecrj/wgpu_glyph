struct Globals {
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var font_sampler: sampler;
@group(0) @binding(2) var font_tex: texture_2d<f32>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) left_top: vec3<f32>,
    @location(1) right_bottom: vec2<f32>,
    @location(2) tex_left_top: vec2<f32>,
    @location(3) tex_right_bottom: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) outline_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) f_tex_pos: vec2<f32>,
    @location(1) f_color: vec4<f32>,
    @location(2) f_outline_color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    var pos: vec2<f32> = vec2<f32>(0.0, 0.0);
    var left: f32 = input.left_top.x;
    var right: f32 = input.right_bottom.x;
    var top: f32 = input.left_top.y;
    var bottom: f32 = input.right_bottom.y;

    switch (i32(input.vertex_index)) {
        case 0: {
            pos = vec2<f32>(left, top);
            out.f_tex_pos = input.tex_left_top;
        }
        case 1: {
            pos = vec2<f32>(right, top);
            out.f_tex_pos = vec2<f32>(input.tex_right_bottom.x, input.tex_left_top.y);
        }
        case 2: {
            pos = vec2<f32>(left, bottom);
            out.f_tex_pos = vec2<f32>(input.tex_left_top.x, input.tex_right_bottom.y);
        }
        case 3: {
            pos = vec2<f32>(right, bottom);
            out.f_tex_pos = input.tex_right_bottom;
        }
        default: {}
    }

    out.f_color = input.color;
    out.f_outline_color = input.outline_color;
    out.position = globals.transform * vec4<f32>(pos, input.left_top.z, 1.0);

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var pixel_size: vec2<f32> = (1.0 / vec2<f32>(textureDimensions(font_tex)));

    var alpha: f32 = textureSample(font_tex, font_sampler, input.f_tex_pos + 0.5*pixel_size).r;

    var border = false;
    for(var i = -1; i <= 1 && !border; i += 1) {
        for(var j = -1; j <= 1 && !border; j += 1) {
            if i == 0 && j == 0 {
                continue;
            }
            if textureSample(font_tex, font_sampler, input.f_tex_pos + 0.5*pixel_size + pixel_size*vec2<f32>(f32(i), f32(j))).r <= 0.0 {
                border = true;
            }
        }
    }

    if (alpha <= 0.0) {
        discard;
    }

    if border {
        return input.f_outline_color * vec4<f32>(1.0, 1.0, 1.0, alpha);
    } else {
        return input.f_color * vec4<f32>(1.0, 1.0, 1.0, alpha);
    }
}
