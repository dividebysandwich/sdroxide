//! GPU waterfall: a ring-buffer history texture scrolled/panned/zoomed in the
//! fragment shader, colorized through a 256×1 LUT.
//!
//! WebGL2-downlevel-safe by design: no compute, no storage buffers, only
//! sampled R8Unorm/RGBA8 textures and one uniform buffer.
//!
//! The renderer's `CallbackResources` type-map holds exactly one value per
//! type, shared by every viewport — so the resources here are a *registry*:
//! pipelines, layouts and samplers built once, plus one set of history
//! textures per radio, keyed by [`WaterfallCallback::wf_id`]. Two radio tabs
//! writing one shared history would corrupt each other's scrollback and fight
//! over the frequency axis on every switch.

use std::collections::HashMap;

use eframe::egui_wgpu::{CallbackResources, CallbackTrait, RenderState, ScreenDescriptor, wgpu};
use sdroxide_types::SpectrumFrame;

use crate::colormap;

/// History texture width where nothing has said otherwise: what every build
/// before the width became the operator's to choose emitted and drew, and the
/// floor every tier is measured up from.
///
/// The matching number at the other end of the wire is
/// `sdroxide_radio::engine::DISPLAY_BINS`.
pub const DEFAULT_TEX_W: u32 = 2048;

/// The widest history this client will ever ask for, whatever the screen or the
/// GPU would allow.
///
/// 8192 columns is 32 MB of texture per radio tab and half a megabyte a second
/// on the wire to a remote station — past there the picture stops improving
/// (no display is that wide) and only the bill grows. Must stay equal to
/// `sdroxide_radio::engine::MAX_DISPLAY_BINS`, which is the same ceiling seen
/// from the engine's side; the two cannot share a constant because a radio
/// engine must not depend on a GUI.
pub const MAX_TEX_W: u32 = 8192;

/// History rows (scrollback depth). Not tiered with the width: rows are time,
/// and 2048 of them is 36 s at the medium scroll rate on every screen.
pub const TEX_H: u32 = 2048;

/// Padded to 32 bytes: a uniform-address-space struct is rounded up to a
/// multiple of 16, so the buffer has to be that big even though five floats
/// are used.
#[repr(C)]
#[derive(Clone, Copy)]
struct Uniforms {
    scroll: f32,
    vscale: f32,
    u_lo: f32,
    u_hi: f32,
    flip: f32,
    _pad: [f32; 3],
}

/// What a history already drawn on one frequency axis has to do to hold a
/// frame drawn on another.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HistoryMove {
    /// Slide along the axis by this many whole texture columns, positive
    /// towards higher frequencies. Whole because a fractional slide has to
    /// interpolate, and this runs on *every frame* of a pan that carries the
    /// window (issue #133): rounding it to a column keeps the move exact, so
    /// the history stays as sharp through a drag as it was before it.
    Shift(f64),
    /// The span changed, so nothing lines up column for column and the whole
    /// picture has to be resampled. A zoom, and rare enough to pay for.
    Rescale,
}

/// How a `cols`-wide history on the `have` axis has to move to hold a frame on
/// the `want` axis, both `(centre, span)` in Hz. `None` when it does not have
/// to move at all.
///
/// The interesting answer is that `None`. A move of less than half a column is
/// a picture nobody can tell from the one already there, and before this every
/// one of them resampled the entire history — including the fractions of a
/// hertz a satellite's Doppler correction walks through, which blurred the
/// waterfall to a wash for no visible motion at all (issue #177).
fn history_move(have: (f64, f64), want: (f64, f64), cols: f64) -> Option<HistoryMove> {
    let ((have_center, have_span), (center, span)) = (have, want);
    if span <= 0.0 {
        return None;
    }
    if (span - have_span).abs() > span * 1e-6 {
        return Some(HistoryMove::Rescale);
    }
    match ((center - have_center) / span * cols).round() {
        0.0 => None,
        by => Some(HistoryMove::Shift(by)),
    }
}

/// What every radio's waterfall shares: compiled pipelines, bind-group
/// layouts and samplers. Built once in [`init`].
struct Shared {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    remap_pipeline: wgpu::RenderPipeline,
    remap_layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    /// The same, unfiltered — what the waterfall is drawn through when the
    /// operator has turned smoothing off. See [`UiSettings::waterfall_smooth`].
    ///
    /// [`UiSettings::waterfall_smooth`]: sdroxide_types::UiSettings::waterfall_smooth
    nearest: wgpu::Sampler,
    lut_sampler: wgpu::Sampler,
    remap_sampler: wgpu::Sampler,
    /// The same, unfiltered — for the case a moving window makes the common
    /// one: a shift along the frequency axis with the span unchanged. Snapped
    /// to whole columns (see [`WaterfallCallback::prepare`]) it lands on texel
    /// centres, so nearest returns each column exactly as it was and the
    /// history survives being shifted sixty times a second. Interpolating
    /// instead spreads every column a little further into its neighbours each
    /// time, and a few seconds of dragging blurs the picture into a wash.
    remap_nearest: wgpu::Sampler,
}

/// How many rows this repaint appends.
///
/// `carried` is what the frame brought and has not been written yet;
/// `wall_clock` is what the app's own elapsed-time accumulator would scroll.
///
/// The rule that matters is the first one. The fallback is for a lane that does
/// not clock rows *at all* — a radio's own sweep, a transmit monitor — and not
/// for a frame that merely happens to carry none. Below the frame rate most
/// frames carry none: at five rows a second and sixty frames, fifty-five in
/// every sixty are empty, and scrolling those on the wall clock as well ran the
/// waterfall at about twice the rate its own time labels are spaced at.
fn rows_to_append(clocked: bool, carried: usize, wall_clock: u32) -> u32 {
    if clocked { carried as u32 } else { wall_clock.min(MAX_FALLBACK_ROWS) }
}

/// The most rows one repaint may append, whichever way they were clocked.
///
/// The size of the staging buffer, so it has to bound the engine's own batch
/// cap (`MAX_BATCH_ROWS`, 64) as well as the wall-clock fallback below — and a
/// frame off the network is a struct this callback did not build, so the count
/// is clamped rather than trusted.
const MAX_APPEND_ROWS: u32 = 64;

