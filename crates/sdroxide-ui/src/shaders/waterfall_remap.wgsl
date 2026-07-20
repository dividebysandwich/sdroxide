// Remaps the waterfall history texture to a new frequency axis when the
// viewport geometry (center/span) changes, so zoom/retune seamlessly
// continues the existing data instead of clearing it to black.
// Fragment-only, WebGL2-downlevel-safe.

struct Remap {
    // Destination column u_dst maps to source column: u_src = u_dst*scale + offset.
    scale: f32,
    offset: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> rm: Remap;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let u_src = in.uv.x * rm.scale + rm.offset;
    // Columns with no source data (outside the old span) go dark.
    if (u_src < 0.0 || u_src > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    // The row (v) axis is preserved identically, so the ring scroll is intact.
    let s = textureSampleLevel(src_tex, src_samp, vec2<f32>(u_src, in.uv.y), 0.0).r;
    return vec4<f32>(s, 0.0, 0.0, 1.0);
}
