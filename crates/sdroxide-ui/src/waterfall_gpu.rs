//! GPU waterfall: a ring-buffer history texture scrolled/panned/zoomed in the
//! fragment shader, colorized through a 256×1 LUT.
//!
//! WebGL2-downlevel-safe by design: no compute, no storage buffers, only
//! sampled R8Unorm/RGBA8 textures and one uniform buffer.

use eframe::egui_wgpu::{CallbackResources, CallbackTrait, RenderState, ScreenDescriptor, wgpu};
use sdroxide_types::SpectrumFrame;

use crate::colormap;

/// History texture width; must match `sdroxide_radio::engine::DISPLAY_BINS`.
pub const TEX_W: u32 = 2048;
/// History rows (scrollback depth).
pub const TEX_H: u32 = 2048;

#[repr(C)]
#[derive(Clone, Copy)]
struct Uniforms {
    scroll: f32,
    vscale: f32,
    u_lo: f32,
    u_hi: f32,
}

pub struct WaterfallResources {
    pipeline: wgpu::RenderPipeline,
    /// One render bind group per history texture; index by `active`.
    bind_group: [wgpu::BindGroup; 2],
    /// Ping-pong history textures. `active` is the live one; the other is the
    /// scratch target for the frequency-remap pass on a geometry change.
    hist: [wgpu::Texture; 2],
    hist_view: [wgpu::TextureView; 2],
    active: usize,
    // Remap pass: rewrites the history to a new frequency axis instead of
    // clearing it, so zoom/retune keeps the existing waterfall on screen.
    remap_pipeline: wgpu::RenderPipeline,
    remap_uniforms: wgpu::Buffer,
    remap_bg: [wgpu::BindGroup; 2],
    lut_tex: wgpu::Texture,
    uniforms: wgpu::Buffer,
    write_row: u32,
    current_lut: Option<usize>,
    last_center: f64,
    last_span: f64,
    zeros: Vec<u8>,
}

/// Create the pipeline/textures and register them in the renderer's
/// callback resources. Call once at app construction.
pub fn init(rs: &RenderState) {
    let device = &rs.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("waterfall"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/waterfall.wgsl").into()),
    });

    // Two history textures for ping-pong remapping. RENDER_ATTACHMENT lets the
    // remap pass render one into the other; R8Unorm is color-renderable on
    // WebGL2, so this stays downlevel-safe.
    let make_hist = |_i: usize| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("waterfall-history"),
            size: wgpu::Extent3d { width: TEX_W, height: TEX_H, depth_or_array_layers: 1 },
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
    let hist_view = [hist[0].create_view(&Default::default()), hist[1].create_view(&Default::default())];
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

    let linear = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("waterfall-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
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

    let lut_view = lut_tex.create_view(&Default::default());
    let make_bg = |i: usize| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("waterfall-bg"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&hist_view[i]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&linear),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&lut_sampler),
                },
            ],
        })
    };
    let bind_group = [make_bg(0), make_bg(1)];

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
    let remap_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("waterfall-remap-uniforms"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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
    let make_remap_bg = |i: usize| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("waterfall-remap-bg"),
            layout: &remap_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: remap_uniforms.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&hist_view[i]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&remap_sampler),
                },
            ],
        })
    };
    let remap_bg = [make_remap_bg(0), make_remap_bg(1)];
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

    rs.renderer.write().callback_resources.insert(WaterfallResources {
        pipeline,
        bind_group,
        hist,
        hist_view,
        active: 0,
        remap_pipeline,
        remap_uniforms,
        remap_bg,
        lut_tex,
        uniforms,
        write_row: 0,
        current_lut: None,
        last_center: 0.0,
        last_span: 0.0,
        zeros: Vec::new(),
    });
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
}

impl CallbackTrait for WaterfallCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(r) = resources.get_mut::<WaterfallResources>() else {
            return Vec::new();
        };

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
            // span/center changes (zoom/retune), remap the existing history onto
            // the new axis instead of clearing it, so the waterfall continues.
            let geom_changed = frame.span_hz > 0.0
                && ((frame.center_hz - r.last_center).abs() > frame.span_hz * 1e-6
                    || (frame.span_hz - r.last_span).abs() > frame.span_hz * 1e-6);
            let mut remapped = false;
            if geom_changed {
                if r.last_span > 0.0 {
                    // Destination column (new axis) -> source column (old axis):
                    // u_src = u_dst * (new_span/old_span) + (new_base-old_base)/old_span.
                    let old_base = r.last_center - r.last_span / 2.0;
                    let new_base = frame.center_hz - frame.span_hz / 2.0;
                    let rm: [f32; 4] = [
                        (frame.span_hz / r.last_span) as f32,
                        ((new_base - old_base) / r.last_span) as f32,
                        0.0,
                        0.0,
                    ];
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
                        pass.set_pipeline(&r.remap_pipeline);
                        pass.set_bind_group(0, &r.remap_bg[src], &[]);
                        pass.draw(0..3, 0..1);
                    }
                    r.active = dst;
                    remapped = true;
                } else {
                    // First frame ever: nothing to remap, just start clean.
                    if r.zeros.is_empty() {
                        r.zeros = vec![0u8; (TEX_W * TEX_H) as usize];
                    }
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &r.hist[r.active],
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &r.zeros,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(TEX_W),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d { width: TEX_W, height: TEX_H, depth_or_array_layers: 1 },
                    );
                }
                r.last_center = frame.center_hz;
                r.last_span = frame.span_hz;
            }
            // Skip appending a row on the remap frame: the new row is written via
            // the queue (applied before the encoder's remap pass in this submit),
            // so it would be overwritten. The next frame resumes normally — one
            // skipped row per zoom is imperceptible.
            // Time-driven scroll: append `rows_to_write` rows of the latest
            // frame (the app computes the count from elapsed wall-clock × the
            // scroll rate, so the axis is stable and matches the gridlines).
            let n = self.rows_to_write.min(32);
            if !remapped && n > 0 && !frame.bins.is_empty() {
                // Resample to texture width if bin count ever differs.
                let row: Vec<u8> = if frame.bins.len() == TEX_W as usize {
                    frame.bins.clone()
                } else {
                    (0..TEX_W as usize)
                        .map(|i| frame.bins[i * frame.bins.len() / TEX_W as usize])
                        .collect()
                };
                for _ in 0..n {
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &r.hist[r.active],
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: 0, y: r.write_row, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &row,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(TEX_W),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d { width: TEX_W, height: 1, depth_or_array_layers: 1 },
                    );
                    r.write_row = (r.write_row + 1) % TEX_H;
                }
            }
        }

        // Newest row center sits at (write_row - 0.5) / TEX_H.
        let u = Uniforms {
            scroll: (r.write_row as f32 - 0.5) / TEX_H as f32,
            vscale: (self.rows_visible / TEX_H as f32).min(1.0),
            u_lo: self.u_lo,
            u_hi: self.u_hi,
        };
        let bytes: [u8; 16] = unsafe { std::mem::transmute(u) };
        queue.write_buffer(&r.uniforms, 0, &bytes);
        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(r) = resources.get::<WaterfallResources>() else { return };
        pass.set_pipeline(&r.pipeline);
        pass.set_bind_group(0, &r.bind_group[r.active], &[]);
        pass.draw(0..3, 0..1);
    }
}