/// The most rows one repaint may scroll on the wall clock.
///
/// Only the fallback needs it: a hitch or a tab-away would otherwise dump a
/// whole backlog of repeated rows into the texture at once. A clocking lane is
/// already bounded by the engine's own batch cap.
const MAX_FALLBACK_ROWS: u32 = 32;

/// What a machine may be asked to draw, cut down to the facts that decide it.
///
/// Plain numbers rather than the wgpu handles they came from, for the reason
/// `AdapterSummary` in the crate root exists: the decision has to be testable
/// against a table of real machines with no GPU in the room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayClass {
    /// `device.limits().max_texture_dimension_2d` — a hard ceiling, not a hint.
    pub max_texture_dim: u32,
    /// What the adapter calls itself. `Cpu` is llvmpipe, lavapipe, SwiftShader,
    /// WARP: a renderer where every pixel of the panadapter is the CPU's work.
    pub device_type: wgpu::DeviceType,
    /// The backend actually in use. `Gl` is this tree's compatibility lane —
    /// where a Raspberry Pi is deliberately put ([`crate::wgpu_options`]),
    /// where a 2013 Intel lands (issue #148), and what a WebGL2 browser gives.
    pub backend: wgpu::Backend,
    /// The engine is on the other end of a WebSocket, so every column is bytes
    /// on a link whose width nothing here can measure.
    pub remote: bool,
    /// `available_parallelism()`, or 1 where it cannot be asked.
    pub cores: u32,
}

/// The widest history [`auto_display_bins`] will pick on its own, whatever the
/// screen and the GPU would allow.
///
/// 4096 columns is one per pixel of a 4K panadapter, which is the point of the
/// exercise; [`MAX_TEX_W`] above it is two per pixel, and that is oversampling
/// nobody asked for by leaving a setting alone. It is on the menu for anyone
/// who wants it.
const AUTO_MAX_TEX_W: u32 = 4096;

/// The frame width to ask for on `class`, drawing a panadapter `panadapter_px`
/// *device pixels* wide.
///
/// The ask is one column per pixel — below that the shader is stretching a
/// texture with a linear sampler, which interpolates and cannot invent detail,
/// and the result is the coarse gravel of issue #172 on a 4K panel. Everything
/// else here is a reason to want less than that.
///
/// Every rule below is a *cap*, so the answer is the smallest of them, and the
/// floor is [`DEFAULT_TEX_W`] — what sdroxide has always drawn. No machine can
/// come out of this worse off than it went in.
pub fn auto_display_bins(class: DisplayClass, panadapter_px: u32) -> u32 {
    // What the screen is actually asking for. Rounded up, so a 3840-pixel
    // panadapter gets 4096 and a 1920-pixel one is not made to pay for columns
    // it cannot show. Zero means the window has not been laid out yet.
    let want = if panadapter_px == 0 { DEFAULT_TEX_W } else { panadapter_px.next_power_of_two() };

    let mut cap = AUTO_MAX_TEX_W.min(class.max_texture_dim);

    // A software rasteriser fragment-shades the panadapter on the CPU that is
    // already running the DSP. Doubling its texture working set is the wrong
    // direction on the one machine with nothing spare.
    if class.device_type == wgpu::DeviceType::Cpu {
        cap = cap.min(DEFAULT_TEX_W);
    }

    // In this tree the GL backend *is* the compatibility lane: a Raspberry Pi
    // is put there on purpose because V3DV flickers, an old Intel falls back
    // there, and a WebGL2 browser has nowhere else to be. The reported texture
    // limit does not catch any of them — a WebGL2 context commonly advertises
    // 8192 or 16384 and would sail straight past the cap above — so the backend
    // has to be its own rule.
    if class.backend == wgpu::Backend::Gl {
        cap = cap.min(DEFAULT_TEX_W);
    }

    // A browser tab is not a process: it renders inside a memory budget it does
    // not own and shares a GPU with every other tab.
    if cfg!(target_arch = "wasm32") {
        cap = cap.min(DEFAULT_TEX_W);
    }

    // The one input that cannot be measured is the link. A frame is a byte a
    // column, so 4096 columns at 60 fps is a quarter of a megabyte a second,
    // and Auto must not put that on somebody's tether without being asked. The
    // setting is still there for a client on a LAN.
    if class.remote {
        cap = cap.min(DEFAULT_TEX_W);
    }

    // Not the FFT — that is a few percent of one core — but everything a frame
    // touches downstream of it: the smoothing, the peak hold, the trace, and
    // the fit's sort over every visible column, all per frame per radio.
    if class.cores < 4 {
        cap = cap.min(DEFAULT_TEX_W);
    }

    // Shared memory and a shared power budget. Room for a 4K panel, not for
    // oversampling it.
    if class.device_type == wgpu::DeviceType::IntegratedGpu {
        cap = cap.min(AUTO_MAX_TEX_W);
    }

    want.clamp(DEFAULT_TEX_W, cap.max(DEFAULT_TEX_W))
}

/// The widest the operator may *choose* on `class`, which is what the settings
/// combo greys its rows against.
///
/// Deliberately more permissive than [`auto_display_bins`]: the caps there for
/// a link nobody can measure or a core count that is only a proxy are Auto
/// being careful on someone's behalf, and an operator who knows their own
/// machine is entitled to overrule that. What cannot be overruled is the GPU's
/// own texture limit, which is not a policy.
pub fn manual_ceiling(class: DisplayClass) -> u32 {
    let mut cap = MAX_TEX_W.min(class.max_texture_dim);
    // The two that are hard facts about the renderer rather than caution.
    if class.device_type == wgpu::DeviceType::Cpu || class.backend == wgpu::Backend::Gl {
        cap = cap.min(DEFAULT_TEX_W);
    }
    if cfg!(target_arch = "wasm32") {
        cap = cap.min(DEFAULT_TEX_W);
    }
    cap.max(DEFAULT_TEX_W)
}

