// Waterfall: scrolling history ring-texture sampled through a colormap LUT.
// WebGL2-safe: fragment-only, sampled textures + one uniform buffer.

struct Uniforms {
    // v offset of the newest written row (write_row / tex_h)
    scroll: f32,
    // visible rows / texture rows
    vscale: f32,
    // viewport in texture-u coordinates
    u_lo: f32,
    u_hi: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var hist_tex: texture_2d<f32>;
@group(0) @binding(2) var hist_samp: sampler;
@group(0) @binding(3) var lut_tex: texture_2d<f32>;
@group(0) @binding(4) var lut_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle covering the callback viewport.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // uv: (0,0) top-left, (1,1) bottom-right of the widget rect
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let u_tex = mix(u.u_lo, u.u_hi, in.uv.x);
    // Newest row at the top; older rows below.
    let v_tex = fract(u.scroll - in.uv.y * u.vscale);
    let intensity = textureSampleLevel(hist_tex, hist_samp, vec2<f32>(u_tex, v_tex), 0.0).r;
    return textureSampleLevel(lut_tex, lut_samp, vec2<f32>(intensity, 0.5), 0.0);
}