/// One radio's waterfall state: its history textures, uniforms and LUT.
/// 8 MB of texture per radio at [`DEFAULT_TEX_W`], and proportionally more on a
/// wide display — retired when its tab closes ([`retire`]).
struct WaterfallResources {
    /// Columns these textures were built with. The callback carries the width
    /// the client wants each frame; when the two part company the set is
    /// rebuilt (see [`WaterfallCallback::prepare`]).
    tex_w: u32,
    /// One render bind group per history texture; index by `active`.
    bind_group: [wgpu::BindGroup; 2],
    /// The same pair bound to the unfiltered sampler — see [`Shared::nearest`].
    bind_group_nearest: [wgpu::BindGroup; 2],
    /// `seq` of the last frame whose rows were appended.
    ///
    /// The same `Arc<SpectrumFrame>` is handed to every repaint until a new one
    /// arrives, and repaints outnumber frames — so without this the rows of one
    /// frame would be written again on every redraw and the waterfall would run
    /// at the frame rate times however fast the compositor felt like going.
    last_rows_seq: Option<u32>,
    /// The running average of the rows in linear power, one per texture
    /// column, while the SMOOTH chip is on; empty otherwise.
    avg_row: RowAverage,
    /// Staging for the rows appended each frame, copied into the history from
    /// inside the encoder rather than written straight to the texture.
    ///
    /// Ordering is the whole reason. Queue writes are applied *before* any of
    /// the encoder's passes in the same submit, so a row written directly
    /// would land underneath the remap pass below and be rewritten away —
    /// which is why appending used to be skipped whenever the geometry moved.
    /// A copy recorded on the encoder runs in the order it was recorded, so
    /// the rows survive a remap and the waterfall keeps scrolling through a
    /// pan that moves the window (issue #177).
    row_buf: wgpu::Buffer,
    /// Ping-pong history textures. `active` is the live one; the other is the
    /// scratch target for the frequency-remap pass on a geometry change.
    hist: [wgpu::Texture; 2],
    hist_view: [wgpu::TextureView; 2],
    active: usize,
    // Remap pass: rewrites the history to a new frequency axis instead of
    // clearing it, so zoom/retune keeps the existing waterfall on screen.
    remap_uniforms: wgpu::Buffer,
    remap_bg: [wgpu::BindGroup; 2],
    /// The same pair bound to the unfiltered sampler, for a shift along the
    /// frequency axis — see [`Shared::remap_nearest`].
    shift_bg: [wgpu::BindGroup; 2],
    lut_tex: wgpu::Texture,
    uniforms: wgpu::Buffer,
    write_row: u32,
    current_lut: Option<usize>,
    last_center: f64,
    last_span: f64,
}

/// The one value registered in `CallbackResources`.
pub struct WaterfallRegistry {
    shared: Shared,
    per: HashMap<u64, WaterfallResources>,
}

/// Compile the pipelines and register the (empty) registry in the renderer's
/// callback resources. Call once at app construction; calling again is a
/// no-op, so a radio tab created at runtime costs nothing here.
pub fn init(rs: &RenderState) {
    {
        let renderer = rs.renderer.read();
        if renderer.callback_resources.get::<WaterfallRegistry>().is_some() {
            return;
        }
    }
    let device = &rs.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("waterfall"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/waterfall.wgsl").into()),
    });

    let linear = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    // Both axes: a column is one FFT bin and a row is one transform, so
    // interpolating either invents detail that was never measured.
    let nearest = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-sampler-nearest"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-lut-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("waterfall-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("waterfall-pl"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("waterfall-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(rs.target_format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // --- Remap pipeline (frequency-axis rewrite on geometry change) --------
    let remap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("waterfall-remap"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/waterfall_remap.wgsl").into()),
    });
    let remap_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("waterfall-remap-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    // Sampling the source for remap: clamp both axes (identity v, transformed u).
    let remap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-remap-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let remap_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-remap-nearest"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let remap_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("waterfall-remap-pl"),
        bind_group_layouts: &[Some(&remap_layout)],
        immediate_size: 0,
    });
    let remap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("waterfall-remap-pipeline"),
        layout: Some(&remap_pl),
        vertex: wgpu::VertexState {
            module: &remap_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &remap_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::TextureFormat::R8Unorm.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    rs.renderer.write().callback_resources.insert(WaterfallRegistry {
        shared: Shared {
            pipeline,
            layout,
            remap_pipeline,
            remap_layout,
            linear,
            nearest,
            lut_sampler,
            remap_sampler,
            remap_nearest,
        },
        per: HashMap::new(),
    });
}

/// Drop one radio's textures when its tab closes; 8 MB apiece — four times
/// that on the widest tier — otherwise
/// leaks for the life of the process. `None` render state (no wgpu) is a no-op.
/// Wanted in the browser as much as in the shack: a browser tab holding a
/// station's radios closes them one at a time too.
pub fn retire(rs: Option<&RenderState>, wf_id: u64) {
    if let Some(rs) = rs
        && let Some(reg) = rs.renderer.write().callback_resources.get_mut::<WaterfallRegistry>()
    {
        reg.per.remove(&wf_id);
    }
}

impl WaterfallResources {
    /// One radio's textures, buffers and bind groups, on the shared layouts,
    /// `tex_w` columns wide.
    fn new(device: &wgpu::Device, shared: &Shared, tex_w: u32) -> Self {
        // Two history textures for ping-pong remapping. RENDER_ATTACHMENT lets
        // the remap pass render one into the other; R8Unorm is
        // color-renderable on WebGL2, so this stays downlevel-safe.
        let make_hist = |_i: usize| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("waterfall-history"),
                size: wgpu::Extent3d { width: tex_w, height: TEX_H, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        let hist = [make_hist(0), make_hist(1)];
        let hist_view =
            [hist[0].create_view(&Default::default()), hist[1].create_view(&Default::default())];
        let lut_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("waterfall-lut"),
            size: wgpu::Extent3d { width: 256, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waterfall-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let lut_view = lut_tex.create_view(&Default::default());
        let make_bg = |i: usize, hist_samp: &wgpu::Sampler| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("waterfall-bg"),
                layout: &shared.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&hist_view[i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(hist_samp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&lut_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&shared.lut_sampler),
                    },
                ],
            })
        };
        let bind_group = [make_bg(0, &shared.linear), make_bg(1, &shared.linear)];
        let bind_group_nearest = [make_bg(0, &shared.nearest), make_bg(1, &shared.nearest)];

        let row_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waterfall-rows"),
            size: u64::from(tex_w) * u64::from(MAX_APPEND_ROWS),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let remap_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waterfall-remap-uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let make_remap_bg = |i: usize, sampler: &wgpu::Sampler| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("waterfall-remap-bg"),
                layout: &shared.remap_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: remap_uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&hist_view[i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        let remap_bg =
            [make_remap_bg(0, &shared.remap_sampler), make_remap_bg(1, &shared.remap_sampler)];
        let shift_bg =
            [make_remap_bg(0, &shared.remap_nearest), make_remap_bg(1, &shared.remap_nearest)];

        WaterfallResources {
            tex_w,
            bind_group,
            bind_group_nearest,
            row_buf,
            hist,
            hist_view,
            active: 0,
            last_rows_seq: None,
            avg_row: RowAverage::default(),
            remap_uniforms,
            remap_bg,
            shift_bg,
            lut_tex,
            uniforms,
            write_row: 0,
            current_lut: None,
            last_center: 0.0,
            last_span: 0.0,
        }
    }
}

// WELCH averages the waterfall's rows over a time, set by the operator
// (`ViewState::welch_tc`), not over a number of rows: at a KiwiSDR's 12 kHz
// there is about one transform per row, so the engine's mean within a row has
// nothing to average, and what calms the picture is averaging across rows.
// Why: HDZ 2026-11-06: "am Wasserfall ändert es nicht so viel, fast
//      unsichtbar"; "die Zeitkonstante als Regler in die FFT legen und in
//      0,1-s-Schritten noch viel größer machbar lassen, bis die Körnigkeit weg
//      ist".

/// WELCH's column kernel: each column with its four neighbours, (1 2 3 2 1)/9.
pub const WELCH_KERNEL: [f32; 5] = [1.0 / 9.0, 2.0 / 9.0, 3.0 / 9.0, 2.0 / 9.0, 1.0 / 9.0];

/// REASSIGN's column kernel: the row as the engine drew it, each line as
/// wide as computed. Fixed; it was a width rail first.
// Why: HDZ 2026-11-07: "line kann schon mal fix auf 1, und Regler weg".
pub const REASSIGN_KERNEL: [f32; 1] = [1.0];

/// Averages waterfall rows in place (WELCH, REASSIGN): each column with its
/// neighbours under a kernel (odd length, centred), then with the running
/// average of the rows before it — all in linear power; averaging the bytes,
/// which are dB, would pull every column towards its quieter neighbours.
///
/// What it costs per row is kept off the hot path: the byte → power table and
/// the byte boundaries in power are built once per floor/ceil, and the column
/// pass reuses one buffer. The running mean is held in absolute power, so a
/// FIT glide that moves floor and ceiling every step leaves it valid; a retune
/// or pan (another centre or span) starts it afresh, since its columns are
/// other frequencies.
// Why: HDZ 2026-11-06: "diesen grieseligen Kram finde ich schauerlich".
#[derive(Default)]
struct RowAverage {
    avg: Vec<f32>,
    scratch: Vec<f32>,
    /// The floor and ceiling the tables were built for.
    levels: Option<(f32, f32)>,
    /// The power each byte stands for.
    lin: Vec<f32>,
    /// The power at which a byte rounds up to the next: 255 ascending edges.
    edges: Vec<f32>,
    /// The centre and span the average describes.
    place: Option<(f64, f64)>,
}

impl RowAverage {
    /// Forget the running mean (the average switched off).
    fn clear(&mut self) {
        self.avg.clear();
        self.place = None;
    }

    /// Average `row` (from `frame`), `new` the new row's share.
    fn row(&mut self, row: &mut [u8], frame: &SpectrumFrame, new: f32, kernel: &[f32]) {
        let (floor, ceil) = (frame.db_floor, frame.db_ceil);
        if ceil.is_nan() || floor.is_nan() || ceil <= floor || row.is_empty() || kernel.is_empty() {
            return;
        }
        if self.place != Some((frame.center_hz, frame.span_hz)) {
            self.place = Some((frame.center_hz, frame.span_hz));
            self.avg.clear();
        }
        if self.levels != Some((floor, ceil)) {
            self.levels = Some((floor, ceil));
            let step = (ceil - floor) / 255.0;
            let p = |b: f32| 10f32.powf((floor + b * step) / 10.0);
            self.lin = (0..256).map(|b| p(b as f32)).collect();
            self.edges = (0..255).map(|b| p(b as f32 + 0.5)).collect();
        }
        let n = row.len();
        let r = (kernel.len() / 2) as isize;
        self.scratch.clear();
        self.scratch.extend((0..n).map(|i| {
            kernel
                .iter()
                .enumerate()
                .map(|(j, w)| {
                    let at = (i as isize + j as isize - r).clamp(0, n as isize - 1) as usize;
                    w * self.lin[row[at] as usize]
                })
                .sum::<f32>()
        }));
        if self.avg.len() != n {
            std::mem::swap(&mut self.avg, &mut self.scratch);
        } else {
            let new = new.clamp(0.0, 1.0);
            for (a, v) in self.avg.iter_mut().zip(&self.scratch) {
                *a += new * (v - *a);
            }
        }
        for (b, a) in row.iter_mut().zip(&self.avg) {
            *b = self.edges.partition_point(|e| e <= a) as u8;
        }
    }
}

/// Per-paint callback carrying the latest frame and view mapping. The frame is
/// shared via `Arc` so per-repaint handoff never deep-clones the bins.
pub struct WaterfallCallback {
    pub frame: Option<std::sync::Arc<SpectrumFrame>>,
    /// Viewport in texture-u coordinates.
    pub u_lo: f32,
    pub u_hi: f32,
    /// Widget height in display rows.
    pub rows_visible: f32,
    pub lut: usize,
    /// Waterfall rows to append this frame. The app derives this from elapsed
    /// wall-clock time × the scroll rate, so the waterfall and the time
    /// gridlines advance together regardless of the actual frame cadence.
    pub rows_to_write: u32,
    /// Draw the newest row at the bottom, scrolling upwards.
    pub flip: bool,
    /// Interpolate between bins and rows rather than drawing each as a block.
    pub smooth: bool,
    /// Average each row lightly with its neighbouring columns and with the
    /// rows before it, in linear power, before it is written (the SMOOTH
    /// chip). The engine's mean (Welch) calms the noise inside a row; this
    /// takes the remaining grain out between rows.
    pub average: bool,
    /// The share of a new row in that average — see `ViewState::welch_tc`.
    pub avg_new: f32,
    /// The column kernel of that average: [`WELCH_KERNEL`] or
    /// [`REASSIGN_KERNEL`].
    pub kernel: Vec<f32>,
    /// Which radio's history to scroll — see [`WaterfallRegistry`].
    pub wf_id: u64,
    /// Columns the history should hold: the width the client has settled on
    /// for this machine and this screen (see [`auto_display_bins`]).
    ///
    /// Not the frame's own width. The engine may serve a narrower frame — it
    /// clamps what it is asked for against its own FFT, and on a station with
    /// two clients the other one may have asked for less — and a texture
    /// rebuilt every time that happened would throw the scrollback away on
    /// every zoom. Narrow frames are resampled up into this instead.
    pub tex_w: u32,
}

impl CallbackTrait for WaterfallCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(reg) = resources.get_mut::<WaterfallRegistry>() else {
            return Vec::new();
        };
        let WaterfallRegistry { shared, per } = reg;
        // Rounded up to the 256 bytes a buffer-to-texture copy wants a row to
        // be a multiple of. Every width actually on offer is a power of two
        // well above that, so this changes nothing today; it is here so a new
        // one cannot make the copy below fail a validation rule that has
        // nothing to do with waterfalls.
        let want_w = self.tex_w.clamp(DEFAULT_TEX_W, MAX_TEX_W).next_multiple_of(256);
        let r = per
            .entry(self.wf_id)
            .or_insert_with(|| WaterfallResources::new(device, shared, want_w));
        // A width change is the operator changing a setting, or a window
        // learning on its first layout how wide it really is. Losing the
        // scrollback is the honest price: a texture cannot be resized in place,
        // and the rows in hand were sampled on a different column grid anyway.
        // It happens once or twice a session and never under a pan or a zoom —
        // those go through the remap pass below, which is the whole reason the
        // texture width follows the *tier* and not the frame.
        if r.tex_w != want_w {
            *r = WaterfallResources::new(device, shared, want_w);
        }

        if r.current_lut != Some(self.lut) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &r.lut_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &colormap::lut(self.lut),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256 * 4),
                    rows_per_image: None,
                },
                wgpu::Extent3d { width: 256, height: 1, depth_or_array_layers: 1 },
            );
            r.current_lut = Some(self.lut);
        }

        if let Some(frame) = &self.frame {
            // The history is stored on one frequency mapping. When the frame's
            // span/centre moves off it (zoom, retune, a pan carrying the
            // window), remap the existing history onto the new axis instead of
            // clearing it, so the waterfall continues.
            let cols = f64::from(r.tex_w);
            let mv =
                history_move((r.last_center, r.last_span), (frame.center_hz, frame.span_hz), cols);
            if let Some(mv) = mv {
                if r.last_span > 0.0 {
                    // Destination column (new axis) -> source column (old axis):
                    // u_src = u_dst * (new_span/old_span) + (new_base-old_base)/old_span.
                    let (scale, offset, bg) = match mv {
                        HistoryMove::Rescale => {
                            let old_base = r.last_center - r.last_span / 2.0;
                            let new_base = frame.center_hz - frame.span_hz / 2.0;
                            (
                                (frame.span_hz / r.last_span) as f32,
                                ((new_base - old_base) / r.last_span) as f32,
                                &r.remap_bg,
                            )
                        }
                        // A pure shift: the bases differ by whole columns and
                        // nothing else, so every destination column lands on a
                        // source texel centre and nearest sampling copies it
                        // whole. `1 / tex_w` is exact in binary, so the offset
                        // is too.
                        HistoryMove::Shift(by) => (1.0, (by / cols) as f32, &r.shift_bg),
                    };
                    let rm: [f32; 4] = [scale, offset, 0.0, 0.0];
                    let bytes: [u8; 16] = unsafe { std::mem::transmute(rm) };
                    queue.write_buffer(&r.remap_uniforms, 0, &bytes);
                    let (src, dst) = (r.active, 1 - r.active);
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("waterfall-remap-pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &r.hist_view[dst],
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        pass.set_pipeline(&shared.remap_pipeline);
                        pass.set_bind_group(0, &bg[src], &[]);
                        pass.draw(0..3, 0..1);
                    }
                    r.active = dst;
                } else {
                    // First frame on this history: nothing to remap, just start
                    // clean. A clear pass rather than uploading a texture's
                    // worth of zeros — the history already carries
                    // `RENDER_ATTACHMENT` for the remap above, and a host buffer
                    // the size of the texture is real memory (16 MB a radio at
                    // the widest tier) held for the life of the tab to be used
                    // once.
                    //
                    // Recorded by being dropped: a pass that binds nothing
                    // still carries out its `LoadOp`.
                    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("waterfall-clear-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &r.hist_view[r.active],
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    }));
                }
                // Where the texture now is, which after a snapped shift is the
                // frame's axis to within half a column and not exactly on it.
                // Recording the frame's instead would hand that half column to
                // the next shift, and a drag would round it away frame after
                // frame until the picture had crept a long way from the band
                // it is labelled with.
                r.last_center = match mv {
                    HistoryMove::Shift(by) if r.last_span > 0.0 => {
                        r.last_center + by * frame.span_hz / cols
                    }
                    _ => frame.center_hz,
                };
                r.last_span = frame.span_hz;
            }
            // Where the frame carries its own rows they *are* the scroll: the
            // engine clocked them at the rate this client asked for, each one
            // the loudest thing its slice of time contained, and they are
            // appended once — hence `last_rows_seq`, because this callback runs
            // on every repaint and a frame outlives several of them.
            //
            // Where it carries none (a radio's own sweep, a transmit monitor)
            // the fallback is what every build before this one did everywhere:
            // repeat the current spectrum at `rows_to_write`, which the app
            // derives from elapsed wall-clock × the scroll rate.
            let cols = frame.bins.len();
            // `rows_clocked` first: it selects the fallback below, which
            // repeats the current spectrum `rows_to_write` times and reads
            // nothing out of `rows`. A frame that carried rows *and* asked for
            // the fallback would take the other branch and index past them.
            // Engines here never send that pair (`Engine::attach_rows`), but
            // this is a render callback reading a struct off the network, and
            // the cost of not trusting it is one `&&`.
            let carried = if !frame.rows_clocked || r.last_rows_seq == Some(frame.seq) {
                0
            } else {
                frame.row_count()
            };
            // A frame's rows belong to it, so consuming them is per frame and
            // not per repaint.
            if carried > 0 {
                r.last_rows_seq = Some(frame.seq);
            }
            let n = rows_to_append(frame.rows_clocked, carried, self.rows_to_write)
                .min(MAX_APPEND_ROWS);
            if n > 0 && cols > 0 {
                // Resample to texture width where the frame is not already it.
                // Routine rather than exceptional: the engine clamps the width
                // it is asked for against its own FFT, and a station serving two
                // clients answers whichever spoke last.
                let w = r.tex_w as usize;
                let widen = |src: &[u8], dst: &mut [u8]| {
                    if src.len() == w {
                        dst.copy_from_slice(src);
                    } else {
                        for (i, d) in dst.iter_mut().enumerate() {
                            *d = src[i * src.len() / w];
                        }
                    }
                };
                let mut block = vec![0u8; w * n as usize];
                for i in 0..n as usize {
                    // The fallback repeats the current spectrum; a clocking
                    // lane has a row of its own for each.
                    let src = if carried == 0 {
                        &frame.bins[..]
                    } else {
                        &frame.rows[i * cols..(i + 1) * cols]
                    };
                    widen(src, &mut block[i * w..(i + 1) * w]);
                    if self.average {
                        r.avg_row.row(
                            &mut block[i * w..(i + 1) * w],
                            &frame,
                            self.avg_new,
                            &self.kernel,
                        );
                    }
                }
                if !self.average {
                    r.avg_row.clear();
                }
                queue.write_buffer(&r.row_buf, 0, &block);
                // Recorded on the encoder, so it runs after the remap pass
                // above and the rows land on the axis they were clocked for.
                // In two copies where the ring wraps: a texture copy is one
                // rectangle and the newest row may be the last in the ring.
                let mut done = 0u32;
                while done < n {
                    let at = (r.write_row + done) % TEX_H;
                    let run = (n - done).min(TEX_H - at);
                    encoder.copy_buffer_to_texture(
                        wgpu::TexelCopyBufferInfo {
                            buffer: &r.row_buf,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: u64::from(done) * r.tex_w as u64,
                                bytes_per_row: Some(r.tex_w),
                                rows_per_image: Some(run),
                            },
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &r.hist[r.active],
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: 0, y: at, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d { width: r.tex_w, height: run, depth_or_array_layers: 1 },
                    );
                    done += run;
                }
                r.write_row = (r.write_row + n) % TEX_H;
            }
        }

        // Newest row center sits at (write_row - 0.5) / TEX_H.
        let u = Uniforms {
            scroll: (r.write_row as f32 - 0.5) / TEX_H as f32,
            vscale: (self.rows_visible / TEX_H as f32).min(1.0),
            u_lo: self.u_lo,
            u_hi: self.u_hi,
            flip: if self.flip { 1.0 } else { 0.0 },
            _pad: [0.0; 3],
        };
        let bytes: [u8; 32] = unsafe { std::mem::transmute(u) };
        queue.write_buffer(&r.uniforms, 0, &bytes);
        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(reg) = resources.get::<WaterfallRegistry>() else { return };
        let Some(r) = reg.per.get(&self.wf_id) else { return };
        pass.set_pipeline(&reg.shared.pipeline);
        let bg = if self.smooth { &r.bind_group } else { &r.bind_group_nearest };
        pass.set_bind_group(0, &bg[r.active], &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod history_move_tests {
    use super::{HistoryMove, history_move};

    /// A 2048-column history over 2 MHz: a column is a bit under a kilohertz.
    const COLS: f64 = 2048.0;
    const SPAN: f64 = 2_000_000.0;

    /// The panadapter drag this was written for: the window follows the
    /// gesture, so the centre arrives a few columns further along every frame
    /// and the history slides by exactly that many.
    #[test]
    fn a_window_that_moved_slides_by_whole_columns() {
        let col = SPAN / COLS;
        let mv = history_move((14e6, SPAN), (14e6 + 3.0 * col, SPAN), COLS);
        assert_eq!(mv, Some(HistoryMove::Shift(3.0)));
        let mv = history_move((14e6, SPAN), (14e6 - 3.0 * col, SPAN), COLS);
        assert_eq!(mv, Some(HistoryMove::Shift(-3.0)));
    }

    /// The whole point: a move too small to reach a column is not a move.
    /// Doppler on a satellite pass walks the centre a few hertz at a time, and
    /// resampling the history for each of those is how it used to be blurred
    /// away while the picture stood still.
    #[test]
    fn a_move_under_half_a_column_is_not_a_move() {
        assert_eq!(history_move((14e6, SPAN), (14e6 + 400.0, SPAN), COLS), None);
        assert_eq!(history_move((14e6, SPAN), (14e6 - 400.0, SPAN), COLS), None);
        // Just over half a column is, though — the residue is not allowed to
        // sit there for ever.
        assert_eq!(
            history_move((14e6, SPAN), (14e6 + 500.0, SPAN), COLS),
            Some(HistoryMove::Shift(1.0))
        );
    }

    /// A zoom changes the span, and then nothing lines up column for column.
    #[test]
    fn a_zoom_has_to_resample() {
        assert_eq!(
            history_move((14e6, SPAN), (14e6, SPAN / 2.0), COLS),
            Some(HistoryMove::Rescale)
        );
    }

    /// The first frame on an empty history: there is no axis to move from, and
    /// the caller reads this as "start clean".
    #[test]
    fn an_empty_history_asks_to_be_rescaled() {
        assert_eq!(history_move((0.0, 0.0), (14e6, SPAN), COLS), Some(HistoryMove::Rescale));
    }

    /// A frame with no span at all is not an axis; nothing is done to the
    /// history on the strength of it.
    #[test]
    fn a_frame_with_no_span_moves_nothing() {
        assert_eq!(history_move((14e6, SPAN), (14e6, 0.0), COLS), None);
    }
}

#[cfg(test)]
mod display_class_tests {
    use super::{DEFAULT_TEX_W, DisplayClass, MAX_TEX_W, auto_display_bins, manual_ceiling, wgpu};

    /// A capable desktop: discrete GPU, Vulkan, plenty of cores, engine in the
    /// same process.
    fn desktop() -> DisplayClass {
        DisplayClass {
            max_texture_dim: 16_384,
            device_type: wgpu::DeviceType::DiscreteGpu,
            backend: wgpu::Backend::Vulkan,
            remote: false,
            cores: 16,
        }
    }

    /// The whole point of the change: a 4K panadapter gets a column per pixel
    /// instead of one per two.
    #[test]
    fn a_4k_panel_on_a_real_gpu_gets_a_column_per_pixel() {
        assert_eq!(auto_display_bins(desktop(), 3840), 4096);
    }

    /// And a 1080p one does not pay for columns it cannot show.
    #[test]
    fn a_smaller_panel_is_not_made_to_pay_for_columns_it_cannot_show() {
        assert_eq!(auto_display_bins(desktop(), 1920), 2048);
        assert_eq!(auto_display_bins(desktop(), 1280), 2048);
    }

    /// Before the first layout there is no width to read, and the answer has to
    /// be the safe one rather than a guess.
    #[test]
    fn an_unmeasured_panadapter_gets_the_standard_width() {
        assert_eq!(auto_display_bins(desktop(), 0), DEFAULT_TEX_W);
    }

    /// A Raspberry Pi, as this tree actually meets one: forced onto the GL
    /// backend because V3DV flickers (see `crate::wgpu_options`), driving a 4K
    /// HDMI panel. It must not be handed the wide waterfall.
    #[test]
    fn a_raspberry_pi_on_a_4k_panel_stays_standard() {
        let pi = DisplayClass {
            max_texture_dim: 4096,
            device_type: wgpu::DeviceType::IntegratedGpu,
            backend: wgpu::Backend::Gl,
            remote: false,
            cores: 4,
        };
        assert_eq!(auto_display_bins(pi, 3840), DEFAULT_TEX_W);
        assert_eq!(manual_ceiling(pi), DEFAULT_TEX_W);
    }

    /// llvmpipe: the texture limit is enormous and says nothing at all about
    /// what the machine can draw.
    #[test]
    fn a_software_rasteriser_stays_standard_however_big_its_textures() {
        let sw = DisplayClass {
            max_texture_dim: 16_384,
            device_type: wgpu::DeviceType::Cpu,
            backend: wgpu::Backend::Vulkan,
            remote: false,
            cores: 8,
        };
        assert_eq!(auto_display_bins(sw, 3840), DEFAULT_TEX_W);
        assert_eq!(manual_ceiling(sw), DEFAULT_TEX_W);
    }

    /// The case the reported texture limit misses on its own: a WebGL2 session
    /// advertises the host's `GL_MAX_TEXTURE_SIZE` and would sail past every
    /// size rule. The backend is what gives it away.
    #[test]
    fn a_webgl2_session_is_caught_by_its_backend_not_its_limit() {
        let web = DisplayClass {
            max_texture_dim: 16_384,
            device_type: wgpu::DeviceType::DiscreteGpu,
            backend: wgpu::Backend::Gl,
            remote: true,
            cores: 8,
        };
        assert_eq!(auto_display_bins(web, 3840), DEFAULT_TEX_W);
    }

    /// Auto will not put a quarter of a megabyte a second on a link it cannot
    /// measure — but the operator who knows their own link still may.
    #[test]
    fn a_remote_client_is_cautious_by_default_and_overridable_by_hand() {
        let remote = DisplayClass { remote: true, ..desktop() };
        assert_eq!(auto_display_bins(remote, 3840), DEFAULT_TEX_W);
        assert_eq!(manual_ceiling(remote), MAX_TEX_W);
    }

    /// Shared memory and a shared power budget: room for a 4K panel, not for
    /// oversampling it.
    #[test]
    fn an_integrated_gpu_gets_a_4k_panel_but_not_twice_over() {
        let igpu = DisplayClass { device_type: wgpu::DeviceType::IntegratedGpu, ..desktop() };
        assert_eq!(auto_display_bins(igpu, 3840), 4096);
        assert_eq!(auto_display_bins(igpu, 7680), 4096);
    }

    /// A GPU that will not hold the texture is the one limit nothing overrules.
    #[test]
    fn the_texture_limit_is_never_overruled() {
        let small = DisplayClass { max_texture_dim: 2048, ..desktop() };
        assert_eq!(auto_display_bins(small, 3840), DEFAULT_TEX_W);
        assert_eq!(manual_ceiling(small), DEFAULT_TEX_W);
    }

    /// No machine and no screen can come out of this worse off than it went in,
    /// or wider than a texture anyone has agreed to allocate.
    #[test]
    fn every_answer_is_a_power_of_two_in_range() {
        let machines = [
            desktop(),
            DisplayClass { remote: true, ..desktop() },
            DisplayClass { cores: 1, ..desktop() },
            DisplayClass { cores: 2, max_texture_dim: 2048, ..desktop() },
            DisplayClass { device_type: wgpu::DeviceType::Cpu, ..desktop() },
            DisplayClass { device_type: wgpu::DeviceType::Other, ..desktop() },
            DisplayClass { backend: wgpu::Backend::Gl, ..desktop() },
            DisplayClass { max_texture_dim: 0, ..desktop() },
        ];
        for m in machines {
            for px in [0u32, 1, 640, 1920, 2560, 3840, 5120, 7680, 15_360, u32::MAX / 2] {
                let n = auto_display_bins(m, px);
                assert!((DEFAULT_TEX_W..=MAX_TEX_W).contains(&n), "{m:?} at {px} gave {n}");
                assert!(n.is_power_of_two(), "{m:?} at {px} gave {n}");
                assert!(n <= manual_ceiling(m).max(DEFAULT_TEX_W), "{m:?} at {px} gave {n}");
            }
        }
    }

    /// Two cores is a machine whose spare capacity is the radio's, not the
    /// panadapter's.
    #[test]
    fn a_thin_machine_stays_standard() {
        assert_eq!(auto_display_bins(DisplayClass { cores: 2, ..desktop() }, 3840), DEFAULT_TEX_W);
        assert_eq!(auto_display_bins(DisplayClass { cores: 4, ..desktop() }, 3840), 4096);
    }
}

#[cfg(test)]
mod row_append_tests {
    use super::{MAX_FALLBACK_ROWS, rows_to_append};
    use super::{REASSIGN_KERNEL, RowAverage, WELCH_KERNEL};

    /// The regression: a lane that clocks rows owns the scroll completely, so a
    /// frame carrying none means "nothing new yet", not "scroll it yourself".
    ///
    /// Getting this wrong was invisible at fast scroll rates, where nearly every
    /// frame carries a row, and obvious at slow ones, where nearly none do — the
    /// waterfall ran at roughly double the rate of its own time labels.
    #[test]
    fn a_clocking_lane_with_nothing_new_scrolls_not_at_all() {
        assert_eq!(rows_to_append(true, 0, 5), 0);
        assert_eq!(rows_to_append(true, 0, 32), 0);
    }

    /// And when it does bring rows, it brings exactly what is drawn.
    #[test]
    fn a_clocking_lane_draws_what_it_brought() {
        assert_eq!(rows_to_append(true, 1, 0), 1);
        assert_eq!(rows_to_append(true, 7, 99), 7);
        // Past the fallback's cap too: the engine bounds its own batch, and a
        // row it clocked is a row that really happened.
        assert_eq!(rows_to_append(true, 64, 0), 64);
    }

    /// A lane that cannot clock rows keeps the behaviour every build before
    /// this one had everywhere: repeat the current spectrum on the wall clock.
    #[test]
    fn a_lane_that_cannot_clock_falls_back_to_the_wall_clock() {
        assert_eq!(rows_to_append(false, 0, 3), 3);
        // Its rows, if any somehow arrived, are not what it scrolls by.
        assert_eq!(rows_to_append(false, 9, 3), 3);
        // And a hitch cannot dump a backlog into the texture at once.
        assert_eq!(rows_to_append(false, 0, 5_000), MAX_FALLBACK_ROWS);
    }

    fn flat_frame() -> sdroxide_types::SpectrumFrame {
        sdroxide_types::SpectrumFrame {
            seq: 0,
            center_hz: 0.0,
            span_hz: 1.0,
            db_floor: -120.0,
            db_ceil: -20.0,
            bins: Vec::new(),
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    /// SMOOTH's row average leaves an even row as it is and takes most of the
    /// grain out of a speckled one, row after row.
    #[test]
    fn the_row_average_calms_speckle_and_keeps_an_even_row() {
        let f = flat_frame();
        let mut avg = RowAverage::default();
        let mut even = vec![100u8; 64];
        avg.row(&mut even, &f, 0.5, &WELCH_KERNEL);
        assert!(even.iter().all(|b| (*b as i32 - 100).abs() <= 1), "{even:?}");
        let mut avg = RowAverage::default();
        let spread = |r: &[u8]| {
            let m = r.iter().map(|b| *b as f32).sum::<f32>() / r.len() as f32;
            (r.iter().map(|b| (*b as f32 - m).powi(2)).sum::<f32>() / r.len() as f32).sqrt()
        };
        let speckle =
            |t: usize| -> Vec<u8> { (0..64).map(|i| 90 + ((i * 7 + t * 13) % 23) as u8).collect() };
        let raw = speckle(0);
        let mut last = Vec::new();
        for t in 0..6 {
            let mut row = speckle(t);
            avg.row(&mut row, &f, 0.5, &WELCH_KERNEL);
            last = row;
        }
        assert!(spread(&last) < spread(&raw) * 0.6, "{} vs {}", spread(&last), spread(&raw));
    }

    /// REASSIGN's kernel leaves a one-column line one column wide and the
    /// floor at its level.
    #[test]
    fn the_reassign_kernel_keeps_a_line_as_computed() {
        let f = flat_frame();
        let mut row = vec![50u8; 64];
        row[32] = 200;
        let mut avg = RowAverage::default();
        avg.row(&mut row, &f, 1.0, &REASSIGN_KERNEL);
        let top = row[32] as i32;
        let wide = row.iter().filter(|&&b| top - b as i32 <= 8).count();
        assert_eq!(wide, 1, "{row:?}");
        assert!((row[5] as i32 - 50).abs() <= 1, "floor moved: {}", row[5]);
    }

    /// A FIT glide moves floor and ceiling under the running mean: a steady
    /// level stays the same level in dB, not the same byte.
    #[test]
    fn the_row_average_follows_a_new_floor_and_ceiling() {
        let mut f = flat_frame();
        let mut avg = RowAverage::default();
        // -70 dB on -120…-20: byte 127.5.
        for _ in 0..4 {
            let mut row = vec![128u8; 16];
            avg.row(&mut row, &f, 0.5, &WELCH_KERNEL);
        }
        // Floor 20 dB lower: the same -70 dB is byte ~149. A first row that
        // says so lands there at once, as the mean already holds that power.
        (f.db_floor, f.db_ceil) = (-140.0, -20.0);
        let mut row = vec![149u8; 16];
        avg.row(&mut row, &f, 0.1, &WELCH_KERNEL);
        assert!(row.iter().all(|b| (*b as i32 - 149).abs() <= 1), "{row:?}");
    }

    /// Another centre is other frequencies: the mean starts afresh.
    #[test]
    fn the_row_average_starts_afresh_on_a_retune() {
        let mut f = flat_frame();
        let mut avg = RowAverage::default();
        let mut row = vec![200u8; 16];
        avg.row(&mut row, &f, 0.5, &REASSIGN_KERNEL);
        f.center_hz = 7_100_000.0;
        let mut row = vec![50u8; 16];
        avg.row(&mut row, &f, 0.1, &REASSIGN_KERNEL);
        assert!(row.iter().all(|b| (*b as i32 - 50).abs() <= 1), "{row:?}");
    }
}
