//! Turns the ephemeris plus the user's settings into a flat list of GPU draws.
//!
//! Deliberately free of wgpu types: everything here is plain `Pod` data, so the
//! geometry can be reasoned about (and unit-tested) without a GPU.

use eframe::egui::Color32;
use sdroxide_solar::{AuroraOval, CloudField, SolarData, aurora, clouds, ephem};
use sdroxide_types::Coverage;

use super::camera::Camera;
use super::math::{M4, V3, v3};
use super::state::{Focus, SolarUi};
use crate::view::solar_layer as layer;

/// The scene's own inks: the classic navy/cyan palette, frozen.
///
/// The 3D world does not follow the UI theme. The globe itself cannot — the
/// Earth's oceans, coasts and atmosphere are baked into `solar_body.wgsl` in
/// these same hues — so everything drawn *over* it has to keep its contrast
/// against those fixed colours, whatever the chrome around the viewport wears.
/// Themed inks broke that: the amber theme recoloured the QSO arcs and orbit
/// rings into the globe's own brightness range and they vanished. The flat FT8
/// map already follows this rule.
mod ink {
    use eframe::egui::Color32;

    const fn c(rgb: u32) -> Color32 {
        Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Live accent: the Earth's glow, fresh traffic.
    pub const CYAN: Color32 = c(0x00d0f4);
    /// Quiet accent: orbit rings, body labels — present, not shouting.
    pub const CYAN_DIM: Color32 = c(0x1d9cbe);
    /// Faint structural lines: the Moon's path and the planets' moons'.
    pub const LINE_LIT: Color32 = c(0x2a4a66);
    /// Alarm: the locked satellite, Earth-bound CMEs, high-threat regions.
    pub const PINK: Color32 = c(0xff2a55);
    /// Highlight: search hits, the sub-solar point, the solar graticule.
    pub const YELLOW: Color32 = c(0xffd23f);
    /// Confirmation: the QTH ring, the aurora ovals, confirmed entities — and
    /// the arc to the station being worked, which is the same statement about
    /// the same operator's own station.
    pub const GREEN: Color32 = c(0x46e07d);
    /// Near-white: live station dots and the pulse crest on the active arc.
    pub const TEXT_STRONG: Color32 = c(0xe8f4ff);
}

/// Per-frame scene constants. 160 bytes; keep in step with `Globals` in the
/// shaders.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    /// xyz = eye position, w = near plane.
    pub camera_pos: [f32; 4],
    /// xyz = Sun centre, w = its rendered radius.
    pub sun_pos: [f32; 4],
    /// xyz = unit Sun→Earth, w = the SDO disk radius as a fraction of the image.
    pub sun_to_earth: [f32; 4],
    /// xyz = unit solar north, w = the Stonyhurst west sign.
    pub solar_north: [f32; 4],
    /// Viewport in pixels: w, h, 1/w, 1/h.
    pub viewport: [f32; 4],
    /// x = seconds (animation phase), y = photo/procedural blend for the Sun,
    /// z, w = spare.
    pub misc: [f32; 4],
}

/// How many lightning flashes may be alight at once.
///
/// Across sixty-odd storms flashing at up to four a second, with a tail a third
/// of a second long, the world average is a couple of dozen — but on a globe
/// only a handful are ever far enough apart to be told from one another, and a
/// fixed-size uniform is what keeps this out of a storage buffer the WebGL2
/// path would not have. The brightest eight win.
pub const MAX_FLASHES: usize = 8;

/// The lightning alight this instant, as the shaders see it.
///
/// A scene-level block rather than per-draw: every shell of the deck has to be
/// lit by the same flashes, and copying them into twenty-two `DrawData`s would
/// be the same bytes written twenty-two times.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Flashes {
    /// xyz = world position inside the tower, w = brightness now. A `w` of zero
    /// is an unused slot and contributes nothing, so the shader can loop over
    /// all of them in uniform control flow at a fixed cost.
    pub items: [[f32; 4]; MAX_FLASHES],
    /// x = how far the light reaches, in world units. y, z, w spare.
    pub reach: [f32; 4],
}

impl Default for Flashes {
    fn default() -> Self {
        Flashes { items: [[0.0; 4]; MAX_FLASHES], reach: [1.0, 0.0, 0.0, 0.0] }
    }
}

/// Per-draw constants. 192 bytes, uploaded at a dynamic offset.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawData {
    pub model: [[f32; 4]; 4],
    /// Rotation only (no scale), so normals and body-space lookups stay unit.
    /// A `mat4x4` rather than `mat3x3` on purpose: WGSL pads `mat3x3` columns
    /// to 16 bytes, which silently corrupts a naively packed uniform.
    pub basis: [[f32; 4]; 4],
    pub tint: [f32; 4],
    /// Second colour: the dark half of a gas giant's bands, or a ring's outer
    /// tint. Unused by the modes that do not need one.
    pub tint2: [f32; 4],
    /// x = shading mode, y = cone half-angle (radians), z = alpha, w = the
    /// cone's inner radius as a fraction of its length.
    pub params: [f32; 4],
    /// Surface style, for the modes that share one shading branch: x = the
    /// [`STYLE_*`] selector, y..w its parameters (see `solar_body.wgsl`).
    pub style: [f32; 4],
}

impl DrawData {
    /// A draw with everything that is usually left alone already zeroed.
    fn new(model: M4, basis: M4, tint: Color32, mode: f32) -> DrawData {
        DrawData {
            model: model.cols,
            basis: basis.cols,
            tint: lin(tint, 1.0),
            tint2: [0.0; 4],
            params: [mode, 0.0, 1.0, 0.0],
            style: [0.0; 4],
        }
    }
}

/// Shading branch selected by `DrawData::params.x`.
pub const MODE_EARTH: f32 = 0.0;
pub const MODE_MOON: f32 = 1.0;
pub const MODE_SUN: f32 = 2.0;
/// Every other body: a procedural surface picked by `DrawData::style.x`.
pub const MODE_BODY: f32 = 3.0;

/// Surface styles for [`MODE_BODY`], in step with `solar_body.wgsl`.
pub const STYLE_CRATERED: f32 = 0.0;
pub const STYLE_CLOUDY: f32 = 1.0;
pub const STYLE_ICE_GIANT: f32 = 2.0;
pub const STYLE_ICY: f32 = 3.0;
pub const STYLE_VOLCANIC: f32 = 4.0;
pub const STYLE_HAZE: f32 = 5.0;
/// Not procedural at all: sample layer `style.y` of the body-map array.
pub const STYLE_MAPPED: f32 = 6.0;

/// Which tail a [`Prim::Tail`] draw is, in step with `solar_tail.wgsl`.
/// Carried in `DrawData::params.x`, as the shading mode is for the spheres.
pub const TAIL_ION: f32 = 0.0;
pub const TAIL_DUST: f32 = 1.0;

/// Layers of that array, in `gpu::BODY_MAPS` order.
pub const MAP_MOON: f32 = 0.0;
pub const MAP_MARS: f32 = 1.0;
pub const MAP_JUPITER: f32 = 2.0;
pub const MAP_SATURN: f32 = 3.0;

/// How one body is painted: which procedural surface, in what colours, and the
/// two per-body switches that surface understands.
#[derive(Clone, Copy)]
pub struct Look {
    style: f32,
    /// Main colour — also what the body's glow and label take.
    base: Color32,
    /// The shade it varies towards: a gas giant's belts, a rocky body's basins.
    second: Color32,
    /// Style-specific: how strongly the second colour shows for a procedural
    /// surface, or which layer of the body-map array for [`STYLE_MAPPED`].
    detail: f32,
    /// Iapetus's dark leading hemisphere.
    two_tone: f32,
    /// How strong a limb glow a [`STYLE_MAPPED`] body's atmosphere gives it.
    /// Mars's dust is the only one so far; the giants' own limb darkening is
    /// already in the map.
    haze: f32,
}

/// The palette and surface each body type is drawn with.
///
/// Colours are eyeball-matched to what the body looks like through a telescope,
/// then muted a little towards the app's palette — a photographic Jupiter next
/// to this cyan Earth would read as a different program's window.
fn look_of(s: sdroxide_solar::Surface) -> Look {
    use sdroxide_solar::Surface as S;
    let look = |style, base: u32, second: u32, detail| Look {
        style,
        base: Color32::from_rgb((base >> 16) as u8, (base >> 8) as u8, base as u8),
        second: Color32::from_rgb((second >> 16) as u8, (second >> 8) as u8, second as u8),
        detail,
        two_tone: 0.0,
        haze: 0.0,
    };
    match s {
        S::Cratered => look(STYLE_CRATERED, 0x9a938a, 0x4a4642, 1.0),
        S::Cloudy => look(STYLE_CLOUDY, 0xe6d3a8, 0xc9ac74, 0.5),
        // Mars, and only Mars: the USGS Viking mosaic. Syrtis Major and the
        // caps are what any small telescope shows, so a procedural desert of
        // noise and ellipses was always going to be compared with a photograph
        // and lose. `base` is the map's own average pushed up in value — it is
        // what the glow and the label take, and MDIM 2.1's honest salmon-grey
        // is not legible against a black sky. The haze is the dust: a real,
        // faint limb, an order thinner than the Earth's.
        S::Desert => Look { haze: 0.35, ..look(STYLE_MAPPED, 0xc4785c, 0x7d3f2c, MAP_MARS) },
        // Both giants are drawn from Cassini's maps; `planet_look` picks which
        // layer, and this is only the fallback average colour.
        S::GasBands => look(STYLE_MAPPED, 0xd8be9c, 0x9a6945, MAP_JUPITER),
        S::IceGiant => look(STYLE_ICE_GIANT, 0x76c8e0, 0x3a7fb8, 0.5),
        S::Icy => look(STYLE_ICY, 0xd9e2ea, 0x8a9db0, 1.0),
        S::Volcanic => look(STYLE_VOLCANIC, 0xe0c25a, 0xa84e30, 1.0),
        S::Haze => look(STYLE_HAZE, 0xd79c4e, 0x9c662a, 0.4),
    }
}

/// The look of a specific planet: the shared surface type, plus what is true of
/// that planet alone.
fn planet_look(p: sdroxide_solar::Planet) -> Look {
    use sdroxide_solar::Planet as P;
    let mut look = look_of(p.info().surface);
    match p {
        // The two giants are drawn from Cassini's own global maps: their belts
        // and the Great Red Spot are things a viewer knows by sight, and no
        // procedural stand-in survives being compared with the photograph.
        // `base` stays the average colour, since it is what the glow and the
        // label take.
        P::Jupiter => {
            look.style = STYLE_MAPPED;
            look.detail = MAP_JUPITER;
            look.base = Color32::from_rgb(0xd8, 0xbe, 0x9c);
        }
        P::Saturn => {
            look.style = STYLE_MAPPED;
            look.detail = MAP_SATURN;
            look.base = Color32::from_rgb(0xe8, 0xd2, 0xa4);
        }
        // Neptune is the deeper blue of the two ice giants.
        P::Neptune => {
            look.base = Color32::from_rgb(0x54, 0x84, 0xd8);
            look.second = Color32::from_rgb(0x2c, 0x50, 0xa8);
        }
        _ => {}
    }
    look
}

fn moon_look(m: &sdroxide_solar::Moon) -> Look {
    let mut look = look_of(m.surface);
    // Cassini Regio: one hemisphere of Iapetus is as dark as asphalt and the
    // other is clean ice, which is the single most recognisable thing about it.
    if m.name == "Iapetus" {
        look.two_tone = 1.0;
    }
    look
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineInst {
    pub a: [f32; 3],
    pub width_px: f32,
    pub b: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInst {
    pub center: [f32; 3],
    pub size_px: f32,
    pub color: [f32; 4],
    /// x = kind (see `SPRITE_*`), y..w spare.
    pub params: [f32; 4],
}

pub const SPRITE_GLOW: f32 = 0.0;
pub const SPRITE_STAR: f32 = 1.0;
pub const SPRITE_RING: f32 = 2.0;
pub const SPRITE_DOT: f32 = 3.0;

/// Which static mesh a draw uses, and with which pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prim {
    Sphere,
    Cone,
    /// The sphere mesh again, drawn as an additive auroral emission shell.
    Aurora,
    /// A flat annulus: a planet's ring system.
    Ring,
    /// The sphere mesh again, as one slice through the cloud deck.
    Cloud,
    /// The sphere mesh once more, just above the surface, carrying the
    /// propagation heat as paint.
    Prop,
    /// One sphere at the top of the troposphere, with the deck marched through
    /// it in the fragment shader instead of sliced.
    CloudVolume,
    /// A comet's ion or dust tail, as a screen-facing ribbon.
    Tail,
}

/// A text label anchored to a point in the scene.
///
/// Drawn by the overlay with egui rather than in the 3D pass — there is no text
/// rendering on the GPU side, and projecting a handful of points is cheaper than
/// adding one.
pub struct Label {
    pub world: [f32; 3],
    pub text: String,
    pub color: Color32,
    /// Pixels to offset from the anchor, so a label does not sit on its marker.
    pub offset: [f32; 2],
    /// What clicking the label does: open a satellite's pass table, or point
    /// the camera at a body.
    pub click: Click,
    /// Who gets the screen when two labels want the same patch of it. Lower
    /// wins; [`RANK_ALWAYS`] is never dropped. See [`Label::rank`] for why the
    /// scene decides this rather than the overlay.
    pub rank: u8,
}

/// A label that is always painted, and takes its space before anything else
/// asks for it: the bodies, the satellites, the station being worked.
///
/// These are few, and each one was either asked for by name or is the subject
/// of the view — none of them is clutter to be thinned out.
pub const RANK_ALWAYS: u8 = 0;

impl Label {
    /// Where a callsign sits in that order, from `k` = 1 at full brightness to
    /// 0 as it ages out.
    ///
    /// The scene ranks them because the scene is what knows how fresh a decode
    /// is; by the time the overlay has them they are all just strings. Fresher
    /// stations get their pick of a crowded map, which is the right way round —
    /// a callsign still on the air is worth more than one on its way out.
    pub fn station_rank(k: f32) -> u8 {
        1 + ((1.0 - k.clamp(0.0, 1.0)) * 200.0) as u8
    }
}

/// What a clickable thing in the scene does.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Click {
    #[default]
    None,
    /// Open this satellite's pass table (by catalogue number).
    Sat(u64),
    /// Make this body the camera's target.
    Focus(Focus),
}

/// A body the pointer can grab to re-target the camera.
///
/// Kept apart from [`Label`] so a planet can be clicked whether or not the
/// labels layer is on, and so the disc itself is a target rather than only the
/// text beside it.
pub struct Pick {
    pub world: [f32; 3],
    /// Screen radius of the grab area, pixels.
    pub radius_px: f32,
    pub focus: Focus,
}

#[derive(Default)]
pub struct Scene {
    pub globals: Globals,
    pub draws: Vec<(Prim, DrawData)>,
    pub lines: Vec<LineInst>,
    pub sprites: Vec<SpriteInst>,
    pub labels: Vec<Label>,
    pub picks: Vec<Pick>,
    /// The star field is a static buffer on the GPU, so it is a flag here
    /// rather than 1500 instances rebuilt every frame.
    pub draw_stars: bool,
    /// The lightning alight this instant. Shared by every cloud draw.
    pub flashes: Flashes,
    /// Where the QSO arc reaches its highest point, in world space — what the
    /// contact card is pinned over. `None` whenever no QSO arc was drawn.
    ///
    /// It comes from here rather than being recomputed by the overlay because
    /// the apex has to be the apex of the arc that was *actually drawn*: it is
    /// the same slerp midpoint, lifted by the same bulge, through the same
    /// Earth basis. Two derivations of one point drift apart the moment either
    /// side is tuned, and the card would float off the arc.
    pub qso_apex: Option<[f32; 3]>,
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            view_proj: M4::IDENTITY.cols,
            camera_pos: [0.0; 4],
            sun_pos: [0.0; 4],
            sun_to_earth: [1.0, 0.0, 0.0, 0.45],
            solar_north: [0.0, 0.0, 1.0, 1.0],
            viewport: [1.0, 1.0, 1.0, 1.0],
            misc: [0.0; 4],
        }
    }
}

/// sRGB → linear. The offscreen target is an sRGB format, so the hardware
/// encodes on write; shader constants therefore have to be linear or every
/// colour comes out washed out relative to the rest of the UI.
pub fn lin(c: Color32, alpha: f32) -> [f32; 4] {
    let f = |v: u8| {
        let s = v as f32 / 255.0;
        if s <= 0.040_45 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    [f(c.r()), f(c.g()), f(c.b()), alpha]
}

/// A planet, placed and oriented for this frame.
pub struct PlanetBody {
    pub planet: sdroxide_solar::Planet,
    pub pos: V3,
    /// Rendered radius, which is the true one times [`PlanetBody::exaggeration`].
    pub radius: f32,
    pub basis: (V3, V3, V3),
    /// Ring system as rendered: inner radius, outer radius, opacity.
    pub rings: Option<(f32, f32, f32)>,
    /// How much the radius was exaggerated. Its moons are scaled by the same
    /// factor — orbit radii included — so the system keeps its true shape: a
    /// moon at six planet radii is drawn at six planet radii.
    pub exaggeration: f32,
}

/// A dwarf planet, asteroid or comet, placed for this frame.
pub struct SmallPlaced {
    /// Index into [`sdroxide_solar::smallbody::BODIES`], which is what
    /// [`Focus::Small`] stores.
    pub index: usize,
    pub info: &'static sdroxide_solar::SmallBody,
    pub pos: V3,
    /// Rendered radius. Exaggerated like a planet's, and for the asteroids
    /// still far below a pixel at any setting — their sprite is what makes them
    /// visible, and the sphere is skipped entirely below [`SPHERE_MIN_PX`].
    pub radius: f32,
    /// The tails, if it has grown any. Directions are unit vectors in world
    /// space, lengths are gigametres — see [`sdroxide_solar::smallbody::Tails`].
    pub tails: Option<sdroxide_solar::smallbody::Tails>,
}

/// A moon of one of those planets.
pub struct MoonBody {
    /// Index into [`sdroxide_solar::planets::MOONS`], which is what
    /// [`Focus::Satellite`] stores.
    pub index: usize,
    pub info: &'static sdroxide_solar::Moon,
    pub pos: V3,
    pub radius: f32,
    /// The planet it belongs to, as an index into [`Bodies::planets`].
    pub parent: usize,
}

/// Positions of everything, in the heliocentric ecliptic frame, gigametres.
pub struct Bodies {
    pub jd: f64,
    pub sun_r: f32,
    pub earth: V3,
    pub earth_r: f32,
    pub earth_basis: (V3, V3, V3),
    pub moon: V3,
    pub moon_r: f32,
    pub moon_basis: (V3, V3, V3),
    /// The Moon's TRUE geocentric offset, gigametres — no orbit compression,
    /// no body exaggeration. The eclipse shadow it casts on the Earth is
    /// computed from this, so the shadow's track stays geographically true
    /// whatever the sliders say the Moon itself looks like.
    pub moon_true_off: V3,
    pub sun_frame: ephem::SunFrame,
    pub planets: Vec<PlanetBody>,
    pub moons: Vec<MoonBody>,
    pub smalls: Vec<SmallPlaced>,
}

/// The most of the Sun's rendered radius any planet may reach.
///
/// The body slider goes to 20×, at which Jupiter would be twice the Sun's size
/// and the picture would be nonsense. Capping the exaggeration per body keeps
/// the ordering everyone knows — the Sun dwarfs everything — while still
/// lifting Mercury off a single pixel.
const PLANET_MAX_SUN_FRAC: f32 = 0.35;

fn basis_cols(b: sdroxide_solar::Basis) -> (V3, V3, V3) {
    (V3::from_f64(b.x), V3::from_f64(b.y), V3::from_f64(b.z))
}

pub fn bodies(st: &SolarUi, unix_s: f64) -> Bodies {
    let jd = ephem::julian_day(unix_s);
    let v = &st.view;
    let earth = V3::from_f64(ephem::earth_heliocentric(jd));
    let b = ephem::earth_basis(jd);
    let moon_true_off = V3::from_f64(ephem::moon_geocentric_vec(jd));
    let moon_off = moon_true_off * v.moon_orbit_scale;
    let sun_r = ephem::SUN_R as f32 * v.sun_scale;

    let mut planets = Vec::with_capacity(sdroxide_solar::Planet::ALL.len());
    let mut moons = Vec::new();
    for p in sdroxide_solar::Planet::ALL {
        let info = p.info();
        let true_r = info.radius as f32;
        let radius = (true_r * v.body_scale).clamp(true_r, sun_r * PLANET_MAX_SUN_FRAC);
        let exaggeration = radius / true_r;
        let pos = V3::from_f64(p.heliocentric(jd));
        let parent = planets.len();
        planets.push(PlanetBody {
            planet: p,
            pos,
            radius,
            basis: basis_cols(p.basis(jd)),
            rings: info
                .rings
                .map(|r| (r.inner as f32 * radius, r.outer as f32 * radius, r.opacity as f32)),
            exaggeration,
        });
        for (index, m) in sdroxide_solar::planets::MOONS.iter().enumerate() {
            if m.parent != p {
                continue;
            }
            moons.push(MoonBody {
                index,
                info: m,
                pos: pos + V3::from_f64(m.offset(jd)) * (exaggeration * v.moon_orbit_scale),
                radius: m.radius as f32 * exaggeration,
                parent,
            });
        }
    }

    // Dwarf planets, asteroids and comets. Radii are exaggerated on the same
    // slider as the planets, which lifts Pluto to something you can see and
    // leaves a 200 m asteroid at a millionth of a pixel — that one is a sprite
    // and a name, and pretending otherwise would be the lie.
    let smalls = sdroxide_solar::smallbody::BODIES
        .iter()
        .enumerate()
        .map(|(index, info)| {
            let true_r = info.radius as f32;
            let radius = (true_r * v.body_scale).clamp(true_r, sun_r * PLANET_MAX_SUN_FRAC);
            SmallPlaced {
                index,
                info,
                pos: V3::from_f64(info.heliocentric(jd)),
                radius,
                tails: info.tails(jd),
            }
        })
        .collect();

    Bodies {
        jd,
        sun_r,
        earth,
        earth_r: ephem::EARTH_R as f32 * v.body_scale,
        earth_basis: basis_cols(b),
        moon: earth + moon_off,
        moon_r: ephem::MOON_R as f32 * v.body_scale,
        moon_basis: basis_cols(ephem::moon_basis(jd)),
        moon_true_off,
        sun_frame: ephem::sun_frame(jd),
        planets,
        moons,
        smalls,
    }
}

/// How big a body has to be on screen before it is worth a sphere.
///
/// Below this the shaded mesh cannot resolve into anything a sprite does not
/// already say, and there are forty of these — so the sphere is skipped and the
/// glow billboard stands for the body. It is what every asteroid in the table
/// gets at every zoom short of flying up to one.
const SPHERE_MIN_PX: f32 = 1.5;

impl Bodies {
    /// Unit vector, in world space, from the Earth's centre towards a point on
    /// its surface. The Earth's own orientation is in `earth_basis`, so this
    /// turns with the planet.
    pub fn surface_dir(&self, lat: f64, lon: f64) -> V3 {
        let (ex, ey, ez) = self.earth_basis;
        let v = ephem::geodetic_to_body(lat, lon);
        (ex * v.x as f32 + ey * v.y as f32 + ez * v.z as f32).normalize()
    }

    /// Where the camera pivots, and a characteristic radius used to clamp how
    /// close it may get.
    pub fn focus(&self, f: Focus) -> (V3, f32) {
        match f {
            Focus::Sun => (V3::ZERO, self.sun_r),
            Focus::Earth => (self.earth, self.earth_r),
            Focus::Moon => (self.moon, self.moon_r),
            Focus::EarthMoon => (
                (self.earth + self.moon) * 0.5,
                ((self.moon - self.earth).len() * 0.5).max(self.earth_r),
            ),
            Focus::Planet(p) => self
                .planets
                .iter()
                .find(|b| b.planet == p)
                .map_or((V3::ZERO, self.sun_r), |b| (b.pos, b.radius)),
            Focus::Satellite(i) => self
                .moons
                .iter()
                .find(|m| m.index == i)
                // A moon can be tiny even exaggerated, so the clamp radius has
                // a floor: without one the camera may end up closer to it than
                // the near plane allows.
                .map_or((V3::ZERO, self.sun_r), |m| (m.pos, m.radius.max(1e-4))),
            // A comet's framing radius is its coma, not its nucleus: fly to
            // Halley at perihelion and the thing you came to see is a hundred
            // thousand kilometres across, while the body inside it is 11 km and
            // would put the camera inside the tail.
            Focus::Small(i) => {
                self.smalls.iter().find(|s| s.index == i).map_or((V3::ZERO, self.sun_r), |s| {
                    let coma = s.tails.map_or(0.0, |t| t.coma_gm as f32);
                    (s.pos, s.radius.max(coma).max(1e-4))
                })
            }
        }
    }
}

/// Build the frame's draw lists.
pub fn build(
    st: &SolarUi,
    data: Option<&SolarData>,
    unix_s: f64,
    size_px: [f32; 2],
    anim_t: f32,
) -> Scene {
    let b = bodies(st, unix_s);
    let cam = Camera::from_view(st, &b, size_px);
    let sun_img = data.and_then(|d| d.sun.as_ref());
    let mut s = Scene {
        globals: Globals {
            view_proj: cam.view_proj.cols,
            camera_pos: cam.eye.arr4(cam.near),
            sun_pos: V3::ZERO.arr4(b.sun_r),
            sun_to_earth: V3::from_f64(b.sun_frame.to_earth)
                .arr4(sun_img.map_or(0.45, |i| i.disk_radius_frac)),
            solar_north: V3::from_f64(b.sun_frame.basis.z).arr4(1.0),
            viewport: [size_px[0], size_px[1], 1.0 / size_px[0], 1.0 / size_px[1]],
            // misc.y blends the photograph in; zero until one has arrived, so
            // the procedural surface is what shows offline.
            misc: [anim_t, if sun_img.is_some() { 1.0 } else { 0.0 }, 0.0, 0.0],
        },
        // The star field and the graticule below are not layers any more: they
        // are the backdrop and the coordinate frame the rest is read against,
        // and a scene without them reads as broken rather than as uncluttered.
        draw_stars: true,
        ..Default::default()
    };

    bodies_draws(&mut s, st, &b, &cam);
    if st.layer(layer::ORBITS) {
        orbits(&mut s, st, &b, &cam);
    }
    grid(&mut s, &b);
    markers(&mut s, st, &b, &cam);
    // Before the markers and the arcs above it, and before the awards: this is
    // ground the rest of the layers stand on.
    if st.layer(layer::PROPAGATION) {
        prop_heat(&mut s, st, &b, &cam);
    }
    if st.layer(layer::AWARDS) {
        award_heat(&mut s, st, &b, &cam, anim_t);
    }
    if st.layer(layer::QSO) {
        digi_traffic(&mut s, st, &b, &cam, unix_s, anim_t);
    }
    if let Some(d) = data {
        let now = unix_s as i64;
        if st.layer(layer::SPOTS) {
            spots(&mut s, &b, &cam, &d.regions, now);
        }
        if st.layer(layer::FLARES) {
            flares(&mut s, &b, &cam, &d.flares, now);
        }
        if st.layer(layer::CME) {
            cones(&mut s, st, &d.cmes, now);
        }
        // Before the aurora, and it matters: the deck occludes and the oval
        // adds, so drawing the oval second puts its glow over the weather —
        // which is the way round every photograph from orbit has it.
        if let Some(field) = &d.clouds
            && st.layer(layer::CLOUDS)
        {
            let fade = cloud_deck(&mut s, st, &b, &cam, field);
            if fade > 0.0 {
                s.flashes = lightning(&b, &field.cells, unix_s, fade);
            }
        }
        if let Some(oval) = &d.aurora
            && st.layer(layer::AURORA)
        {
            aurora_shells(&mut s, &b, &cam, oval);
        }
        if st.layer(layer::SATS) {
            satellites(&mut s, st, &b, &cam, d, unix_s);
        }
    }
    // Always called: it also builds the click targets, which are not a layer —
    // turning names off should not make the planets unclickable.
    body_labels(&mut s, st, &b, &cam, size_px[1], st.layer(layer::LABELS));
    s
}

/// Amateur-radio satellites: position, orbit and label.
///
/// Distances come back from the tracker in Earth radii, so they scale with
/// whatever exaggerated radius the Earth is being drawn at — LEO stays just
/// above the surface and QO-100 stays 6.6 radii out, at any setting.
fn satellites(
    s: &mut Scene,
    st: &SolarUi,
    b: &Bodies,
    cam: &Camera,
    data: &SolarData,
    unix_s: f64,
) {
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // Orbits are several Earth radii across, so they are legible before a point
    // on the surface is — a lower threshold than the QTH marker's.
    let fade = ((earth_px - 1.5) / 6.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    let show_all = st.view.all_satellites;
    let place = |dir: sdroxide_solar::Vec3, radii: f64| {
        b.earth + V3::from_f64(dir) * (b.earth_r * radii as f32)
    };

    for sat in data.satellites() {
        // A search match is drawn whether or not it otherwise would be: looking
        // for a satellite that is not on screen is the main reason to search.
        let hit = st.sat_hit(&sat.name, sat.norad_id);
        // The locked satellite likewise: the engine is correcting Doppler
        // against this one *right now*, so it is on screen no matter what.
        let locked = st.sat_lock == Some(sat.norad_id);
        if !(show_all || sat.popular || hit || locked) {
            continue;
        }
        let Some(state) = sat.at(unix_s) else { continue };
        let pos = place(state.dir_ecliptic, state.radii);

        // Geostationary orbits read differently from low ones — one is a fixed
        // relay, the other a pass you have to catch — so they are coloured apart.
        let geo = sat.period_min > 1300.0;
        // A match is pulled out of that scheme entirely rather than merely
        // brightened: against ninety cyan dots, "a bit lighter" is not findable,
        // and yellow is the one accent nothing else in this layer uses. The
        // locked bird outranks even that, in the one colour louder still.
        let color = match (locked, hit, geo) {
            (true, ..) => ink::PINK,
            (false, true, _) => ink::YELLOW,
            (false, false, true) => ink::GREEN,
            (false, false, false) => ink::CYAN_DIM,
        };
        let ringed = sat.popular || hit || locked;

        s.sprites.push(SpriteInst {
            center: pos.arr(),
            size_px: if locked {
                11.0
            } else if hit {
                10.0
            } else if sat.popular {
                7.0
            } else {
                4.0
            },
            color: lin(color, (if ringed { 0.95 } else { 0.5 }) * fade),
            params: [SPRITE_DOT, 0.0, 0.0, 0.0],
        });

        // Orbit rings only for the ringed set: ninety of them at once is noise.
        // A pasted element set, a subscription with orbits on, and a search
        // match all land here; a ninety-satellite group without does not.
        if ringed {
            let path = sat.orbit(unix_s, 96);
            let (w_px, alpha) = if locked {
                (2.6, 0.95)
            } else if hit {
                (2.4, 0.9)
            } else {
                (1.3, 0.4)
            };
            for w in path.windows(2) {
                s.lines.push(seg(
                    place(w[0].0, w[0].1),
                    place(w[1].0, w[1].1),
                    w_px,
                    lin(color, alpha * fade),
                ));
            }
        }

        // The sightline from the operator to the locked bird: a straight
        // chord, not a great-circle arc — this is where the antenna points.
        // Dashed so it reads as a pointer rather than another orbit.
        if locked {
            if let Some((lat, lon)) = st.qth {
                let home = b.earth + b.surface_dir(lat, lon) * b.earth_r;
                const DASHES: usize = 12;
                for k in 0..DASHES {
                    let t0 = (k as f32 + 0.15) / DASHES as f32;
                    let t1 = (k as f32 + 0.85) / DASHES as f32;
                    s.lines.push(seg(
                        home + (pos - home) * t0,
                        home + (pos - home) * t1,
                        2.0,
                        lin(ink::PINK, 0.85 * fade),
                    ));
                }
            }
        }

        // A match is labelled even with the LABELS layer off and even when it
        // is not one of the curated few — the name is what was searched for, so
        // withholding it would defeat the search. The locked bird gets the
        // same treatment for the same reason. The floor is lower than the
        // ordinary one but not absent: on an Earth twelve pixels across there
        // is nothing for a label to point at.
        let labelled = if hit || locked {
            earth_px > 12.0
        } else {
            st.layer(layer::LABELS) && sat.popular && earth_px > 24.0
        };
        if labelled {
            // The elevation is what decides whether it is workable right now,
            // so it goes in the label rather than a separate panel.
            let text = match st.qth {
                Some((lat, lon)) => {
                    let el = state.elevation_from(lat, lon);
                    if el > 0.0 {
                        format!("{}  {el:.0}°", sat.name)
                    } else {
                        format!("{}  ▼", sat.name)
                    }
                }
                None => sat.name.clone(),
            };
            s.labels.push(Label {
                world: pos.arr(),
                text,
                color: lin_color(color, (if hit { 1.0 } else { 0.9 }) * fade),
                offset: [9.0, -7.0],
                click: Click::Sat(sat.norad_id),
                rank: RANK_ALWAYS,
            });
        }
    }
}

// ── Aurora ──────────────────────────────────────────────────────────────────
//
// The oval is drawn as a stack of concentric emission shells rather than as a
// texture painted on the globe, because that is what it is: a hundred-odd
// kilometres of thin glowing air standing above the surface. Doing it in the
// round is what produces the bright ribbon on the limb and the colour change
// with height, neither of which a flat overlay can show. See
// `shaders/solar_aurora.wgsl` for what each shell contributes.

/// The band of atmosphere the shells span, kilometres. Green oxygen emission
/// begins around 95 km; the red line is still radiating at 400.
const AURORA_BOTTOM_KM: f32 = 92.0;
const AURORA_TOP_KM: f32 = 400.0;
/// Earth mean radius, kilometres. Altitudes become fractions of whatever radius
/// the globe is *drawn* at, so the aurora stays attached to the surface at any
/// setting of the exaggeration slider — the same trick the satellites use.
const AURORA_EARTH_R_KM: f32 = 6371.0;
/// Shells are packed towards the bottom of the band, where the green emission
/// that dominates the picture lives; spacing them evenly would spend most of
/// them on the faint red top.
const AURORA_SHELL_BIAS: f32 = 1.5;

/// Altitude of shell `k` of `n`, kilometres.
fn shell_altitude_km(k: usize, n: usize) -> f32 {
    let t = if n <= 1 { 0.0 } else { k as f32 / (n - 1) as f32 };
    AURORA_BOTTOM_KM + (AURORA_TOP_KM - AURORA_BOTTOM_KM) * t.powf(AURORA_SHELL_BIAS)
}

/// How many shells to spend on the oval, given how big the Earth is on screen.
///
/// Every shell is a full additive sphere, so this is the cost knob: twenty
/// layers is worth it when the globe fills the window and pure waste when it is
/// twenty pixels across and the layering cannot be resolved at all.
pub fn shell_count(earth_px: f32) -> usize {
    match earth_px {
        px if px >= 260.0 => 20,
        px if px >= 90.0 => 14,
        px if px >= 26.0 => 8,
        _ => 4,
    }
}

/// The auroral oval: emission shells, and the contour of its equatorward edge.
fn aurora_shells(s: &mut Scene, b: &Bodies, cam: &Camera, oval: &AuroraOval) {
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // The oval is a feature *of the surface*, so it fades in with the same
    // threshold as the QTH marker: a polar band on a three-pixel Earth reads as
    // a property of the planet rather than of a place on it.
    let fade = ((earth_px - 3.0) / 12.0).clamp(0.0, 1.0);
    if fade <= 0.0 || oval.is_empty() {
        return;
    }

    let (ex, ey, ez) = b.earth_basis;
    let n = shell_count(earth_px);
    for k in 0..n {
        let alt = shell_altitude_km(k, n);
        // The slab of atmosphere this shell stands for: half way to each
        // neighbour. Passing it to the shader is what keeps total brightness
        // independent of how many shells were drawn.
        let lo = if k == 0 { alt } else { shell_altitude_km(k - 1, n) };
        let hi = if k + 1 == n { alt } else { shell_altitude_km(k + 1, n) };
        let slab = ((hi - lo) * 0.5).max(1.0);
        let radius = 1.0 + alt / AURORA_EARTH_R_KM;
        s.draws.push((
            Prim::Aurora,
            DrawData {
                model: M4::from_basis(ex, ey, ez, b.earth, b.earth_r * radius).cols,
                basis: M4::from_basis(ex, ey, ez, V3::ZERO, 1.0).cols,
                tint: [1.0; 4],
                tint2: [0.0; 4],
                params: [alt, slab, fade, 0.0],
                style: [0.0; 4],
            },
        ));
    }

    // The contour only means anything once a line on the surface can be told
    // apart from the surface, which is a good deal closer in than the glow.
    if earth_px > 60.0 {
        aurora_edge(s, b, oval, fade);
    }
}

/// The equatorward edge of the oval, as a ring on the globe.
///
/// This is the line to compare your own latitude against, and it comes from the
/// grid rather than from a rule of thumb about Kp — so it bulges towards the
/// equator on the night side and over the magnetic poles, which is where it
/// really does.
fn aurora_edge(s: &mut Scene, b: &Bodies, oval: &AuroraOval, fade: f32) {
    /// Sample spacing along the contour, degrees of longitude.
    const STEP_DEG: usize = 4;
    /// A step larger than this is the contour ending rather than a boundary
    /// that steep; joining across it would draw a chord through the oval.
    const MAX_JUMP_DEG: f64 = 14.0;

    let (ex, ey, ez) = b.earth_basis;
    let on_earth = |lat: f64, lon: f64| {
        let d = ephem::geodetic_to_body(lat, lon);
        b.earth + (ex * d.x as f32 + ey * d.y as f32 + ez * d.z as f32) * (b.earth_r * 1.004)
    };

    for north in [true, false] {
        let profile = oval.edge_profile(north, aurora::EDGE_PCT, STEP_DEG);
        for k in 0..profile.len() {
            let j = (k + 1) % profile.len();
            let (Some(from), Some(to)) = (profile[k], profile[j]) else { continue };
            if (from - to).abs() > MAX_JUMP_DEG {
                continue;
            }
            let lon = |i: usize| -180.0 + (i * STEP_DEG) as f64;
            s.lines.push(seg(
                on_earth(from, lon(k)),
                on_earth(to, lon(j)),
                1.4,
                lin(ink::GREEN, 0.45 * fade),
            ));
        }
    }
}

// ── Clouds ──────────────────────────────────────────────────────────────────
//
// Same argument as the aurora, one atmosphere lower down. Weather is a depth of
// air, not a picture stuck on a sphere, and the infrared mosaic hands over the
// one number that makes that drawable: how cold each cloud top is, and so how
// high it stands. A thunderhead towers here because it really is fifteen
// kilometres tall.
//
// Two ways to draw it, because they fail differently. The shell stack slices
// the troposphere into concentric spheres and is cheap and predictable. The
// volume path marches the slab per pixel and is what makes a flash glow
// *through* a storm instead of merely brightening its outside. See
// `shaders/solar_cloud.wgsl` and `shaders/solar_cloud_march.wgsl`.

/// The band of air the weather lives in, kilometres. Marine stratus sits a few
/// hundred metres up; a tropical anvil stops at the tropopause, which is where
/// [`clouds::TOP_MAX_KM`] stops too.
const CLOUD_BOTTOM_KM: f32 = 0.5;
const CLOUD_TOP_KM: f32 = clouds::TOP_MAX_KM;
/// Earth mean radius, kilometres — see [`AURORA_EARTH_R_KM`]. Altitudes are
/// fractions of the radius the globe is *drawn* at, so the deck stays glued to
/// the surface at any setting of the exaggeration slider.
const CLOUD_EARTH_R_KM: f32 = 6371.0;
/// How much the deck's thickness is exaggerated.
///
/// The one deliberate lie in this layer, and it is unavoidable: eighteen
/// kilometres on a six-thousand-kilometre planet is a quarter of one per cent
/// of the radius, which is a hairline at any zoom short of skimming the surface
/// — and a hairline cannot be volumetric. Six times over is enough for a storm
/// to stand up out of the deck and still shallow enough that nobody would mistake
/// the result for a mountain range.
const CLOUD_LIFT: f32 = 6.0;

/// How many shells to slice the deck into, given how big the Earth is on screen.
///
/// Lower than the aurora's counts at every step, because the aurora covers a
/// tenth of the planet and leaves the rest early, while cloud covers most of it
/// and every shell pays for nearly every pixel it crosses.
pub fn cloud_shell_count(earth_px: f32) -> usize {
    match earth_px {
        px if px >= 420.0 => 18,
        px if px >= 160.0 => 12,
        px if px >= 50.0 => 7,
        _ => 4,
    }
}

/// Altitude of cloud shell `k` of `n`, kilometres.
///
/// Packed towards the bottom, where nearly all the world's cloud is: spacing
/// them evenly would spend half of them on the thin air above the anvils.
fn cloud_altitude_km(k: usize, n: usize) -> f32 {
    let t = if n <= 1 { 0.0 } else { k as f32 / (n - 1) as f32 };
    CLOUD_BOTTOM_KM + (CLOUD_TOP_KM - CLOUD_BOTTOM_KM) * t.powf(1.7)
}

/// Radius of a point at altitude `alt_km`, as a multiple of the Earth's own.
fn cloud_radius(alt_km: f32) -> f32 {
    1.0 + alt_km * CLOUD_LIFT / CLOUD_EARTH_R_KM
}

/// The cloud deck, one way or the other.
///
/// Returns how strongly it was drawn, so the lightning can be skipped entirely
/// when there is no deck for it to be inside.
fn cloud_deck(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera, field: &CloudField) -> f32 {
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // Later than the aurora's threshold. The oval is a shape, legible at a few
    // pixels across; a cloud deck at that size is a grey wash that only makes
    // the planet harder to read.
    let fade = ((earth_px - 6.0) / 24.0).clamp(0.0, 1.0);
    if fade <= 0.0 || field.is_empty() {
        return 0.0;
    }

    let (ex, ey, ez) = b.earth_basis;
    let basis = M4::from_basis(ex, ey, ez, V3::ZERO, 1.0).cols;
    let draw = |prim: Prim, alt: f32, slab: f32, extra: f32| {
        (
            prim,
            DrawData {
                model: M4::from_basis(ex, ey, ez, b.earth, b.earth_r * cloud_radius(alt)).cols,
                basis,
                tint: [1.0; 4],
                // The Moon's true geocentric offset, as on the Earth draw: the
                // deck sits in the same eclipse shadow as the ground under it.
                tint2: b.moon_true_off.arr4(0.0),
                params: [alt, slab, fade, extra],
                style: [CLOUD_LIFT, CLOUD_BOTTOM_KM, CLOUD_TOP_KM, b.earth_r],
            },
        )
    };

    if st.view.cloud_march {
        // One sphere at the top of the slab; the shader owns everything under
        // it. `extra` is the step budget, which has to follow the on-screen
        // size or a globe filling a 4K window costs forty texture taps in every
        // one of eight million pixels.
        let steps = (10.0 + 30.0 * ((earth_px - 60.0) / 500.0).clamp(0.0, 1.0)).round();
        s.draws.push(draw(Prim::CloudVolume, CLOUD_TOP_KM, CLOUD_TOP_KM - CLOUD_BOTTOM_KM, steps));
        return fade;
    }

    // Bottom-up, so the premultiplied "over" blend composites back to front for
    // the near hemisphere — the far one is behind the opaque planet and never
    // reaches the blender. Getting this backwards puts the deck's underside on
    // top of its anvils.
    let n = cloud_shell_count(earth_px);
    for k in 0..n {
        let alt = cloud_altitude_km(k, n);
        // The slab of air this shell stands for, as with the aurora: it is what
        // keeps the deck's total opacity independent of how many were drawn.
        let lo = if k == 0 { alt } else { cloud_altitude_km(k - 1, n) };
        let hi = if k + 1 == n { alt } else { cloud_altitude_km(k + 1, n) };
        s.draws.push(draw(Prim::Cloud, alt, ((hi - lo) * 0.5).max(0.05), 0.0));
    }
    fade
}

/// The lightning alight at `unix_s`.
///
/// A pure function of the clock, with no state carried between frames. Two
/// things fall out of that. Scrubbing time backwards replays the same storm
/// rather than a fresh one, which is what the time chips promise; and the whole
/// model is testable, because the same instant always gives the same answer.
///
/// What is real here and what is not: [`clouds::ConvCell`] — where the storms
/// are, how big, how tall, and how often each should flash — is measured, out
/// of the infrared mosaic. The individual strokes are invented. No free
/// worldwide feed of real strikes exists, so the honest thing is to drive
/// synthetic flashes from real convection and say so.
fn lightning(b: &Bodies, cells: &[clouds::ConvCell], unix_s: f64, fade: f32) -> Flashes {
    /// How long one stroke stays visible. Real return strokes are under a
    /// millisecond; what the eye keeps is the afterglow through the cloud.
    const DECAY_S: f64 = 0.055;
    /// Nothing is worth carrying past this, and bounding it is what lets only
    /// two scheduling slots be examined per storm.
    const LIFE_S: f64 = 0.34;

    let mut out = Flashes {
        // Light dies off over roughly the size of the storm lighting it.
        reach: [b.earth_r * 0.035, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    let mut lit: Vec<([f32; 4], f32)> = Vec::new();

    for (i, c) in cells.iter().enumerate() {
        if c.flash_rate <= 0.0 {
            continue;
        }
        let period = 1.0 / c.flash_rate as f64;
        let slot = (unix_s / period).floor() as i64;
        // The slot before this one as well, or a flash that began just before a
        // boundary would be cut off at it.
        for k in [slot, slot - 1] {
            let h = hash2(i as u64, k as u64);
            // Fire somewhere inside the slot rather than on its edge: a storm
            // that ticked like a metronome would read as an animation, and real
            // flashes arrive in ragged bursts.
            let start = k as f64 * period + (h[0] as f64) * period;
            let age = unix_s - start;
            if !(0.0..LIFE_S).contains(&age) {
                continue;
            }
            // Most flashes are one stroke; a third of them flicker two or three
            // times over a tenth of a second, which is the thing that makes
            // lightning read as lightning rather than as a blinking light.
            let strokes = if h[1] < 0.68 {
                1
            } else if h[1] < 0.92 {
                2
            } else {
                3
            };
            let mut bright = 0.0f32;
            for stroke in 0..strokes {
                let dt = age - stroke as f64 * (0.04 + 0.05 * h[2] as f64);
                if dt >= 0.0 {
                    bright += (-dt / DECAY_S).exp() as f32;
                }
            }
            bright *= fade * (0.55 + 0.45 * h[3]);
            if bright < 0.02 {
                continue;
            }
            // Scattered across the storm, not all on its centre.
            let lat = c.lat + (h[4] - 0.5) * 2.0 * c.radius_deg * 0.7;
            let lon = c.lon + (h[5] - 0.5) * 2.0 * c.radius_deg * 0.7;
            // Half way up the tower. Above it the anvil lights from below and
            // under it the deck lights from above, which is what a storm seen
            // from orbit actually does.
            let alt = c.top_km * 0.5;
            let dir = b.surface_dir(lat as f64, lon as f64);
            let p = b.earth + dir * (b.earth_r * cloud_radius(alt));
            lit.push(([p.x, p.y, p.z, bright], bright));
        }
    }

    // Only the brightest few survive; the rest are already too faint to tell
    // apart on a globe.
    lit.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (slot, (item, _)) in out.items.iter_mut().zip(&lit) {
        *slot = *item;
    }
    out
}

/// Six independent values in 0..1 from two integers.
///
/// Deliberately not an RNG: there is no state to advance, so the same storm and
/// the same slot always give the same flash however the frames fall.
fn hash2(a: u64, k: u64) -> [f32; 6] {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ k.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let mut out = [0.0f32; 6];
    for o in &mut out {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        *o = (x >> 40) as f32 / 16_777_216.0;
    }
    out
}

/// A body is named while it is small enough on screen to need naming, and the
/// name gets out of the way once it is not.
///
/// The same rule for every body, the Earth included. A name is how you find
/// something that is a fraction of a pixel across — which, from anywhere in the
/// inner system, is what Neptune is — and it is pure clutter stamped across a
/// planet you have flown right up to and can obviously identify.
fn label_visible(px: f32, view_h: f32) -> bool {
    px < view_h * 0.18
}

/// How big a planet has to be on screen before its moons are named too.
const MOON_LABEL_PX: f32 = 26.0;

/// Names for every body, and the click targets that go with them.
fn body_labels(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera, view_h: f32, labels: bool) {
    // `force` is for a name that was asked for by name: a find-box match, or the
    // body the camera is on. Those go through with the LABELS chip off, exactly
    // as a matching satellite's does — withholding the one name somebody typed
    // reads as the search being broken rather than as the display being tidy.
    let mut add = |pos: V3,
                   radius: f32,
                   name: &str,
                   color: Color32,
                   focus: Focus,
                   show: bool,
                   force: bool| {
        let px = cam.pixels_for(pos, radius);
        // Everything named is also clickable, and the grab area never shrinks
        // below something a pointer can actually hit.
        s.picks.push(Pick { world: pos.arr(), radius_px: px.clamp(7.0, 400.0), focus });
        if !show || !(labels || force) || !label_visible(px, view_h) {
            return;
        }
        s.labels.push(Label {
            world: pos.arr(),
            text: name.to_string(),
            color: if st.focus() == focus { ink::CYAN } else { color },
            offset: [10.0, -6.0],
            click: Click::Focus(focus),
            rank: RANK_ALWAYS,
        });
    };

    // Our own Moon follows the same rule as everybody else's: it is named once
    // the Earth is big enough on screen for the two names not to sit on top of
    // one another, and from further out the Earth's label stands for the pair.
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    for (pos, radius, name, focus, show) in [
        (V3::ZERO, b.sun_r, "SUN", Focus::Sun, true),
        (b.earth, b.earth_r, "EARTH", Focus::Earth, true),
        (b.moon, b.moon_r, "MOON", Focus::Moon, earth_px > MOON_LABEL_PX),
    ] {
        add(pos, radius, name, ink::CYAN_DIM, focus, show, false);
    }

    if !st.layer(layer::PLANETS) {
        return;
    }
    for p in &b.planets {
        let color = planet_look(p.planet).base;
        let name = p.planet.name().to_uppercase();
        add(p.pos, p.radius, &name, color, Focus::Planet(p.planet), true, false);
    }
    for m in &b.moons {
        // Same rule for every other moon.
        let parent = &b.planets[m.parent];
        let show = cam.pixels_for(parent.pos, parent.radius) > MOON_LABEL_PX;
        let color = moon_look(m.info).base;
        let name = m.info.name.to_uppercase();
        add(m.pos, m.radius, &name, color, Focus::Satellite(m.index), show, false);
    }

    // Small bodies. The dwarf planets are named like planets, because that is
    // what they are to the eye. The asteroids and comets are behind their own
    // chip, and within it a name still has to earn its place: naming all
    // thirty-five at once buries the planets under a thicket of designations.
    // So the chip is the ceiling and these are the reasons — it is a comet with
    // its tail up, which is an event rather than a fixture, or you have flown
    // close enough that it is a thing in front of you rather than one of forty
    // dots.
    //
    // Two cases go through regardless of the chip: what the find box matched,
    // and what the camera is pointed at. Both were asked for *by name*, and a
    // search that highlights something and then withholds its name reads as
    // broken rather than as tidy.
    let focus = st.focus();
    let small_names = st.layer(layer::SMALL_LABELS);
    for sb in &b.smalls {
        let hit = st.small_hit(sb.info);
        let active = sb.tails.is_some();
        let dwarf = sb.info.class == sdroxide_solar::SmallClass::Dwarf;
        let targeted = focus == Focus::Small(sb.index);
        let worth_it = active || (cam.eye - sb.pos).len() < NEARBY_GM;
        let show = hit || targeted || dwarf || (small_names && worth_it);
        let color = if hit { ink::YELLOW } else { small_color(sb.info, active) };
        add(sb.pos, sb.radius, sb.info.name, color, Focus::Small(sb.index), show, hit || targeted);
    }
}

/// How close the camera has to be for a small body to name itself, gigametres.
///
/// A third of an AU: near enough that you have gone looking for it, far enough
/// that the name arrives before the body resolves into anything.
const NEARBY_GM: f32 = 50.0;

/// Fade a colour's alpha for a label, staying in sRGB (egui's space) rather
/// than the linear space the shaders want.
fn lin_color(c: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (255.0 * alpha.clamp(0.0, 1.0)) as u8)
}

/// Where a sunspot region is *now*, in Stonyhurst longitude.
///
/// Uses the Carrington longitude rather than propagating the observed
/// Stonyhurst one: Carrington longitude is fixed to the rotating Sun, so
/// `L_stonyhurst = L_carrington − L0(now)` is exact for any age of report and
/// needs no differential-rotation model. (The residual drift from differential
/// rotation is a degree or two over the life of a region.)
fn region_longitude_now(region: &sdroxide_solar::ActiveRegion, jd: f64) -> f64 {
    let (_, _, l0) = ephem::solar_p_b0_l0(jd);
    ephem::wrap180(region.carrington_deg - l0)
}

/// Sunspot active regions, from the NOAA daily summary.
///
/// Drawn on the photosphere so the depth buffer hides the far-side ones for
/// free — a marker behind the opaque Sun simply fails the depth test.
fn spots(
    s: &mut Scene,
    b: &Bodies,
    cam: &Camera,
    regions: &[sdroxide_solar::ActiveRegion],
    _now: i64,
) {
    let sun_px = cam.pixels_for(V3::ZERO, b.sun_r);
    // Below this the disk is too small to place anything on meaningfully.
    let fade = ((sun_px - 12.0) / 30.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    for r in regions {
        let lon = region_longitude_now(r, b.jd);
        let dir = V3::from_f64(b.sun_frame.direction(r.lat_deg, lon));
        // Just clear of the surface, so it is not z-fighting the sphere.
        let pos = dir * (b.sun_r * 1.004);
        let radius_px = cam.pixels_for(pos, b.sun_r * r.angular_radius() as f32);
        // Colour by NOAA's own flare probability: the regions worth watching
        // are the ones likely to produce something.
        let t = r.threat() as f32;
        let color = if t > 0.5 {
            ink::PINK
        } else if t > 0.2 {
            ink::YELLOW
        } else {
            Color32::from_rgb(0x30, 0x20, 0x14)
        };
        s.sprites.push(SpriteInst {
            center: pos.arr(),
            size_px: (radius_px * 2.0).clamp(3.5, 60.0),
            color: lin(color, (0.55 + 0.45 * t) * fade),
            params: [SPRITE_DOT, 0.0, 0.0, 0.0],
        });
    }
}

/// Recent flares, marked at their source region and fading out over a day.
fn flares(
    s: &mut Scene,
    b: &Bodies,
    cam: &Camera,
    events: &[sdroxide_solar::FlareEvent],
    now: i64,
) {
    const FADE_S: f64 = 86_400.0;
    let sun_px = cam.pixels_for(V3::ZERO, b.sun_r);
    if sun_px < 12.0 {
        return;
    }
    for f in events {
        let Some((lat, lon0)) = f.location else { continue };
        let age = (now - f.peak_unix) as f64;
        if !(0.0..FADE_S).contains(&age) {
            continue;
        }
        // The region has rotated since the flare; carry it forward at the
        // synodic rate for its latitude.
        let days = age / 86_400.0;
        let lon = lon0
            + (ephem::sidereal_rotation_deg_per_day(lat) - ephem::EARTH_MEAN_MOTION_DEG_PER_DAY)
                * days;
        let dir = V3::from_f64(b.sun_frame.direction(lat, ephem::wrap180(lon)));
        let alpha = (1.0 - age / FADE_S) as f32;
        let sev = f.severity() as f32;
        s.sprites.push(SpriteInst {
            center: (dir * (b.sun_r * 1.01)).arr(),
            size_px: 8.0 + 7.0 * (sev - 2.0).clamp(0.0, 2.5),
            color: lin(ink::PINK, (0.35 + 0.55 * alpha).min(1.0)),
            params: [SPRITE_RING, 0.0, 0.0, 0.0],
        });
    }
}

/// CME trajectory cones.
///
/// The apex sits at the Sun and the leading edge at `speed × elapsed`, so the
/// picture is a direct read-out of where the plasma actually is — and whether
/// the Earth is inside the cone.
fn cones(s: &mut Scene, st: &SolarUi, cmes: &[sdroxide_solar::CmeEvent], now: i64) {
    let window = (st.view.cme_window_h as f64 * 3600.0) as i64;
    for e in cmes {
        let Some(a) = &e.analysis else { continue };
        let age = now - a.t21_5_unix;
        if !(0..window).contains(&age) {
            continue;
        }
        // The front never precedes the launch height, and a week-old event is
        // stopped before it stretches out past the outer planets.
        let launch = sdroxide_solar::impact::LAUNCH_RADIUS;
        let length = sdroxide_solar::impact::front_distance(a, now)
            .clamp(launch * 1.08, 1.8 * sdroxide_solar::AU);
        let axis = V3::from_f64(sdroxide_solar::impact::axis(a));
        let (x, y) = perpendicular_basis(axis);

        // Speed, over the range CMEs actually occupy: the slow majority sit
        // near 400 km/s and anything past ~1200 is a fast, geoeffective event.
        // A wider ramp would render almost every cone the same colour.
        let fast = ((a.speed_km_s - 350.0) / 850.0).clamp(0.0, 1.0) as f32;
        let earth_bound = sdroxide_solar::earth_impact(a).is_some();
        let color = if earth_bound {
            // The one that matters gets the alarm colour, whatever its speed.
            ink::PINK
        } else {
            Color32::from_rgb(
                (fast * 255.0) as u8,
                (0xd0 as f32 - fast * 0x50 as f32) as u8,
                (0xf4 as f32 - fast * 0x74 as f32) as u8,
            )
        };
        // Fade with age so the display does not silt up with old events.
        let mut alpha = (1.0 - age as f32 / window as f32).clamp(0.15, 1.0) * 0.5;
        // An estimated axis (from `sourceLocation` rather than a fit) is a
        // coarser number, and is drawn fainter to say so.
        if a.estimated {
            alpha *= 0.55;
        }
        if earth_bound {
            alpha = (alpha * 1.6).min(0.95);
        }

        s.draws.push((
            Prim::Cone,
            DrawData {
                model: M4::from_basis(x, y, axis, V3::ZERO, length as f32).cols,
                basis: M4::from_basis(x, y, axis, V3::ZERO, 1.0).cols,
                tint: lin(color, 1.0),
                tint2: [0.0; 4],
                params: [
                    0.0,
                    (a.half_angle_deg as f32).to_radians(),
                    alpha,
                    (launch / length) as f32,
                ],
                style: [0.0; 4],
            },
        ));
    }
}

/// Any orthonormal pair perpendicular to `z`. The choice is arbitrary — the
/// cone is rotationally symmetric about its axis — but it must be numerically
/// stable, hence picking the seed axis `z` is least aligned with.
fn perpendicular_basis(z: V3) -> (V3, V3) {
    let seed = if z.z.abs() < 0.9 { v3(0.0, 0.0, 1.0) } else { v3(1.0, 0.0, 0.0) };
    let x = seed.cross(z).normalize();
    (x, z.cross(x))
}

fn bodies_draws(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera) {
    let sun_basis = basis_cols(b.sun_frame.basis);
    let sphere = |s: &mut Scene, (x, y, z): (V3, V3, V3), pos, r, tint, mode, style| {
        let mut d = DrawData::new(
            M4::from_basis(x, y, z, pos, r),
            M4::from_basis(x, y, z, V3::ZERO, 1.0),
            tint,
            mode,
        );
        d.style = style;
        s.draws.push((Prim::Sphere, d));
    };

    // Sun.
    let sun = Color32::from_rgb(0xff, 0xc4, 0x6a);
    sphere(s, sun_basis, V3::ZERO, b.sun_r, sun, MODE_SUN, [0.0; 4]);

    // Earth — its body frame is ECEF, so the land mask, the QTH marker and the
    // terminator all share one coordinate system. `tint2` carries the Moon's
    // true geocentric offset, from which the shader casts the eclipse shadow.
    let (ex, ey, ez) = b.earth_basis;
    let mut earth = DrawData::new(
        M4::from_basis(ex, ey, ez, b.earth, b.earth_r),
        M4::from_basis(ex, ey, ez, V3::ZERO, 1.0),
        ink::CYAN,
        MODE_EARTH,
    );
    earth.tint2 = b.moon_true_off.arr4(0.0);
    s.draws.push((Prim::Sphere, earth));

    // Moon, in the tidally locked frame — which is what puts Imbrium and
    // Tranquillitatis on the near side, where they belong. `tint2` carries the
    // same true geocentric offset the Earth draw gets, plus the orbit
    // compression in `w`: from those and its own rendered centre the shader
    // recovers the true geometry and casts the Earth's shadow back onto the
    // Moon — the lunar eclipse.
    let (mx, my, mz) = b.moon_basis;
    let mut moon = DrawData::new(
        M4::from_basis(mx, my, mz, b.moon, b.moon_r),
        M4::from_basis(mx, my, mz, V3::ZERO, 1.0),
        Color32::from_rgb(0x9a, 0xa4, 0xb4),
        MODE_MOON,
    );
    moon.tint2 = b.moon_true_off.arr4(st.view.moon_orbit_scale);
    moon.style = [0.0, MAP_MOON, 0.0, 0.0];
    s.draws.push((Prim::Sphere, moon));

    if st.layer(layer::PLANETS) {
        for p in &b.planets {
            body_sphere(s, p.basis, p.pos, p.radius, planet_look(p.planet));
        }
        for m in &b.moons {
            body_sphere(s, moon_facing_basis(b, m), m.pos, m.radius, moon_look(m.info));
        }
        // Rings last of all the bodies: they are transparent, so they have to
        // blend over whatever is already there — and keeping them in one run
        // costs a single pipeline switch.
        for p in &b.planets {
            rings(s, p);
        }
    }

    if st.layer(layer::PLANETS) {
        small_bodies(s, b, cam);
    }

    // A glow billboard with a pixel floor under every body, so "can I see the
    // Earth from 2 AU" never depends on the exaggeration slider.
    glow(s, cam, V3::ZERO, b.sun_r, 22.0, Color32::from_rgb(0xff, 0xd0, 0x80));
    glow(s, cam, b.earth, b.earth_r, 7.0, ink::CYAN);
    glow(s, cam, b.moon, b.moon_r, 5.0, Color32::from_rgb(0xc8, 0xd2, 0xe0));
    if st.layer(layer::PLANETS) {
        for p in &b.planets {
            glow(s, cam, p.pos, p.radius, 6.0, planet_look(p.planet).base);
        }
        for m in &b.moons {
            // A moon's glow only once its planet is big enough on screen for
            // the two to be told apart; further out the planet's own glow
            // stands for the whole system.
            if cam.pixels_for(b.planets[m.parent].pos, b.planets[m.parent].radius) > 6.0 {
                glow(s, cam, m.pos, m.radius, 3.0, moon_look(m.info).base);
            }
        }
    }
}

/// The look of a small body: what it is drawn in, and what its label takes.
///
/// Split by class rather than given per body, because the classes are what the
/// eye needs to tell apart: an icy dwarf planet, a rock, and something with a
/// tail. Within a class the bodies differ by where they are, which is the whole
/// point of the picture.
fn small_look(b: &sdroxide_solar::SmallBody) -> Look {
    use sdroxide_solar::SmallClass as C;
    let mut look = look_of(match b.class {
        // Bright, ancient ice with tholin staining — the far ones are
        // photometrically red, and Pluto very much so.
        C::Dwarf => sdroxide_solar::Surface::Icy,
        C::Asteroid | C::Comet => sdroxide_solar::Surface::Cratered,
    });
    if b.class == C::Dwarf {
        look.base = Color32::from_rgb(0xd8, 0xc4, 0xae);
        look.second = Color32::from_rgb(0x8a, 0x6a, 0x58);
    }
    look
}

/// Marker colour for a small body — what the sprite and the label take.
fn small_color(b: &sdroxide_solar::SmallBody, active: bool) -> Color32 {
    use sdroxide_solar::SmallClass as C;
    match b.class {
        C::Dwarf => Color32::from_rgb(0xd8, 0xc4, 0xae),
        C::Asteroid => Color32::from_rgb(0xa8, 0x9a, 0x86),
        // A comet with its tails up is the one thing in this layer that is an
        // *event* rather than a fixture, so it is the one thing given the
        // palette's live colour.
        C::Comet if active => ink::CYAN,
        C::Comet => Color32::from_rgb(0x8a, 0x9a, 0xa8),
    }
}

/// Dwarf planets, asteroids and comets: the body itself, its tails, and the
/// marker that keeps it visible when it is a millionth of a pixel across.
fn small_bodies(s: &mut Scene, b: &Bodies, cam: &Camera) {
    for sb in &b.smalls {
        // Big enough to shade is a small club — the five dwarf planets at any
        // useful zoom, and the larger asteroids once you have flown to them.
        if cam.pixels_for(sb.pos, sb.radius) > SPHERE_MIN_PX {
            let basis = (v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0));
            body_sphere(s, basis, sb.pos, sb.radius, small_look(sb.info));
        }
        if let Some(t) = sb.tails {
            comet_tails(s, sb, &t);
        }

        // The marker. A dwarf planet gets a planet's floor; an asteroid gets
        // less, because there are thirty of them and they are not events.
        let color = small_color(sb.info, sb.tails.is_some());
        let floor = match sb.info.class {
            sdroxide_solar::SmallClass::Dwarf => 6.0,
            _ if sb.tails.is_some() => 5.0,
            _ => 3.2,
        };
        glow(s, cam, sb.pos, sb.radius, floor, color);

        // ...and on an active comet, the coma over the top of it: a hundred
        // thousand kilometres of fluorescing gas, which is the part of a comet
        // that is actually bright.
        if let Some(t) = sb.tails {
            let px = cam.pixels_for(sb.pos, t.coma_gm as f32);
            let a = (t.activity as f32).clamp(0.0, 1.0);
            s.sprites.push(SpriteInst {
                center: sb.pos.arr(),
                size_px: (px * 2.0).clamp(4.0, 900.0),
                // Green: C₂ and CN fluorescing, which is why a bright comet's
                // head photographs green while its ion tail is blue.
                color: lin(Color32::from_rgb(0x9c, 0xf0, 0xc0), 0.16 + 0.5 * a),
                params: [SPRITE_GLOW, 0.0, 0.0, 0.0],
            });
        }
    }
}

/// A comet's two tails.
///
/// Dust first, then ions: they overlap near the nucleus, both add light, and
/// the ion tail is the one that should read as being in front.
fn comet_tails(s: &mut Scene, sb: &SmallPlaced, t: &sdroxide_solar::smallbody::Tails) {
    let activity = t.activity as f32;
    let ion = V3::from_f64(t.ion);
    let lag = V3::from_f64(t.lag);

    // Dust first: the broader, fainter, curved one.
    if t.dust_gm > 0.0 {
        plume(
            s,
            sb.pos,
            ion,
            lag,
            t.dust_gm as f32,
            // Wider than the ion tail and blunter at the head — dust leaves the
            // coma in every direction and is only then pushed back.
            (t.coma_gm as f32) * 2.4,
            // How far it bows away from the anti-solar line by the tip. Real
            // dust tails curve tens of degrees; this is a fraction of the
            // length, applied as the square of the distance out.
            0.42,
            TAIL_DUST,
            activity,
            Color32::from_rgb(0xf0, 0xdc, 0xa8),
            Color32::from_rgb(0xb4, 0x8c, 0x50),
        );
    }
    // Then the ion tail: narrow, straight, and blue, because the light is CO⁺
    // fluorescing at 420 nm and not reflected sunlight at all.
    if t.ion_gm > 0.0 {
        plume(
            s,
            sb.pos,
            ion,
            lag,
            t.ion_gm as f32,
            (t.coma_gm as f32) * 0.9,
            0.0,
            TAIL_ION,
            activity,
            Color32::from_rgb(0x7c, 0xb4, 0xff),
            Color32::from_rgb(0x40, 0x70, 0xd8),
        );
    }
}

/// One tail, as a screen-facing ribbon streaming away from the nucleus.
///
/// `axis` is the unit direction it runs along and `bend` the unit direction it
/// curves towards; the pair are perpendicular, so they and their cross product
/// make the frame the shader shapes the plume in.
#[allow(clippy::too_many_arguments)]
fn plume(
    s: &mut Scene,
    pos: V3,
    axis: V3,
    bend: V3,
    length: f32,
    width: f32,
    curve: f32,
    kind: f32,
    activity: f32,
    near: Color32,
    far: Color32,
) {
    let mut d = DrawData::new(
        // No scale: the shader works in gigametres along the frame's axes, so
        // one mesh serves a 40 Gm ion tail and a 4 Gm dust one.
        M4::from_basis(bend, axis.cross(bend), axis, pos, 1.0),
        M4::from_basis(bend, axis.cross(bend), axis, V3::ZERO, 1.0),
        near,
        kind,
    );
    d.tint2 = lin(far, 1.0);
    d.params = [kind, length, width, curve];
    d.style = [activity, 0.0, 0.0, 0.0];
    s.draws.push((Prim::Tail, d));
}

/// A billboard under a body, never smaller than `min_px` across.
fn glow(s: &mut Scene, cam: &Camera, pos: V3, radius: f32, min_px: f32, color: Color32) {
    let px = cam.pixels_for(pos, radius);
    s.sprites.push(SpriteInst {
        center: pos.arr(),
        size_px: (px * 2.6).max(min_px),
        color: lin(color, if px > min_px * 0.5 { 0.35 } else { 0.9 }),
        params: [SPRITE_GLOW, 0.0, 0.0, 0.0],
    });
}

/// One of the procedurally shaded bodies: a planet or one of their moons.
fn body_sphere(s: &mut Scene, (x, y, z): (V3, V3, V3), pos: V3, radius: f32, look: Look) {
    let mut d = DrawData::new(
        M4::from_basis(x, y, z, pos, radius),
        M4::from_basis(x, y, z, V3::ZERO, 1.0),
        look.base,
        MODE_BODY,
    );
    d.tint2 = lin(look.second, 1.0);
    d.style = [look.style, look.detail, look.two_tone, look.haze];
    s.draws.push((Prim::Sphere, d));
}

/// A moon's body frame. Every moon this view draws is tidally locked, so — as
/// with our own — the near side faces the planet and the frame follows from
/// the geometry rather than from a rotation rate.
fn moon_facing_basis(b: &Bodies, m: &MoonBody) -> (V3, V3, V3) {
    let parent = &b.planets[m.parent];
    let to_parent = (parent.pos - m.pos).normalize();
    let pole = parent.basis.2;
    let z = (pole - to_parent * pole.dot(to_parent)).normalize();
    (to_parent, z.cross(to_parent), z)
}

/// A planet's rings, as a flat annulus in its equatorial plane.
fn rings(s: &mut Scene, p: &PlanetBody) {
    let Some((inner, outer, opacity)) = p.rings else { return };
    let (x, y, z) = p.basis;
    let mut d = DrawData::new(
        // Scaled to the outer edge; the shader carries the inner one as a
        // fraction, so one unit annulus mesh serves every ring system.
        M4::from_basis(x, y, z, p.pos, outer),
        M4::from_basis(x, y, z, V3::ZERO, 1.0),
        Color32::from_rgb(0xe8, 0xdc, 0xc0),
        0.0,
    );
    d.tint2 = lin(Color32::from_rgb(0xa9, 0x95, 0x78), 1.0);
    // x = inner edge as a fraction of the outer, y = opacity, z = the planet's
    // radius in the same units (for the shadow it casts on them), w = which
    // radial profile: Saturn's broad sheet, or Uranus's handful of threads.
    let narrow = if opacity < 0.5 { 1.0 } else { 0.0 };
    d.params = [inner / outer, opacity, p.radius / outer, narrow];
    s.draws.push((Prim::Ring, d));
}

/// Orbital paths, sampled from the same ephemeris that places the bodies — so
/// the ring is the real (eccentric) orbit rather than an idealised circle.
fn orbits(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera) {
    const EARTH_STEPS: usize = 256;
    let mut prev = V3::from_f64(ephem::earth_heliocentric(b.jd));
    for k in 1..=EARTH_STEPS {
        let jd = b.jd + 365.256_363 * k as f64 / EARTH_STEPS as f64;
        let p = V3::from_f64(ephem::earth_heliocentric(jd));
        s.lines.push(seg(prev, p, 1.6, lin(ink::CYAN_DIM, 0.55)));
        prev = p;
    }

    // The Moon's path is drawn around the Earth's *current* position, so it
    // reads as a ring on the Earth rather than a smear along the Earth's orbit.
    const MOON_STEPS: usize = 128;
    let moon_at =
        |jd: f64| b.earth + V3::from_f64(ephem::moon_geocentric_vec(jd)) * st.view.moon_orbit_scale;
    let mut prev = moon_at(b.jd);
    for k in 1..=MOON_STEPS {
        let jd = b.jd + 27.321_661 * k as f64 / MOON_STEPS as f64;
        let p = moon_at(jd);
        s.lines.push(seg(prev, p, 1.3, lin(ink::LINE_LIT, 0.7)));
        prev = p;
    }

    if !st.layer(layer::PLANETS) {
        return;
    }
    // The other planets. Sampled over one of their own years, so Neptune's
    // ring is as smooth as Mercury's rather than 165 times coarser.
    const PLANET_STEPS: usize = 192;
    for p in &b.planets {
        let period = p.planet.info().orbit_days();
        let at = |jd: f64| V3::from_f64(p.planet.heliocentric(jd));
        let mut prev = at(b.jd);
        for k in 1..=PLANET_STEPS {
            let q = at(b.jd + period * k as f64 / PLANET_STEPS as f64);
            s.lines.push(seg(prev, q, 1.2, lin(ink::CYAN_DIM, 0.3)));
            prev = q;
        }
    }

    // Small bodies. The dwarf planets get a ring as a matter of course — they
    // are planets in every sense the eye cares about. The asteroids and comets
    // do not: thirty-five ellipses, half of them steeply inclined and crossing
    // everything, is a ball of wool rather than a diagram. Theirs appears when
    // it has been asked for, by search or by pointing the camera at it, and
    // then it is the only one on screen and says something.
    const SMALL_STEPS: usize = 192;
    let focus = st.focus();
    for sb in &b.smalls {
        let hit = st.small_hit(sb.info);
        let dwarf = sb.info.class == sdroxide_solar::SmallClass::Dwarf;
        if !(hit || dwarf || focus == Focus::Small(sb.index)) {
            continue;
        }
        let (w, alpha, color) = if hit {
            (1.8, 0.75, ink::YELLOW)
        } else if dwarf {
            (1.2, 0.28, ink::CYAN_DIM)
        } else {
            (1.5, 0.55, small_color(sb.info, sb.tails.is_some()))
        };
        let mut path = sb.info.orbit_path(b.jd, SMALL_STEPS).map(V3::from_f64);
        let mut prev = path.next().unwrap_or(V3::ZERO);
        for q in path {
            s.lines.push(seg(prev, q, w, lin(color, alpha)));
            prev = q;
        }
    }

    // Moon paths, only for a planet big enough on screen for a ring around it
    // to be a ring rather than a smudge.
    const SAT_STEPS: usize = 64;
    for m in &b.moons {
        let parent = &b.planets[m.parent];
        if cam.pixels_for(parent.pos, parent.radius) < 10.0 {
            continue;
        }
        let scale = parent.exaggeration * st.view.moon_orbit_scale;
        let at = |jd: f64| parent.pos + V3::from_f64(m.info.offset(jd)) * scale;
        let mut prev = at(b.jd);
        for k in 1..=SAT_STEPS {
            let q = at(b.jd + m.info.period_d * k as f64 / SAT_STEPS as f64);
            s.lines.push(seg(prev, q, 1.0, lin(ink::LINE_LIT, 0.5)));
            prev = q;
        }
    }
}

/// Solar rotation axis and equator, and the ecliptic plane reference ring.
fn grid(s: &mut Scene, b: &Bodies) {
    let n = V3::from_f64(b.sun_frame.basis.z);
    s.lines.push(seg(n * (-b.sun_r * 1.6), n * (b.sun_r * 1.6), 1.4, lin(ink::YELLOW, 0.55)));

    const RING: usize = 96;
    let mut prev = V3::from_f64(b.sun_frame.direction(0.0, 0.0)) * (b.sun_r * 1.004);
    for k in 1..=RING {
        let lon = 360.0 * k as f64 / RING as f64;
        let p = V3::from_f64(b.sun_frame.direction(0.0, lon)) * (b.sun_r * 1.004);
        s.lines.push(seg(prev, p, 1.2, lin(ink::YELLOW, 0.4)));
        prev = p;
    }

    // Heliographic parallels every 30°, so latitude is readable on the disk.
    for lat in [-60.0, -30.0, 30.0, 60.0] {
        let mut prev = V3::from_f64(b.sun_frame.direction(lat, 0.0)) * (b.sun_r * 1.004);
        for k in 1..=RING / 2 {
            let lon = 360.0 * k as f64 / (RING / 2) as f64;
            let p = V3::from_f64(b.sun_frame.direction(lat, lon)) * (b.sun_r * 1.004);
            s.lines.push(seg(prev, p, 1.0, lin(ink::YELLOW, 0.22)));
            prev = p;
        }
    }
}

/// The operator's QTH and the sub-solar point.
///
/// Both are *positions on the Earth's surface*, so they only mean anything once
/// the Earth is big enough on screen to place them on. Below that they are
/// faded out entirely — a 13 px ring around a 2 px Earth reads as a property of
/// the planet rather than of a location on it.
fn markers(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera) {
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    let fade = ((earth_px - 3.0) / 9.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    // Never wider than the planet it sits on.
    let size = |base: f32| base.min(earth_px * 1.5);

    let (ex, ey, ez) = b.earth_basis;
    let on_earth = |lat: f64, lon: f64, lift: f32| {
        let d = ephem::geodetic_to_body(lat, lon);
        b.earth + (ex * d.x as f32 + ey * d.y as f32 + ez * d.z as f32) * (b.earth_r * lift)
    };

    if let Some((lat, lon)) = st.qth {
        s.sprites.push(SpriteInst {
            center: on_earth(lat, lon, 1.02).arr(),
            size_px: size(14.0),
            color: lin(ink::GREEN, 0.95 * fade),
            params: [SPRITE_RING, 0.0, 0.0, 0.0],
        });
    }

    let (slat, slon) = ephem::subsolar_point(b.jd);
    s.sprites.push(SpriteInst {
        center: on_earth(slat, slon, 1.02).arr(),
        size_px: size(9.0),
        color: lin(ink::YELLOW, 0.85 * fade),
        params: [SPRITE_DOT, 0.0, 0.0, 0.0],
    });
}

fn seg(a: V3, b: V3, width_px: f32, color: [f32; 4]) -> LineInst {
    LineInst { a: a.arr(), width_px, b: b.arr(), _pad: 0.0, color }
}

/// The most history arcs the time-lapse draws at once.
///
/// An hour of a busy 20 m evening is a few thousand decodes. Past a couple of
/// hundred arcs the globe is a ball of wool in which nothing can be read, so
/// the newest win — the trail is already a fade, and this is where it ends.
const MAX_LAPSE_ARCS: usize = 160;
/// Segments per history arc. The contact being worked gets the full [`arc`];
/// these are many and short-lived, so they trade smoothness for count.
const LAPSE_ARC_STEPS: usize = 28;
/// History arcs bow lower than the active one, so the QSO in progress stands
/// clear of the traffic behind it instead of being one strand among many.
const LAPSE_BULGE: f32 = 0.7;

/// How big the Earth has to be on screen — apparent radius, pixels — before
/// decoded stations are named.
///
/// Well above the threshold the dots themselves need. A dot means something on
/// a planet forty pixels across; a callsign beside it needs room for the *text*,
/// and below this the names cover the continents they are standing on. Above it
/// the overlay's declutter does the rest: pull in and more of them fit, which is
/// exactly what zooming in should get you.
const STATION_LABEL_MIN_PX: f32 = 90.0;

/// The most callsigns offered to the overlay at once.
///
/// The overlay drops the ones that would overlap, so this is not the number
/// drawn — it is a bound on the layout work done to find that out. A busy 20 m
/// evening puts several hundred stations up, and laying out every one of their
/// names to discard nine in ten is work no frame needs to do.
const MAX_STATION_LABELS: usize = 72;

/// Is a point on the Earth's surface turned towards the camera?
///
/// The dots get this for free from the depth buffer — a marker behind the globe
/// simply fails the depth test. The labels do not: they are painted by egui
/// after the 3D pass has finished, so without this every station on the far side
/// of the world would have its callsign written across the near side.
///
/// The margin keeps a name off the silhouette itself, where it reads as
/// belonging to neither hemisphere.
fn faces_camera(cam: &Camera, dir: V3, pos: V3) -> bool {
    dir.dot((cam.eye - pos).normalize()) > 0.06
}

/// Name a station standing on the Earth's surface, if it can be seen from here.
fn station_label(
    s: &mut Scene,
    b: &Bodies,
    cam: &Camera,
    (lat, lon): (f64, f64),
    call: &str,
    color: Color32,
    rank: u8,
) {
    let dir = b.surface_dir(lat, lon);
    // Just clear of the dot's own lift, so the text is not z-fighting with the
    // marker it belongs to.
    let pos = b.earth + dir * (b.earth_r * 1.02);
    if !faces_camera(cam, dir, pos) {
        return;
    }
    s.labels.push(Label {
        world: pos.arr(),
        text: call.to_string(),
        color,
        // Tighter than a body's: this label belongs to a 4 px dot, and at arm's
        // length from it the eye stops pairing the two.
        offset: [7.0, -6.0],
        click: Click::None,
        rank,
    });
}

/// Decoded stations, the last hour of them, and the path to the one being
/// worked. Every mode that places stations feeds this — FT8 and FT4, JS8, WSPR,
/// FSQ — through the one [`crate::digi_map::DigiStations`].
///
/// Close in, each dot is named with the callsign that was heard there, which is
/// the question the map is being asked: not "is anyone out there" — the dots
/// already answer that — but *who*. It is why the callsign is carried all the
/// way from the decoder rather than dropped at the grid square.
///
/// The flat map in the FT8 panel draws the live set as a great-circle line
/// across a rectangle; here the path is the *actual* great circle, lifted off
/// the surface so it arcs through space between the two stations instead of
/// disappearing round the back of the globe.
///
/// The globe also draws what the panel map has no room for: every decode of the
/// last hour, as an arc that fades out behind a replay head the operator can
/// park anywhere in that hour (see [`SolarUi::lapse_head`]). Live, the head sits
/// at now and the trail is simply "recent activity"; wound back, the same
/// machinery is a time-lapse of the band.
fn digi_traffic(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera, now: f64, anim_t: f32) {
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // Below this the Earth is too small for a point on its surface to mean
    // anything, same threshold the QTH marker uses.
    let fade = ((earth_px - 3.0) / 9.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    let (ex, ey, ez) = b.earth_basis;
    let to_world = |v: sdroxide_solar::Vec3, lift: f32| {
        b.earth + (ex * v.x as f32 + ey * v.y as f32 + ez * v.z as f32) * (b.earth_r * lift)
    };

    // Whether there is room to write a callsign beside a dot. The LABELS chip
    // answers "do I want names at all", and a callsign is a name — an operator
    // who turned the planet names off did not mean "except for these".
    let names = st.layer(layer::LABELS) && earth_px > STATION_LABEL_MIN_PX;

    // The live dots are a statement about *now*, so they are drawn only while
    // the replay head is at now. Wound back, the history below is the whole
    // picture and a set of dots from the present would contradict it.
    if st.lapse_live() {
        for d in &st.digi.stations {
            if d.fade <= 0.0 {
                continue;
            }
            s.sprites.push(SpriteInst {
                center: to_world(ephem::geodetic_to_body(d.lat, d.lon), 1.015).arr(),
                size_px: (4.0 + 3.0 * d.fade).min(earth_px * 0.9),
                color: lin(ink::TEXT_STRONG, 0.85 * d.fade * fade),
                params: [SPRITE_DOT, 0.0, 0.0, 0.0],
            });
        }
        if names {
            // Freshest first, because both the cap below and the overlay's
            // declutter keep the head of this list — and what should survive a
            // crowded map is the station still on the air.
            let mut lit: Vec<&crate::digi_map::DigiStation> =
                st.digi.stations.iter().filter(|d| d.fade > 0.0 && d.call.is_some()).collect();
            lit.sort_by(|a, b| b.fade.total_cmp(&a.fade));
            for d in lit.into_iter().take(MAX_STATION_LABELS) {
                let Some(call) = d.call.as_deref() else { continue };
                station_label(
                    s,
                    b,
                    cam,
                    (d.lat, d.lon),
                    call,
                    // Held well clear of the floor as the dot fades: a name that
                    // dims with its dot becomes unreadable long before it
                    // disappears, and an unreadable name is worse than none.
                    lin_color(ink::TEXT_STRONG, (0.55 + 0.45 * d.fade) * fade),
                    Label::station_rank(d.fade),
                );
            }
        }
    }

    // The arcs need a home to start from.
    let Some(home) = st.qth else { return };

    lapse_arcs(s, st, b, cam, &to_world, home, earth_px, fade, now, anim_t, names);

    // The contact being worked, and the decode clicked but not yet answered.
    //
    // Green for the live path: it is the one arc on the globe that is *this*
    // station's, and green is already what the globe uses to say so — the QTH
    // ring under one end of it is the same colour. Everything else the QSO
    // layer draws is cyan cooling to violet, so the contact never has to
    // compete with the traffic for attention.
    for (target, color, width, active) in
        [(st.digi.dx, ink::GREEN, 5.2, true), (st.digi.preview, ink::YELLOW, 1.6, false)]
    {
        let Some(dx) = target else { continue };
        arc(
            s,
            b,
            &to_world,
            home,
            dx,
            color,
            width,
            fade,
            active.then_some(anim_t),
            st.digi.transmitting,
        );
    }

    // Where that arc peaks, for the card the overlay hangs there. The slerp
    // midpoint of the two ends, lifted by the full bulge — which is `arc_points`
    // at t = 0.5, where `sin(pi t)` is 1, evaluated without sampling the other
    // ninety-six points again.
    s.qso_apex = st.digi.dx.and_then(|dx| {
        let a = ephem::geodetic_to_body(home.0, home.1);
        let c = ephem::geodetic_to_body(dx.0, dx.1);
        let omega = a.dot(c).clamp(-1.0, 1.0).acos();
        let sum = a + c;
        // Same place, or exactly antipodal: the first has no arc, and the
        // second has no *unique* midpoint — every meridian through the two is
        // as good as the next, so there is no honest place to put the card.
        if omega < 1e-4 || sum.len() < 1e-5 {
            return None;
        }
        Some(to_world(sum.normalize(), 1.0 + arc_bulge(omega)).arr())
    });

    // Name the far end when it is a transmitter rather than a callsign: a
    // weather-fax or broadcast station. The arc already puts a ring on the spot,
    // so this only has to say what is standing there — "Nauen" or "Ascension
    // Island" is the whole point of drawing the path at all.
    //
    // Only while the transmitter is on the near side. The arc reaching it bows
    // out into space and stays visible over the horizon, but the name belongs to
    // a point on the ground — printed over a globe that station is behind, it
    // says the signal is coming from the wrong ocean.
    if let (Some(dx), Some(text)) = (st.digi.dx, st.digi.dx_label.as_ref()) {
        let dir = b.surface_dir(dx.0, dx.1);
        let pos = b.earth + dir * (b.earth_r * 1.02);
        if faces_camera(cam, dir, pos) {
            s.labels.push(Label {
                world: pos.arr(),
                text: text.clone(),
                // The colour of the arc it stands at the end of.
                color: lin_color(ink::GREEN, 0.95 * fade),
                offset: [10.0, -7.0],
                click: Click::None,
                rank: RANK_ALWAYS,
            });
        }
    }
}

/// The last hour of decodes as fading arcs, newest first.
///
/// Walking the history backwards is what makes the cap honest: what gets
/// dropped when a band is wide open is the oldest, faintest end of the trail,
/// which is the part already on its way out.
#[allow(clippy::too_many_arguments)]
fn lapse_arcs(
    s: &mut Scene,
    st: &SolarUi,
    b: &Bodies,
    cam: &Camera,
    to_world: &impl Fn(sdroxide_solar::Vec3, f32) -> V3,
    home: (f64, f64),
    earth_px: f32,
    fade: f32,
    now: f64,
    anim_t: f32,
    names: bool,
) {
    let head = st.lapse_head(now);
    let trail = st.lapse_trail_s();
    // Only the replay names its own stations. With the head at now the live set
    // above is the answer to "who is on the band", and labelling the trail as
    // well would print most callsigns twice — once for the dot and once for the
    // arc that arrived at it. Wound back, the trail is the only thing on screen,
    // and an unlabelled replay is a light show rather than a record.
    let names = names && !st.lapse_live();
    let mut drawn = 0usize;
    for hit in st.digi.history.iter().rev() {
        let age = head - hit.slot_utc as f64;
        if age < 0.0 {
            continue; // not yet decoded, as far as the replay head is concerned
        }
        if age > trail {
            break; // ordered, so everything further back is older still
        }
        let k = (1.0 - age / trail) as f32; // 1 at the head, 0 at the tail
        let alpha = k * k * fade;
        // Fresh traffic arrives cyan and cools to a dim violet as it ages out,
        // so "this minute" and "half an hour ago" are told apart by colour and
        // not only by how faint they are.
        let color = mix(LAPSE_OLD, ink::CYAN, k);

        let Some(pts) =
            arc_points(to_world, home, (hit.lat, hit.lon), LAPSE_ARC_STEPS, LAPSE_BULGE)
        else {
            continue;
        };
        // A spark runs the newest arcs from the station to us — the direction
        // the signal travelled. Only the freshest quarter of the trail gets
        // one, or every arc on the globe twinkles at once and none of it reads.
        let spark = ((k - 0.75) * 4.0).clamp(0.0, 1.0);
        // Offset per station so they do not march in lockstep.
        let jitter = (hit.slot_utc.rem_euclid(37) as f32) / 37.0;
        let sh = 1.0 - (anim_t * 0.4 + jitter).fract();
        for w in 0..pts.len() - 1 {
            let mid = (w as f32 + 0.5) / (pts.len() - 1) as f32;
            let d = (mid - sh).abs();
            let bright = 1.0 + 3.0 * spark * (-d * d * 260.0).exp();
            s.lines.push(seg(
                pts[w],
                pts[w + 1],
                1.1 * bright.min(2.6),
                lin(color, (alpha * bright).min(1.0)),
            ));
        }
        // The station itself, so a trail that has faded to a thread still says
        // where it lands. Lifted clear of the surface like the live dots, or it
        // z-fights with the globe it sits on.
        s.sprites.push(SpriteInst {
            center: to_world(ephem::geodetic_to_body(hit.lat, hit.lon), 1.015).arr(),
            size_px: (2.4 + 2.0 * k).min(earth_px * 0.6),
            color: lin(color, (0.9 * alpha).min(1.0)),
            params: [SPRITE_DOT, 0.0, 0.0, 0.0],
        });
        // Who it was. The arc's own colour, so a name reads as belonging to the
        // strand it sits on rather than to the one beside it, and its rank comes
        // from the same `k` — nearest the replay head wins the space.
        if names
            && drawn < MAX_STATION_LABELS
            && let Some(call) = hit.call.as_deref()
        {
            station_label(
                s,
                b,
                cam,
                (hit.lat, hit.lon),
                call,
                lin_color(color, (0.55 + 0.45 * k) * fade),
                Label::station_rank(k),
            );
        }

        drawn += 1;
        if drawn >= MAX_LAPSE_ARCS {
            break;
        }
    }
}

/// What a decode's arc has cooled to at the tail of the trail.
const LAPSE_OLD: Color32 = Color32::from_rgb(0x5a, 0x3c, 0xa8);

/// Award coverage: every DXCC entity, painted by what the logbook is missing.
///
/// A heat map of absence. An entity never worked burns orange-red and breathes;
/// one worked but unconfirmed is a quiet amber ring; a confirmed one is a dim
/// green dot that stays out of the way. Read from orbit, the hot patches are
/// where the log has holes — which is the question this layer answers, and the
/// reason the confirmed ones are drawn faintest rather than brightest.
///
/// The positions are cty.dat's nominal entity centres, so this is "the entity
/// is around there", not a border map. For an award chase that is exactly the
/// resolution wanted: the target is the entity, not a spot inside it.
/// The propagation heat: where signals are getting through, per band.
///
/// One sphere just clear of the surface. Everything that makes it a picture —
/// which bands, what colours, how they mix where two overlap — was resolved on
/// the CPU into the RGBA the shader samples, so this only has to decide *where*
/// and *how strongly*. See `solar_prop.wgsl`.
fn prop_heat(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera) {
    if st.prop.live_bands().is_empty() {
        return;
    }
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // A surface field is legible as soon as the surface is: the same threshold
    // the aurora uses, and far earlier than the awards layer's, which has three
    // hundred discrete markers to place rather than a continuous wash.
    let fade = ((earth_px - 3.0) / 12.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    let (ex, ey, ez) = b.earth_basis;
    s.draws.push((
        Prim::Prop,
        DrawData {
            // Just above the surface — below the QTH ring and the traffic arcs,
            // which is the order they should occlude in.
            model: M4::from_basis(ex, ey, ez, b.earth, b.earth_r * 1.004).cols,
            basis: M4::from_basis(ex, ey, ez, V3::ZERO, 1.0).cols,
            tint: [1.0; 4],
            tint2: [0.0; 4],
            params: [fade, 0.0, 0.0, 0.0],
            style: [0.0; 4],
        },
    ));
}

fn award_heat(s: &mut Scene, st: &SolarUi, b: &Bodies, cam: &Camera, anim_t: f32) {
    if st.awards.is_empty() {
        return;
    }
    let earth_px = cam.pixels_for(b.earth, b.earth_r);
    // A marker per entity only means something once the Earth is big enough to
    // place them on — and there are three hundred of them, so this layer needs
    // more room than a single QTH ring does before it is worth drawing.
    let fade = ((earth_px - 40.0) / 120.0).clamp(0.0, 1.0);
    if fade <= 0.0 {
        return;
    }
    let (ex, ey, ez) = b.earth_basis;
    let on_earth = |lat: f64, lon: f64, lift: f32| {
        let d = ephem::geodetic_to_body(lat, lon);
        b.earth + (ex * d.x as f32 + ey * d.y as f32 + ez * d.z as f32) * (b.earth_r * lift)
    };
    // One slow breath across the whole layer, so the missing entities read as
    // live heat rather than as a static scatter of dots.
    let beat = 1.0 + 0.18 * (anim_t * 1.6).sin();

    for slot in st.awards.iter() {
        let (color, size, glow) = match slot.coverage {
            Coverage::Missing => (AWARD_MISSING, 7.0 * beat, Some(22.0 * beat)),
            Coverage::Worked => (ink::YELLOW, 5.0, None),
            Coverage::Confirmed => (ink::GREEN, 3.5, None),
        };
        let alpha = match slot.coverage {
            Coverage::Missing => 0.95,
            Coverage::Worked => 0.7,
            Coverage::Confirmed => 0.45,
        };
        if let Some(g) = glow {
            s.sprites.push(SpriteInst {
                center: on_earth(slot.lat, slot.lon, 1.01).arr(),
                size_px: g.min(earth_px * 0.5),
                color: lin(color, 0.3 * fade),
                params: [SPRITE_GLOW, 0.0, 0.0, 0.0],
            });
        }
        s.sprites.push(SpriteInst {
            center: on_earth(slot.lat, slot.lon, 1.012).arr(),
            size_px: size.min(earth_px * 0.25),
            color: lin(color, alpha * fade),
            params: [
                if slot.coverage == Coverage::Confirmed { SPRITE_DOT } else { SPRITE_RING },
                0.0,
                0.0,
                0.0,
            ],
        });
    }
}

/// The heat of an entity that has never been worked.
const AWARD_MISSING: Color32 = Color32::from_rgb(0xff, 0x5a, 0x28);

/// Blend two palette colours in sRGB, `t` = 1 giving `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let c = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
}

/// How far an arc spanning `omega` radians bows off the surface, as a fraction
/// of the Earth's rendered radius.
///
/// The lift is proportional to the angular separation, so a short contact hugs
/// the surface and an antipodal one springs well clear of it — which is also
/// the only way both ends stay visible at once on a sphere.
///
/// The camera's contact framing reads this too: the shot it composes has to be
/// built around the arc that actually gets drawn, not a second guess at it.
pub fn arc_bulge(omega: f64) -> f32 {
    0.06 + 0.42 * (omega / std::f64::consts::PI) as f32
}

/// Sample the great-circle arc `from`→`to`, bowed `bulge_scale` × the standard
/// lift off the surface. Both ends land on the ground. `None` when the two
/// points are the same place, which has no arc.
fn arc_points(
    to_world: &impl Fn(sdroxide_solar::Vec3, f32) -> V3,
    from: (f64, f64),
    to: (f64, f64),
    steps: usize,
    bulge_scale: f32,
) -> Option<Vec<V3>> {
    let a = ephem::geodetic_to_body(from.0, from.1);
    let c = ephem::geodetic_to_body(to.0, to.1);
    let omega = a.dot(c).clamp(-1.0, 1.0).acos();
    if omega < 1e-4 {
        return None;
    }
    let bulge = arc_bulge(omega) * bulge_scale;
    Some(
        (0..=steps)
            .map(|k| {
                let t = k as f64 / steps as f64;
                // Spherical interpolation, so the path is the true great circle
                // rather than a chord through the planet.
                let s0 = ((1.0 - t) * omega).sin() / omega.sin();
                let s1 = (t * omega).sin() / omega.sin();
                let dir = a * s0 + c * s1;
                let lift = 1.0 + bulge * (std::f64::consts::PI * t).sin() as f32;
                to_world(dir.normalize(), lift)
            })
            .collect(),
    )
}

/// A great-circle arc between two points on the globe, bowed out into space.
///
/// With `anim` set this is the contact being worked, and it is drawn to be
/// unmistakable among an hour of traffic: a wide soft halo under a bright core,
/// a pulse running the path end to end, and rings on both stations. The pulse
/// runs whichever way the traffic is going — out to them while transmitting,
/// back to us the rest of the time — and hits far harder on transmit, which is
/// the one moment the operator wants to see at a glance from across the room.
#[allow(clippy::too_many_arguments)]
fn arc(
    s: &mut Scene,
    b: &Bodies,
    to_world: &impl Fn(sdroxide_solar::Vec3, f32) -> V3,
    from: (f64, f64),
    to: (f64, f64),
    color: Color32,
    width_px: f32,
    fade: f32,
    anim: Option<f32>,
    transmitting: bool,
) {
    const STEPS: usize = 96;
    let Some(pts) = arc_points(to_world, from, to, STEPS, 1.0) else { return };
    let bulge = arc_bulge({
        let a = ephem::geodetic_to_body(from.0, from.1);
        let c = ephem::geodetic_to_body(to.0, to.1);
        a.dot(c).clamp(-1.0, 1.0).acos()
    });

    // Where the pulse's head is on the path this frame, and how hard it hits.
    let (head, gain, width_gain) = match anim {
        // Transmitting: a fast, near-white surge running home → them.
        Some(t0) if transmitting => ((t0 * 0.55).fract(), 5.0, 3.4),
        // Receiving: a slower swell coming the other way, them → home.
        Some(t0) => (1.0 - (t0 * 0.28).fract(), 2.6, 2.2),
        None => (f32::NAN, 0.0, 1.0),
    };
    let pulse_at = |mid: f32| {
        if gain <= 0.0 {
            return 1.0;
        }
        let d = (mid - head).abs().min(1.0 - (mid - head).abs());
        1.0 + gain * (-d * d * 160.0).exp()
    };

    // A wide, dim halo under the core, so the active path reads as a beam
    // rather than as one more thread in the web.
    if anim.is_some() {
        for w in 0..STEPS {
            let mid = (w as f32 + 0.5) / STEPS as f32;
            s.lines.push(seg(
                pts[w],
                pts[w + 1],
                width_px * 2.6,
                lin(color, (0.12 * pulse_at(mid)).min(0.5) * fade),
            ));
        }
    }

    for w in 0..STEPS {
        let mid = (w as f32 + 0.5) / STEPS as f32;
        let pulse = pulse_at(mid);
        // The crest goes white-hot: colour alone cannot get brighter than the
        // arc already is, so the pulse borrows the one thing that can.
        let c =
            if anim.is_some() { mix(color, ink::TEXT_STRONG, (pulse - 1.0) * 0.4) } else { color };
        s.lines.push(seg(
            pts[w],
            pts[w + 1],
            width_px * pulse.min(width_gain),
            lin(c, (0.62 * pulse).min(1.0) * fade),
        ));
    }

    // Anchor ticks: a short radial stub at each end, so the arc visibly lands
    // on the surface rather than floating near it.
    for (lat, lon) in [from, to] {
        let d = ephem::geodetic_to_body(lat, lon);
        s.lines.push(seg(
            to_world(d, 1.0),
            to_world(d, 1.0 + bulge * 0.16),
            width_px,
            lin(color, 0.7 * fade),
        ));
    }

    if anim.is_some() {
        // The pulse's head as a glow travelling the path, and a ring on each
        // station: the endpoints of the QSO in progress, not merely of a line.
        let k = ((head.clamp(0.0, 1.0) * STEPS as f32) as usize).min(STEPS);
        s.sprites.push(SpriteInst {
            center: pts[k].arr(),
            size_px: 18.0,
            color: lin(mix(color, ink::TEXT_STRONG, 0.5), 0.85 * fade),
            params: [SPRITE_GLOW, 0.0, 0.0, 0.0],
        });
        // A slow breath on the rings, in step with the pulse's period.
        let beat = 1.0 + 0.25 * (anim.unwrap_or(0.0) * 3.0).sin();
        for (lat, lon) in [from, to] {
            s.sprites.push(SpriteInst {
                center: to_world(ephem::geodetic_to_body(lat, lon), 1.02).arr(),
                size_px: 15.0 * beat,
                color: lin(color, 0.9 * fade),
                params: [SPRITE_RING, 0.0, 0.0, 0.0],
            });
        }
    }
    let _ = b;
}

/// A fixed star field, generated once. Uniform on the sphere via the inverse
/// transform of the z coordinate; a plain LCG keeps it reproducible without a
/// dependency (and without `Math::random`, which is unavailable in wasm anyway).
pub fn stars() -> Vec<SpriteInst> {
    /// Far enough that the camera's motion within the solar system gives no
    /// visible parallax, but inside the far plane.
    const R: f32 = 300_000.0;
    const N: usize = 1500;
    let mut seed: u64 = 0x5150_5344_524f_5849;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f64 / (1u64 << 31) as f64) as f32
    };
    (0..N)
        .map(|_| {
            let z = next() * 2.0 - 1.0;
            let phi = next() * std::f32::consts::TAU;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let dir = v3(r * phi.cos(), r * phi.sin(), z);
            // A shallow magnitude distribution: mostly faint, a few bright.
            let m = next();
            let bright = 0.25 + 0.75 * m * m * m;
            SpriteInst {
                center: (dir * R).arr(),
                size_px: 1.1 + 2.2 * m * m,
                color: [bright * 0.8, bright * 0.86, bright, 1.0],
                params: [SPRITE_STAR, 0.0, 0.0, 0.0],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Solar3dView;

    fn ui() -> SolarUi {
        let mut st = SolarUi::new(Solar3dView::default());
        st.set_qth("JN78ve");
        st
    }

    #[test]
    fn uniform_blocks_are_the_size_the_shaders_expect() {
        assert_eq!(std::mem::size_of::<Globals>(), 160);
        assert_eq!(std::mem::size_of::<DrawData>(), 192);
        assert_eq!(std::mem::size_of::<LineInst>(), 48);
        assert_eq!(std::mem::size_of::<SpriteInst>(), 48);
    }

    #[test]
    fn bodies_sit_where_the_solar_system_puts_them() {
        let b = bodies(&ui(), 1_784_937_600.0);
        let au = ephem::AU as f32;
        assert!((b.earth.len() / au - 1.0).abs() < 0.02, "Earth at {} AU", b.earth.len() / au);
        // The Earth's orbit is very nearly in the ecliptic plane.
        assert!(b.earth.z.abs() < 1e-4, "Earth off-plane by {}", b.earth.z);
        let moon_off = (b.moon - b.earth).len();
        assert!((0.35..0.41).contains(&moon_off), "Earth–Moon {moon_off} Gm");
    }

    /// The shader's `eclipse_light` (`solar_body.wgsl`), mirrored in the same
    /// f32 arithmetic: how much sunlight reaches the surface point with world
    /// normal `n`, from the Earth draw's own uniforms.
    fn eclipse_light(centre: V3, moon_off: V3, n: V3) -> f32 {
        let p = n * (ephem::EARTH_R as f32);
        let to_sun = (V3::ZERO - centre) - p;
        let to_moon = moon_off - p;
        let su = to_sun.normalize();
        let mu = to_moon.normalize();
        if su.dot(mu) <= 0.0 {
            return 1.0;
        }
        let rs = ephem::SUN_R as f32 / to_sun.len();
        let rm = ephem::MOON_R as f32 / to_moon.len();
        let sep = su.cross(mu).len();
        let f = ((rs + rm - sep) / (2.0 * rs)).clamp(0.0, 1.0);
        1.0 - f * f * (3.0 - 2.0 * f)
    }

    /// The deepest Moon shadow anywhere on the sunlit hemisphere at `unix_s`,
    /// as (coverage, lat, lon) on a half-degree grid — computed from the very
    /// DrawData the Earth is rendered with, so the plumbing is under test too.
    fn deepest_shadow(st: &SolarUi, unix_s: f64) -> (f32, f64, f64) {
        let b = bodies(st, unix_s);
        let s = build(st, None, unix_s, [1600.0, 900.0], 0.0);
        let earth = s
            .draws
            .iter()
            .find(|(p, d)| *p == Prim::Sphere && d.params[0] == MODE_EARTH)
            .map(|(_, d)| *d)
            .expect("no Earth draw");
        let centre = v3(earth.model[3][0], earth.model[3][1], earth.model[3][2]);
        let moon = v3(earth.tint2[0], earth.tint2[1], earth.tint2[2]);
        let to_sun = (V3::ZERO - centre).normalize();
        let mut worst = (0.0f32, 0.0f64, 0.0f64);
        for lat_half in -180..=180 {
            for lon_half in -360..360 {
                let (lat, lon) = (lat_half as f64 * 0.5, lon_half as f64 * 0.5);
                let n = b.surface_dir(lat, lon);
                // The night side sees the same alignment through the planet;
                // there the shader's `day` is already zero, so skip it here.
                if n.dot(to_sun) <= 0.0 {
                    continue;
                }
                let cover = 1.0 - eclipse_light(centre, moon, n);
                if cover > worst.0 {
                    worst = (cover, lat, lon);
                }
            }
        }
        worst
    }

    /// 2026-08-12 17:46 UTC is the greatest eclipse of a real total solar
    /// eclipse — NASA's catalogue puts the point at 65.2°N, 25.2°W, in the
    /// Denmark Strait. The shadow is computed at TRUE scale whatever the
    /// sliders say, so the same track must come out with the Moon exaggerated
    /// and its orbit compressed — and there must be no shadow a day earlier.
    #[test]
    fn the_moons_shadow_falls_where_the_2026_eclipse_did() {
        let mut st = ui();
        st.view.body_scale = 8.0;
        st.view.moon_orbit_scale = 0.25;
        let (cover, lat, lon) = deepest_shadow(&st, 1_786_556_760.0);
        assert!(cover > 0.95, "greatest eclipse only covers {cover}");
        // ±1.5° / ±5° of longitude at 65°N is ~250 km: the floor set by the
        // solar theory's ±0.01°, the half-degree scan and the geocentric
        // latitude of the scan itself — and about the totality band's width.
        assert!(
            (lat - 65.2).abs() < 1.5 && (lon + 25.2).abs() < 5.0,
            "deepest shadow at {lat}°, {lon}° against NASA's 65.2°, −25.2°"
        );

        // Same instant, honest sliders: the track must not move.
        let plain = deepest_shadow(&ui(), 1_786_556_760.0);
        assert_eq!((lat, lon), (plain.1, plain.2), "the sliders moved the shadow");

        // A day earlier the Moon has not reached the node yet.
        let (cover, lat, lon) = deepest_shadow(&st, 1_786_470_360.0);
        assert!(cover < 1e-3, "phantom shadow {cover} at {lat}°, {lon}°");
    }

    /// The shader's `lunar_eclipse_light` (`solar_body.wgsl`), mirrored in
    /// the same f32 arithmetic: how much sunlight reaches the Moon-surface
    /// point with world normal `n`, from the Moon draw's own uniforms.
    fn lunar_eclipse_light(d: &DrawData, n: V3) -> f32 {
        let off = v3(d.tint2[0], d.tint2[1], d.tint2[2]);
        let earth = v3(d.model[3][0], d.model[3][1], d.model[3][2]) - off * d.tint2[3];
        let p = off + n * (ephem::MOON_R as f32);
        let to_sun = (V3::ZERO - earth) - p;
        let to_earth = -p;
        let su = to_sun.normalize();
        let bu = to_earth.normalize();
        if su.dot(bu) <= 0.0 {
            return 1.0;
        }
        let rs = ephem::SUN_R as f32 / to_sun.len();
        let rb = ephem::EARTH_R as f32 * 1.02 / to_earth.len();
        let sep = su.cross(bu).len();
        let f = ((rs + rb - sep) / (2.0 * rs)).clamp(0.0, 1.0);
        1.0 - f * f * (3.0 - 2.0 * f)
    }

    /// Sunlight across the Moon's sunlit face at `unix_s`, as (min, max) of
    /// the shader's lunar eclipse factor — from the Moon draw the scene
    /// actually emitted, so the tint2 plumbing is under test too.
    fn moon_sunlight(st: &SolarUi, unix_s: f64) -> (f32, f32) {
        let s = build(st, None, unix_s, [1600.0, 900.0], 0.0);
        let moon = s
            .draws
            .iter()
            .find(|(p, d)| *p == Prim::Sphere && d.params[0] == MODE_MOON)
            .map(|(_, d)| *d)
            .expect("no Moon draw");
        let off = v3(moon.tint2[0], moon.tint2[1], moon.tint2[2]);
        let earth = v3(moon.model[3][0], moon.model[3][1], moon.model[3][2]) - off * moon.tint2[3];
        let to_sun = (V3::ZERO - (earth + off)).normalize();
        let (mut lo, mut hi) = (1.0f32, 0.0f32);
        for az in (0..360).step_by(5) {
            for el in (-85..=85i32).step_by(5) {
                let (a, b) = ((az as f32).to_radians(), (el as f32).to_radians());
                let n = v3(b.cos() * a.cos(), b.cos() * a.sin(), b.sin());
                if n.dot(to_sun) <= 0.1 {
                    continue;
                }
                let l = lunar_eclipse_light(&moon, n);
                lo = lo.min(l);
                hi = hi.max(l);
            }
        }
        (lo, hi)
    }

    /// 2026-03-03 ~11:34 UTC is the greatest eclipse of a real total lunar
    /// eclipse, umbral magnitude 1.15 — the whole Moon inside the umbra. So
    /// every point of the sunlit face must sit in deep shadow, however the
    /// sliders draw the two bodies, and a day earlier none of it may.
    #[test]
    fn the_earths_shadow_swallows_the_moon_in_the_2026_eclipse() {
        let mut st = ui();
        st.view.body_scale = 8.0;
        st.view.moon_orbit_scale = 0.25;
        let (_, hi) = moon_sunlight(&st, 1_772_537_640.0);
        assert!(hi < 0.05, "a sunlit patch survives totality: {hi}");

        // An hour before greatest the eclipse is total but only just — U2 is
        // 11:04 UTC — and two hours before it is partial, with part of the
        // disc still in full sunlight. The whole published timeline, in two
        // probes.
        let (lo, hi) = moon_sunlight(&st, 1_772_537_640.0 - 3_600.0);
        assert!(lo < 0.01 && hi < 0.5, "one hour before greatest: lo {lo}, hi {hi}");
        let (lo, hi) = moon_sunlight(&st, 1_772_537_640.0 - 7_200.0);
        assert!(lo < 0.5 && hi > 0.99, "two hours before greatest: lo {lo}, hi {hi}");

        // Same instant, honest sliders: the eclipse must not move.
        let (_, hi_plain) = moon_sunlight(&ui(), 1_772_537_640.0);
        assert!(hi_plain < 0.05, "the sliders moved the lunar eclipse: {hi_plain}");

        // A day earlier the Moon is not yet opposite the Sun.
        let (lo, _) = moon_sunlight(&st, 1_772_451_240.0);
        assert!(lo > 0.999, "phantom Earth shadow on the Moon: {lo}");
    }

    /// How many bodies the scene is made of, so the counts below read as
    /// something other than magic numbers.
    fn population() -> (usize, usize) {
        (sdroxide_solar::Planet::ALL.len(), sdroxide_solar::planets::MOONS.len())
    }

    /// How many small bodies get a shaded sphere at the default framing, and
    /// how many are only ever a marker. Counted from the table rather than
    /// written down, so adding a body cannot make this quietly wrong.
    fn small_spheres(s: &Scene, b: &Bodies, cam: &Camera) -> usize {
        let _ = s;
        b.smalls.iter().filter(|sb| cam.pixels_for(sb.pos, sb.radius) > SPHERE_MIN_PX).count()
    }

    #[test]
    fn a_frame_produces_every_body_plus_orbits() {
        let (planets, moons) = population();
        let s = build(&ui(), None, 1_784_937_600.0, [1600.0, 900.0], 0.0);

        let st = ui();
        let b = bodies(&st, 1_784_937_600.0);
        let cam = Camera::from_view(&st, &b, [1600.0, 900.0]);
        let smalls = small_spheres(&s, &b, &cam);

        let spheres = s.draws.iter().filter(|(p, _)| *p == Prim::Sphere).count();
        assert_eq!(
            spheres,
            3 + planets + moons + smalls,
            "Sun, Earth, Moon, the planets, their moons and whichever small \
             bodies are big enough on screen to be worth shading"
        );
        // Saturn and Uranus have rings; nothing else drawn here does.
        assert_eq!(s.draws.iter().filter(|(p, _)| *p == Prim::Ring).count(), 2);
        // ...and the rings come after every sphere, so the transparent sheet
        // blends over the planet instead of being clipped by it.
        let first_ring = s.draws.iter().position(|(p, _)| *p == Prim::Ring).expect("rings");
        assert!(s.draws[..first_ring].iter().all(|(p, _)| *p == Prim::Sphere));

        // 256 Earth-orbit + 128 Moon-orbit segments, plus the grid and a ring
        // for every planet.
        assert!(s.lines.len() > 384 + 192 * planets, "only {} line segments", s.lines.len());
        // A glow under each body, so none of them can be invisible — including
        // every small body, which is the only thing keeping a 200 m asteroid on
        // screen at all. Comets that are active add a second one for the coma.
        let comas = b.smalls.iter().filter(|sb| sb.tails.is_some()).count();
        assert_eq!(
            s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_GLOW).count(),
            3 + planets + b.smalls.len() + comas,
            "at this framing the moons are inside their planets' glows"
        );
        assert!(s.globals.view_proj[3][3].is_finite());
    }

    /// Every planet is named from anywhere in the system — that is the only
    /// thing that makes a body a fraction of a pixel across findable — and
    /// every body is clickable whether or not it is named.
    #[test]
    fn distant_planets_are_labelled_and_clickable() {
        let (planets, moons) = population();
        let s = build(&ui(), None, 1_784_937_600.0, [1600.0, 900.0], 0.0);

        for p in sdroxide_solar::Planet::ALL {
            let name = p.name().to_uppercase();
            assert!(s.labels.iter().any(|l| l.text == name), "{name} is not labelled");
        }
        // Moons stay quiet until their planet is big enough to hang names off.
        assert!(!s.labels.iter().any(|l| l.text == "IO"), "moons labelled from 2 AU away");

        // Pick targets exist for everything, name or no name, and each is big
        // enough to actually hit — a 200 m asteroid especially, since a pointer
        // has nothing else to aim at.
        let smalls = sdroxide_solar::smallbody::BODIES.len();
        assert_eq!(s.picks.len(), 3 + planets + moons + smalls);
        assert!(s.picks.iter().all(|p| p.radius_px >= 7.0));
        for f in [
            Focus::Sun,
            Focus::Earth,
            Focus::Planet(sdroxide_solar::Planet::Neptune),
            Focus::Small(0),
        ] {
            assert!(s.picks.iter().any(|p| p.focus == f), "{f:?} cannot be clicked");
        }

        // With the labels layer off the names go but the targets stay.
        let mut dark = ui();
        dark.view.layers &= !layer::LABELS;
        let s = build(&dark, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.labels.is_empty());
        assert_eq!(s.picks.len(), 3 + planets + moons + smalls);
    }

    /// A comet's tails, as geometry rather than as decoration.
    ///
    /// Everything here is checkable from the draw list, and every one of these
    /// is a mistake that would still look like a comet in a screenshot: a tail
    /// pointing the wrong way, a tail that never switches off, a dust tail that
    /// does not curve, tails on a body that has none.
    #[test]
    fn a_comet_grows_tails_that_point_away_from_the_sun() {
        use sdroxide_solar::smallbody;

        // Halley at its 2061 perihelion — the one apparition in the window.
        let halley = smallbody::find("Halley").unwrap();
        let peri = halley.next_perihelion(smallbody::WINDOW.0);
        let unix = sdroxide_solar::ephem::unix_from_julian_day(peri);

        let st = ui();
        let b = bodies(&st, unix);
        let sb = b.smalls.iter().find(|s| s.info.name == "Halley").expect("Halley is placed");
        let t = sb.tails.expect("Halley has tails at perihelion");

        // Away from the Sun, but not exactly: the wind is met at an angle, and
        // the few degrees that puts in are the whole point of modelling it.
        let sunward = sb.pos.normalize();
        let ion = V3::from_f64(t.ion);
        let off = ion.dot(sunward).clamp(-1.0, 1.0).acos().to_degrees();
        assert!(ion.dot(sunward) > 0.9, "the ion tail is not anti-sunward at all");
        assert!((1.0..12.0).contains(&off), "aberration is {off}°");

        // Two tail draws, and they are the last of the geometry, so they add
        // over everything they are in front of instead of being clipped by it.
        let scene = build(&st, None, unix, [1600.0, 900.0], 0.0);
        let tails: Vec<&DrawData> =
            scene.draws.iter().filter(|(p, _)| *p == Prim::Tail).map(|(_, d)| d).collect();
        let comets = b.smalls.iter().filter(|s| s.tails.is_some()).count();
        assert!(comets >= 1);
        let want: usize = b.smalls.iter().filter_map(|s| s.tails).map(tail_count).sum();
        assert_eq!(tails.len(), want);

        // The dust tail bows and the ion tail does not — `params.w` is the
        // curvature, and a dust tail that came out straight would be the most
        // plausible-looking bug in the whole feature.
        let ions: Vec<&&DrawData> = tails.iter().filter(|d| d.params[0] == TAIL_ION).collect();
        let dusts: Vec<&&DrawData> = tails.iter().filter(|d| d.params[0] == TAIL_DUST).collect();
        assert!(ions.iter().all(|d| d.params[3] == 0.0), "an ion tail is curved");
        assert!(dusts.iter().all(|d| d.params[3] > 0.1), "a dust tail is straight");
        // ...and the ion tail is the longer of the two, as it is in every
        // photograph of a comet ever taken.
        assert!(t.ion_gm > t.dust_gm);

        // Half an orbit later Halley is at aphelion, out past Neptune, and has
        // nothing to draw. The scene still does — with fifteen comets in the
        // table something is always near the Sun, which is rather the point of
        // having put them there — so this asks about Halley, not about the
        // draw list.
        let cold =
            sdroxide_solar::ephem::unix_from_julian_day(peri + halley.arc(peri).period_d() * 0.5);
        let far = bodies(&st, cold);
        let halley_then = far.smalls.iter().find(|s| s.info.name == "Halley").unwrap();
        assert!(halley_then.tails.is_none(), "a comet at aphelion still has its tails up");
        assert!(halley.distance_au(sdroxide_solar::ephem::julian_day(cold)) > 30.0);
    }

    /// How many tail draws a set of tails is worth: one each, and none for a
    /// length of zero — which is what a rock like Phaethon gets for its ion
    /// tail, having no ice to make one from.
    fn tail_count(t: sdroxide_solar::Tails) -> usize {
        (t.ion_gm > 0.0) as usize + (t.dust_gm > 0.0) as usize
    }

    /// The small-body name chip, and the two things that outrank it.
    ///
    /// Its whole reason to exist is that thirty-five asteroid designations at
    /// once bury the planets, so with it on there must be visibly more names
    /// and with it off the dwarf planets must survive — they are planets to the
    /// eye and were never the clutter. And a name that was *asked for*, by
    /// search or by pointing the camera, has to appear either way, or the find
    /// box would look broken to anyone who had switched the chip off.
    #[test]
    fn the_small_body_chip_governs_names_but_not_the_ones_asked_for() {
        let named = |st: &SolarUi| -> Vec<String> {
            build(st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0)
                .labels
                .iter()
                .map(|l| l.text.clone())
                .collect()
        };

        let mut on = ui();
        on.view.layers |= layer::SMALL_LABELS;
        let mut off = ui();
        off.view.layers &= !layer::SMALL_LABELS;

        // Pluto is a planet as far as the eye is concerned, and stays named.
        assert!(named(&off).iter().any(|t| t == "Pluto"), "the dwarf planets went with the chip");
        assert!(named(&on).len() >= named(&off).len(), "the chip did not add any names");

        // Searching names the match with the chip off — and with LABELS off too.
        let mut hunting = ui();
        hunting.view.layers &= !layer::SMALL_LABELS;
        hunting.search = "Apophis".into();
        hunting.view.layers &= !layer::LABELS;
        assert!(
            named(&hunting).iter().any(|t| t == "Apophis"),
            "the find box highlighted a body and then withheld its name"
        );

        // ...and so does pointing the camera at one.
        let mut aimed = ui();
        aimed.view.layers &= !layer::SMALL_LABELS;
        let apophis = sdroxide_solar::smallbody::BODIES
            .iter()
            .position(|b| b.name == "Apophis")
            .expect("Apophis is in the table");
        aimed.set_focus(Focus::Small(apophis));
        aimed.view.layers &= !layer::LABELS;
        assert!(named(&aimed).iter().any(|t| t == "Apophis"), "the camera target is not named");
    }

    /// Framed on Jupiter, its moons get their names — and they are in the right
    /// order outward from the planet, which is the check that the table's
    /// orbit radii and the renderer's scaling agree.
    #[test]
    fn a_planets_moons_appear_when_it_fills_the_frame() {
        let mut st = ui();
        st.set_focus(Focus::Planet(sdroxide_solar::Planet::Jupiter));
        st.view.dist = 0.6;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        for name in ["IO", "EUROPA", "GANYMEDE", "CALLISTO"] {
            assert!(s.labels.iter().any(|l| l.text == name), "{name} missing");
        }

        let b = bodies(&st, 1_784_937_600.0);
        let jupiter =
            b.planets.iter().find(|p| p.planet == sdroxide_solar::Planet::Jupiter).unwrap();
        let radii: Vec<f32> = b
            .moons
            .iter()
            .filter(|m| m.info.parent == sdroxide_solar::Planet::Jupiter)
            .map(|m| (m.pos - jupiter.pos).len() / jupiter.radius)
            .collect();
        assert!(radii.windows(2).all(|w| w[0] < w[1]), "the Galilean order is wrong: {radii:?}");
        // Io orbits at 5.9 Jupiter radii and Callisto at 26.3, whatever the
        // exaggeration slider is set to — the system keeps its true shape.
        assert!((radii[0] - 5.9).abs() < 0.2, "Io at {} radii", radii[0]);
        assert!((radii[3] - 26.9).abs() < 0.5, "Callisto at {} radii", radii[3]);
    }

    /// The exaggeration cap: the Sun has to stay the biggest thing in the
    /// picture, or the view stops being a picture of the solar system.
    #[test]
    fn no_planet_outgrows_the_sun() {
        let mut st = ui();
        st.view.body_scale = 20.0;
        let b = bodies(&st, 1_784_937_600.0);
        for p in &b.planets {
            assert!(p.radius < b.sun_r * 0.5, "{} is {} Gm across", p.planet.name(), p.radius);
            assert!(p.radius >= p.planet.info().radius as f32, "{} shrank", p.planet.name());
            assert!(p.exaggeration >= 1.0);
        }
        // Mercury is small enough that the cap never binds: it gets the full
        // exaggeration the slider asks for.
        let mercury =
            b.planets.iter().find(|p| p.planet == sdroxide_solar::Planet::Mercury).unwrap();
        assert!((mercury.exaggeration - 20.0).abs() < 0.01);
    }

    #[test]
    fn the_planets_layer_removes_them_all() {
        let mut st = ui();
        st.view.layers &= !layer::PLANETS;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert_eq!(s.draws.len(), 3, "only the Sun, the Earth and the Moon are left");
        assert!(s.draws.iter().all(|(p, _)| *p == Prim::Sphere), "a ring survived");
        assert!(!s.labels.iter().any(|l| l.text == "JUPITER"));
        assert_eq!(s.picks.len(), 3);
    }

    /// Surface markers are positions *on* the Earth, so they appear only once
    /// the Earth is big enough on screen to place them — at the default
    /// whole-system framing it is under a pixel across.
    #[test]
    fn surface_markers_fade_in_with_the_earth() {
        let ring = |s: &Scene| s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_RING).count();

        let wide = build(&ui(), None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert_eq!(ring(&wide), 0, "QTH ring drawn over a sub-pixel Earth");

        let mut close = ui();
        close.view.focus = Focus::Earth.to_id();
        close.view.dist = 1.0;
        let s = build(&close, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert_eq!(ring(&s), 1, "no QTH ring when framed on the Earth");
        assert_eq!(s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_DOT).count(), 1);
        // ...and never wider than the planet they sit on.
        let earth_px = {
            let b = bodies(&close, 1_784_937_600.0);
            Camera::from_view(&close, &b, [1600.0, 900.0]).pixels_for(b.earth, b.earth_r)
        };
        for sp in s.sprites.iter().filter(|sp| sp.params[0] != SPRITE_GLOW) {
            assert!(
                sp.size_px <= earth_px * 1.5 + 0.01,
                "marker {} px on a {earth_px} px Earth",
                sp.size_px
            );
        }
    }

    /// With every switchable layer off, what is left is what is not a layer:
    /// the three near bodies, the star field, and the graticule the scene is
    /// read against.
    #[test]
    fn layers_actually_remove_geometry() {
        let mut st = ui();
        st.view.layers = 0;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert_eq!(s.draws.len(), 3, "the Sun, the Earth and the Moon are not a layer");
        assert!(s.draw_stars, "the star field is the backdrop, not a layer");

        // Only the graticule's lines survive; switching a layer back on adds to
        // them, which is what makes this a real check that the rest went away.
        let grid_lines = s.lines.len();
        assert!(grid_lines > 0, "the graticule is always drawn");
        st.view.layers = layer::ORBITS;
        let with_orbits = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(with_orbits.lines.len() > grid_lines, "ORBITS added nothing");
    }

    #[test]
    fn the_qth_marker_lands_on_the_daylit_side_at_local_noon() {
        // 2026-07-25, when the Sun is overhead near 15.79°E (the test QTH's
        // longitude), i.e. about 11:00 UTC.
        let st = ui();
        let unix = 1_784_977_500.0; // 2026-07-25T11:05:00Z
        let b = bodies(&st, unix);
        let (ex, ey, ez) = b.earth_basis;
        let d = ephem::geodetic_to_body(48.19, 15.79);
        let qth_normal = ex * d.x as f32 + ey * d.y as f32 + ez * d.z as f32;
        let to_sun = (V3::ZERO - b.earth).normalize();
        assert!(qth_normal.dot(to_sun) > 0.6, "QTH not near the sub-solar point");
    }

    fn cme(speed: f64, lat: f64, lon_west: f64, t: i64) -> sdroxide_solar::CmeEvent {
        sdroxide_solar::CmeEvent {
            id: "test".into(),
            start_unix: t,
            active_region: None,
            note: String::new(),
            link: String::new(),
            analysis: Some(sdroxide_solar::CmeAnalysis {
                t21_5_unix: t,
                lat_deg: lat,
                lon_west_deg: lon_west,
                half_angle_deg: 30.0,
                speed_km_s: speed,
                kind: "C".into(),
                estimated: false,
            }),
        }
    }

    /// Cones are frusta beginning at the 21.5 R☉ launch height, not full cones
    /// from the Sun's centre. Without that, a close-up of the solar disk sits
    /// *inside* every cone in the scene and is swamped by their inner surfaces.
    #[test]
    fn cme_cones_are_truncated_at_the_launch_radius() {
        let now = 1_784_937_600i64;
        let mut st = ui();
        st.view.focus = Focus::Sun.to_id();
        let mut data = SolarData::default();
        // A fresh event, a two-day-old one, and a very old one.
        data.cmes = vec![
            cme(700.0, 5.0, 0.0, now),
            cme(700.0, 5.0, 40.0, now - 2 * 86_400),
            cme(400.0, -20.0, -90.0, now - 60 * 3600),
        ];
        let s = build(&st, Some(&data), now as f64, [1600.0, 900.0], 0.0);

        let cones: Vec<_> = s.draws.iter().filter(|(p, _)| *p == Prim::Cone).collect();
        assert_eq!(cones.len(), 3, "all three CMEs are inside the 72 h window");
        for (_, d) in &cones {
            let inner = d.params[3];
            assert!((0.0..1.0).contains(&inner), "inner radius {inner} out of range");
            // Scale is the length; the model matrix's first column is the
            // scaled basis vector, so its length is the cone's length.
            let len =
                (d.model[0][0].powi(2) + d.model[0][1].powi(2) + d.model[0][2].powi(2)).sqrt();
            let launch = sdroxide_solar::impact::LAUNCH_RADIUS as f32;
            assert!(len >= launch, "cone only {len} Gm long, inside the launch radius");
            assert!(
                (inner * len - launch).abs() < 0.01,
                "truncation at {} Gm, expected {launch}",
                inner * len
            );
            assert!(d.params[1] > 0.0 && d.params[2] > 0.0);
        }
        // The older event has travelled further.
        let lens: Vec<f32> = cones
            .iter()
            .map(|(_, d)| {
                (d.model[0][0].powi(2) + d.model[0][1].powi(2) + d.model[0][2].powi(2)).sqrt()
            })
            .collect();
        assert!(lens[1] > lens[0], "the two-day-old CME should be further out");
    }

    #[test]
    fn cme_events_outside_the_window_are_dropped() {
        let now = 1_784_937_600i64;
        let st = ui();
        let mut data = SolarData::default();
        data.cmes = vec![
            cme(700.0, 0.0, 0.0, now - 200 * 3600), // older than the 72 h window
            cme(700.0, 0.0, 0.0, now + 3600),       // not launched yet
        ];
        let s = build(&st, Some(&data), now as f64, [1600.0, 900.0], 0.0);
        assert_eq!(s.draws.iter().filter(|(p, _)| *p == Prim::Cone).count(), 0);
    }

    /// Tokyo, Sydney, New York — the fixture stations, spread far enough apart
    /// that they never crowd each other's labels.
    const TOKYO: (f64, f64) = (35.7, 139.7);
    const SYDNEY: (f64, f64) = (-33.9, 151.2);
    const NEW_YORK: (f64, f64) = (40.7, -74.0);

    fn station((lat, lon): (f64, f64), fade: f32, call: &str) -> crate::digi_map::DigiStation {
        crate::digi_map::DigiStation { lat, lon, fade, call: Some(call.into()), band: None }
    }

    fn hit((lat, lon): (f64, f64), slot_utc: i64, call: &str) -> crate::digi_map::DigiHit {
        crate::digi_map::DigiHit { lat, lon, slot_utc, call: Some(call.into()) }
    }

    /// Framed on the Earth, with a contact on the other side of the planet.
    fn earth_view_with_traffic(dx: Option<(f64, f64)>) -> SolarUi {
        let mut st = ui();
        st.view.focus = Focus::Earth.to_id();
        st.view.dist = 0.5;
        st.digi = super::super::state::DigiTraffic {
            stations: vec![
                station(TOKYO, 1.0, "JA1ABC"),
                station(SYDNEY, 0.4, "VK2XYZ"),
                station(NEW_YORK, 0.05, "W2NYC"),
            ],
            dx,
            preview: None,
            transmitting: false,
            ..Default::default()
        };
        st
    }

    /// The same view with an hour of decode history behind it: three stations
    /// at 0, 10 and 50 minutes before `now`.
    fn earth_view_with_history(now: i64) -> SolarUi {
        let mut st = earth_view_with_traffic(None);
        st.digi.history = std::sync::Arc::new(vec![
            hit(NEW_YORK, now - 50 * 60, "W2NYC"),
            hit(SYDNEY, now - 10 * 60, "VK2XYZ"),
            hit(TOKYO, now, "JA1ABC"),
        ]);
        st
    }

    /// Every callsign the scene offered to the overlay.
    fn callsigns(s: &Scene) -> Vec<String> {
        s.labels.iter().filter(|l| l.rank != RANK_ALWAYS).map(|l| l.text.clone()).collect()
    }

    #[test]
    fn decoded_stations_become_dots_on_the_globe() {
        let st = earth_view_with_traffic(None);
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        // Three stations plus the sub-solar dot.
        assert_eq!(s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_DOT).count(), 4);
        // ...and they sit on the surface, not floating or buried.
        let b = bodies(&st, 1_784_937_600.0);
        for sp in s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_DOT) {
            let r = (v3(sp.center[0], sp.center[1], sp.center[2]) - b.earth).len() / b.earth_r;
            assert!((1.0..1.05).contains(&r), "marker at {r} Earth radii");
        }
    }

    /// A dot says "somebody is there"; the callsign says who, which is the
    /// question an operator is actually asking of the map.
    #[test]
    fn decoded_stations_are_named_with_their_callsign() {
        let st = earth_view_with_traffic(None);
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        let named = callsigns(&s);
        assert!(!named.is_empty(), "no station was named");
        assert!(
            named.iter().all(|c| ["JA1ABC", "VK2XYZ", "W2NYC"].contains(&c.as_str())),
            "unexpected labels: {named:?}"
        );
    }

    /// egui paints the labels after the 3D pass, so nothing hides them the way
    /// the depth buffer hides the dots. A station round the back of the planet
    /// must be dropped here, or its callsign is written across the near side and
    /// the map says the signal came from the wrong ocean.
    #[test]
    fn a_station_on_the_far_side_of_the_globe_is_not_named() {
        let now = 1_784_937_600.0;
        let mut st = earth_view_with_traffic(None);
        // Tokyo faces this camera; the point opposite it on the globe is as far
        // behind the planet as a station can be.
        st.digi.stations =
            vec![station(TOKYO, 1.0, "NEARSIDE"), station(antipode(TOKYO), 1.0, "FARSIDE")];
        let named = callsigns(&build(&st, None, now, [1600.0, 900.0], 0.0));
        assert_eq!(named, ["NEARSIDE"], "the far side of the world was named");
    }

    /// Diametrically opposite on the globe.
    fn antipode((lat, lon): (f64, f64)) -> (f64, f64) {
        (-lat, if lon > 0.0 { lon - 180.0 } else { lon + 180.0 })
    }

    /// The dots survive to the horizon and beyond; only the names stop there.
    #[test]
    fn the_far_side_still_gets_its_dot() {
        let mut st = earth_view_with_traffic(None);
        st.digi.stations =
            vec![station(TOKYO, 1.0, "NEARSIDE"), station(antipode(TOKYO), 1.0, "FARSIDE")];
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        // Both stations, plus the sub-solar dot.
        assert_eq!(s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_DOT).count(), 3);
    }

    /// Turning names off means all names. An operator who cleared the LABELS
    /// chip did not mean "except for these".
    #[test]
    fn callsigns_follow_the_labels_layer() {
        let mut st = earth_view_with_traffic(None);
        st.view.layers &= !layer::LABELS;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(callsigns(&s).is_empty());
    }

    /// From far enough out a callsign covers the continent it is standing on,
    /// so the names drop away before the dots do.
    #[test]
    fn callsigns_go_away_when_the_earth_is_small() {
        let mut st = earth_view_with_traffic(None);
        st.view.dist = 40.0;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(callsigns(&s).is_empty(), "named a planet too small to write on");
        assert!(s.sprites.iter().any(|sp| sp.params[0] == SPRITE_DOT), "the dots went with them");
    }

    /// Wound back, the live dots are not drawn — so the replay has to name its
    /// own stations, or the time-lapse is a light show with nobody in it.
    #[test]
    fn the_replay_names_the_stations_it_is_replaying() {
        let now = 1_784_937_600i64;
        let mut st = earth_view_with_traffic(None);
        st.digi.history = std::sync::Arc::new(vec![
            hit(TOKYO, now - 30 * 60, "OLDNEWS"),
            hit(TOKYO, now - 5 * 60, "JA1ABC"),
        ]);
        st.set_lapse_back(4.0 * 60.0);
        let named = callsigns(&build(&st, None, now as f64, [1600.0, 900.0], 0.0));
        assert_eq!(named, ["JA1ABC"], "the replay named the wrong hour");
    }

    /// Live, the trail and the live set overlap: every station on the trail was
    /// also decoded in the last two minutes. Naming both would print most calls
    /// twice, once for the dot and once for the arc that arrived at it.
    #[test]
    fn the_replay_does_not_double_up_the_live_names() {
        let now = 1_784_937_600i64;
        let st = earth_view_with_history(now);
        assert!(st.lapse_live());
        let named = callsigns(&build(&st, None, now as f64, [1600.0, 900.0], 0.0));
        let mut seen = named.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), named.len(), "a callsign was labelled twice: {named:?}");
    }

    /// The arc has to leave the surface, or on a sphere the far half of it is
    /// hidden behind the planet and the contact reads as going nowhere.
    #[test]
    fn a_qso_arc_bows_out_into_space_and_lands_at_both_ends() {
        // Tokyo — nearly antipodal to the JN78ve test QTH, so the longest case.
        let st = earth_view_with_traffic(Some((35.7, 139.7)));
        let now = 1_784_937_600.0;
        let b = bodies(&st, now);
        let plain = build(&earth_view_with_traffic(None), None, now, [1600.0, 900.0], 0.0);
        let s = build(&st, None, now, [1600.0, 900.0], 0.0);

        let added = s.lines.len() - plain.lines.len();
        assert!(added >= 96, "only {added} arc segments");

        let radius = |p: [f32; 3]| (v3(p[0], p[1], p[2]) - b.earth).len() / b.earth_r;
        let arc: Vec<f32> = s.lines[plain.lines.len()..].iter().map(|l| radius(l.a)).collect();
        let peak = arc.iter().copied().fold(0.0f32, f32::max);
        assert!(peak > 1.25, "arc only reached {peak} Earth radii — it hugs the surface");
        // Both ends come back down to the ground.
        assert!(arc[0] < 1.02, "arc starts {} radii up", arc[0]);
        assert!(radius(s.lines.last().unwrap().b) < 1.05);
        // ...and never dips inside the planet, which would hide it.
        assert!(arc.iter().all(|r| *r >= 0.999), "arc passes through the Earth");
    }

    #[test]
    fn a_short_hop_arcs_less_than_a_long_one() {
        let now = 1_784_937_600.0;
        let peak = |dx| {
            let st = earth_view_with_traffic(Some(dx));
            let b = bodies(&st, now);
            let base = build(&earth_view_with_traffic(None), None, now, [1600.0, 900.0], 0.0);
            let s = build(&st, None, now, [1600.0, 900.0], 0.0);
            s.lines[base.lines.len()..]
                .iter()
                .map(|l| (v3(l.a[0], l.a[1], l.a[2]) - b.earth).len() / b.earth_r)
                .fold(0.0f32, f32::max)
        };
        // A neighbouring country versus the far side of the world.
        assert!(peak((48.0, 11.0)) < peak((35.7, 139.7)));
    }

    /// The scene's inks are the classic palette, frozen — *not* the live
    /// theme. The Earth shader bakes these hues into the globe itself, and the
    /// amber theme once recoloured the arcs and orbit rings into the globe's
    /// own brightness range, where they vanished. The atomics are process-wide
    /// and tests run in parallel, so this pins the values instead of flipping
    /// the theme; scene code reaching for `crate::theme` is the thing to catch
    /// in review.
    #[test]
    fn the_scene_inks_are_the_classic_palette_not_the_theme() {
        let p = crate::theme::palette(); // tests never call set_look, so this is the boot-up look
        for (got, want, name) in [
            (ink::CYAN, p.cyan, "cyan"),
            (ink::CYAN_DIM, p.cyan_dim, "cyan_dim"),
            (ink::LINE_LIT, p.line_lit, "line_lit"),
            (ink::PINK, p.pink, "pink"),
            (ink::YELLOW, p.yellow, "yellow"),
            (ink::GREEN, p.green, "green"),
            (ink::TEXT_STRONG, p.text_strong, "text_strong"),
        ] {
            assert_eq!(got, want, "ink::{name} drifted from the classic palette");
        }
    }

    #[test]
    fn the_qso_layer_removes_stations_and_arcs() {
        let mut st = earth_view_with_traffic(Some((35.7, 139.7)));
        st.view.layers &= !layer::QSO;
        let s = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        // Only the sub-solar dot is left.
        assert_eq!(s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_DOT).count(), 1);
    }

    /// The contact being worked has to win against an hour of traffic behind
    /// it, or the operator cannot find their own QSO on the globe.
    #[test]
    fn the_active_arc_is_drawn_far_heavier_than_a_history_arc() {
        let now = 1_784_937_600i64;
        let quiet = build(&earth_view_with_history(now), None, now as f64, [1600.0, 900.0], 0.0);
        let mut st = earth_view_with_history(now);
        st.digi.dx = Some((35.7, 139.7));
        let s = build(&st, None, now as f64, [1600.0, 900.0], 0.0);

        let widest = s.lines.iter().map(|l| l.width_px).fold(0.0f32, f32::max);
        let history_max = 1.1 * 2.6; // a history arc at the peak of its spark
        assert!(widest > 2.0 * history_max, "the active arc is only {widest} px wide");
        // …and it carries a glow head and a ring on each of the two stations.
        let count =
            |s: &Scene, kind: f32| s.sprites.iter().filter(|sp| sp.params[0] == kind).count();
        assert_eq!(count(&s, SPRITE_GLOW), count(&quiet, SPRITE_GLOW) + 1);
        assert_eq!(count(&s, SPRITE_RING), count(&quiet, SPRITE_RING) + 2);
    }

    /// The card is hung off `qso_apex`, so the apex has to be a point *on the
    /// arc*, halfway along it. It is derived rather than sampled — the whole
    /// point is not to walk ninety-six points again — so this is what stops the
    /// two derivations drifting apart and the card floating off the arc.
    #[test]
    fn the_qso_apex_sits_on_the_arc_halfway_between_its_ends() {
        let now = 1_784_937_600.0;
        let plain = build(&earth_view_with_traffic(None), None, now, [1600.0, 900.0], 0.0);
        let s =
            build(&earth_view_with_traffic(Some((35.7, 139.7))), None, now, [1600.0, 900.0], 0.0);
        let apex = s.qso_apex.expect("an arc was drawn, so it has an apex");

        let dist = |a: [f32; 3], b: [f32; 3]| {
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        // Every vertex the contact's arc added, both ends of every segment.
        let added = &s.lines[plain.lines.len()..];
        let verts: Vec<[f32; 3]> = added.iter().flat_map(|l| [l.a, l.b]).collect();

        // The path runs from the operator's QTH to Tokyo, so its two ground ends
        // are the furthest-apart pair on it — and the first segment starts at one
        // of them, because `arc_points` samples from `home` outwards.
        let home = added[0].a;
        let (far, span) = verts
            .iter()
            .map(|v| (*v, dist(home, *v)))
            .fold((home, 0.0f32), |a, b| if b.1 > a.1 { b } else { a });

        // On the arc: the apex is one of the points actually drawn, not merely
        // near them.
        let off = verts.iter().map(|v| dist(apex, *v)).fold(f32::MAX, f32::min);
        assert!(off < span * 1e-3, "the apex is {off} off the arc it should sit on (span {span})");
        // …and halfway along it. Loosely, because the furthest vertex from the
        // near end is the tip of the far end's anchor tick rather than the last
        // point of the path itself — a fraction of a percent out, against the
        // tens of percent a card hung off the wrong point would be.
        let (a, b) = (dist(apex, home), dist(apex, far));
        assert!(
            (a - b).abs() < span * 1e-2,
            "the apex is {a} from one end and {b} from the other, so it is not the midpoint"
        );
    }

    /// No arc, no card: the apex is what the overlay tests to decide whether to
    /// draw one at all.
    #[test]
    fn there_is_no_qso_apex_without_a_contact() {
        let now = 1_784_937_600.0;
        let s = build(&earth_view_with_traffic(None), None, now, [1600.0, 900.0], 0.0);
        assert!(s.qso_apex.is_none(), "an apex with nobody being worked");
    }

    /// The contact's arc is green, and the traffic around it is not — the one
    /// thing that tells the operator's own QSO from an hour of other people's.
    #[test]
    fn the_active_arc_is_green_and_the_history_behind_it_is_not() {
        let now = 1_784_937_600i64;
        let mut st = earth_view_with_history(now);
        st.digi.dx = Some((35.7, 139.7));
        let plain = build(&earth_view_with_history(now), None, now as f64, [1600.0, 900.0], 0.0);
        let s = build(&st, None, now as f64, [1600.0, 900.0], 0.0);

        // Linear-space green, which is what the scene stores.
        let want = lin(ink::GREEN, 1.0);
        let hue = |c: [f32; 4]| {
            let m = c[0].max(c[1]).max(c[2]).max(1e-6);
            [c[0] / m, c[1] / m, c[2] / m]
        };
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05);
        let added = &s.lines[plain.lines.len()..];
        // Not every segment: the travelling pulse takes its crest to white, and
        // that handful of segments is supposed to be the brightest thing on the
        // arc. The body of it is green.
        let green = added.iter().filter(|l| close(hue(l.color), hue(want))).count();
        assert!(
            green * 4 > added.len() * 3,
            "only {green} of {} segments of the contact's arc are green",
            added.len()
        );
        assert!(
            !plain.lines.iter().any(|l| close(hue(l.color), hue(want))),
            "the traffic behind it is green too, so the contact does not stand out"
        );
    }

    /// The pulse is a travelling band, so the arc's brightest segment has to
    /// move along it as time passes — and transmitting has to hit harder than
    /// receiving, which is the whole point of the cue.
    #[test]
    fn the_active_arc_pulse_travels_and_transmitting_hits_hardest() {
        let now = 1_784_937_600.0;
        // Widest line and where it is, over the lines the active arc added.
        let crest = |tx: bool, t: f32| {
            let mut st = earth_view_with_traffic(Some((35.7, 139.7)));
            st.digi.transmitting = tx;
            let plain = build(&earth_view_with_traffic(None), None, now, [1600.0, 900.0], t);
            let s = build(&st, None, now, [1600.0, 900.0], t);
            s.lines[plain.lines.len()..]
                .iter()
                .enumerate()
                .map(|(i, l)| (i, l.width_px))
                .fold((0, 0.0f32), |a, b| if b.1 > a.1 { b } else { a })
        };

        let (_, tx_w) = crest(true, 0.0);
        let (_, rx_w) = crest(false, 0.0);
        assert!(tx_w > rx_w, "transmit pulse {tx_w} px is no wider than receive's {rx_w}");
        // Both are heavier than the old flat 2.4 px line this replaced.
        assert!(rx_w > 4.0, "even the receive arc should be a beam, not a hair: {rx_w} px");

        assert_ne!(crest(true, 0.0).0, crest(true, 0.5).0, "the pulse did not move");
    }

    /// The last hour, replayed: what the head is parked over decides which
    /// decodes are on the globe at all.
    #[test]
    fn the_time_lapse_shows_the_hour_around_its_replay_head() {
        let now = 1_784_937_600i64;
        let mut st = earth_view_with_history(now);
        st.view.lapse_trail_min = 15.0;
        // The same view with nothing in the history, as the line-count baseline.
        let base = {
            let mut b = earth_view_with_history(now);
            b.digi.history = Default::default();
            build(&b, None, now as f64, [1600.0, 900.0], 0.0).lines.len()
        };
        let arcs =
            |st: &SolarUi| build(st, None, now as f64, [1600.0, 900.0], 0.0).lines.len() - base;

        // Live: the decodes from now and ten minutes ago are inside a 15-minute
        // trail; the one from fifty minutes ago is not.
        assert_eq!(arcs(&st), 2 * LAPSE_ARC_STEPS);

        // Wound back fifty minutes, only the oldest is at the head.
        st.set_lapse_back(50.0 * 60.0);
        assert_eq!(arcs(&st), LAPSE_ARC_STEPS);

        // A decode the head has not reached yet is not on the globe at all.
        st.set_lapse_back(55.0 * 60.0);
        assert_eq!(arcs(&st), 0);
    }

    /// Wound back, "now" is not what is being shown, so the live dots — which
    /// are a statement about the present — have to go.
    #[test]
    fn winding_the_replay_back_drops_the_live_dots() {
        let now = 1_784_937_600i64;
        let mut st = earth_view_with_history(now);
        let dots = |st: &SolarUi| {
            build(st, None, now as f64, [1600.0, 900.0], 0.0)
                .sprites
                .iter()
                .filter(|sp| sp.params[0] == SPRITE_DOT)
                .count()
        };
        // Three live stations, two history hits inside the trail, the sub-solar dot.
        assert_eq!(dots(&st), 3 + 2 + 1);
        st.set_lapse_back(10.0 * 60.0);
        // The live three are gone; the head sits on the ten-minute-old decode,
        // with the one from fifty minutes ago still outside the trail.
        assert_eq!(dots(&st), 1 + 1);
    }

    /// The award layer is a map of what is *missing*, so an unworked entity has
    /// to be the loudest thing on it and a confirmed one the quietest.
    #[test]
    fn the_award_layer_burns_hottest_where_the_log_has_holes() {
        let now = 1_784_937_600.0;
        let mut st = ui();
        st.view.focus = Focus::Earth.to_id();
        st.view.dist = 0.06; // close enough for a per-entity marker to mean something
        st.view.layers |= layer::AWARDS;
        let slot = |name, lat, lon, coverage| sdroxide_types::EntitySlot {
            name,
            lat,
            lon,
            continent: "EU",
            coverage,
        };
        st.awards = std::sync::Arc::new(vec![
            slot("Mongolia", 46.0, 104.0, Coverage::Missing),
            slot("Fed. Rep. of Germany", 51.0, 10.0, Coverage::Worked),
            slot("Japan", 36.0, 138.0, Coverage::Confirmed),
        ]);

        let s = build(&st, None, now, [1600.0, 900.0], 0.0);
        // The missing one gets a glow the others do not.
        let glows = s.sprites.iter().filter(|sp| sp.params[0] == SPRITE_GLOW).count();
        let quiet = {
            let mut st = ui();
            st.view.focus = Focus::Earth.to_id();
            st.view.dist = 0.06;
            build(&st, None, now, [1600.0, 900.0], 0.0)
        };
        let base_glows = quiet.sprites.iter().filter(|sp| sp.params[0] == SPRITE_GLOW).count();
        assert_eq!(glows, base_glows + 1, "only the missing entity should glow");

        // Three markers, and the missing one is the biggest.
        let added = &s.sprites[quiet.sprites.len()..];
        assert_eq!(added.len() - 1, 3, "one marker per entity, plus the glow");
        // Emitted in list order: the missing entity's glow, then a marker each.
        let sizes: Vec<f32> = added.iter().skip(1).map(|sp| sp.size_px).collect();
        assert!(
            sizes[0] > sizes[1] && sizes[1] > sizes[2],
            "sizes do not rank missing > worked > confirmed: {sizes:?}"
        );

        // …and switching the layer off takes all of it away again.
        st.view.layers &= !layer::AWARDS;
        let off = build(&st, None, now, [1600.0, 900.0], 0.0);
        assert_eq!(off.sprites.len(), quiet.sprites.len());
    }

    /// Three hundred markers on a globe a few pixels across is noise, so the
    /// layer waits until the Earth is worth painting them on.
    #[test]
    fn the_award_layer_stays_off_a_distant_earth() {
        let mut st = ui();
        st.view.layers |= layer::AWARDS;
        st.awards = std::sync::Arc::new(vec![sdroxide_types::EntitySlot {
            name: "Mongolia",
            lat: 46.0,
            lon: 104.0,
            continent: "AS",
            coverage: Coverage::Missing,
        }]);
        // The default view frames the whole Earth orbit: the planet is a dot.
        let far = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        st.awards = Default::default();
        let none = build(&st, None, 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert_eq!(far.sprites.len(), none.sprites.len());
    }

    /// A synthetic oval: a solid band from 60° to 75° in both hemispheres, so
    /// the tests assert against a shape they know rather than against whatever
    /// the sky was doing when a fixture was taken.
    fn test_oval() -> AuroraOval {
        let mut grid = vec![0u8; aurora::GRID_W * aurora::GRID_H];
        for row in 0..aurora::GRID_H {
            let lat = 90.0 - row as f64;
            if (60.0..=75.0).contains(&lat.abs()) {
                for col in 0..aurora::GRID_W {
                    grid[row * aurora::GRID_W + col] = 40;
                }
            }
        }
        AuroraOval { observed_unix: 0, forecast_unix: 0, grid }
    }

    fn earth_view_with_aurora(oval: Option<AuroraOval>) -> (SolarUi, SolarData) {
        let mut st = ui();
        st.view.focus = Focus::Earth.to_id();
        st.view.dist = 0.5;
        let mut data = SolarData::default();
        data.aurora = oval.map(std::sync::Arc::new);
        (st, data)
    }

    /// The shells have to be a *thin* shell of atmosphere standing on the
    /// surface. If they drifted to satellite altitudes the oval would detach
    /// from the globe and stop meaning anything.
    #[test]
    fn the_aurora_is_a_stack_of_shells_in_the_upper_atmosphere() {
        let (st, data) = earth_view_with_aurora(Some(test_oval()));
        let now = 1_784_937_600.0;
        let b = bodies(&st, now);
        let earth_px = Camera::from_view(&st, &b, [1600.0, 900.0]).pixels_for(b.earth, b.earth_r);
        let s = build(&st, Some(&data), now, [1600.0, 900.0], 0.0);

        let shells: Vec<_> = s.draws.iter().filter(|(p, _)| *p == Prim::Aurora).collect();
        assert_eq!(shells.len(), shell_count(earth_px), "wrong number of shells");

        let mut prev_alt = 0.0f32;
        let mut slab_total = 0.0f32;
        for (_, d) in &shells {
            let (alt, slab, intensity) = (d.params[0], d.params[1], d.params[2]);
            assert!(
                (AURORA_BOTTOM_KM..=AURORA_TOP_KM).contains(&alt),
                "shell at {alt} km is outside the emission band"
            );
            assert!(alt > prev_alt, "shells are not ordered by altitude");
            prev_alt = alt;
            assert!(slab > 0.0 && intensity > 0.0);
            slab_total += slab;

            // Scale is the first column's length, and it is the *rendered*
            // radius, so the altitude must be a fraction of the globe's own
            // radius however exaggerated that is.
            let scale =
                (d.model[0][0].powi(2) + d.model[0][1].powi(2) + d.model[0][2].powi(2)).sqrt();
            let ratio = scale / b.earth_r;
            assert!((1.014..1.064).contains(&ratio), "shell at {ratio}× the Earth's radius");
        }
        // The slabs tile the band rather than overlapping or leaving gaps.
        let band = AURORA_TOP_KM - AURORA_BOTTOM_KM;
        assert!(
            (slab_total - band).abs() < band * 0.15,
            "slabs total {slab_total} km over a {band} km band"
        );

        // Bodies first, then the shells: the draw loop only rebinds a pipeline
        // when the primitive changes, so an interleaved order would cost a
        // switch per shell.
        let first = s.draws.iter().position(|(p, _)| *p == Prim::Aurora).unwrap();
        assert!(first >= 3, "aurora drawn before the Sun, Earth and Moon");
        assert!(s.draws[first..].iter().all(|(p, _)| *p == Prim::Aurora));
    }

    #[test]
    fn fewer_shells_are_spent_on_a_smaller_earth() {
        assert!(shell_count(600.0) > shell_count(120.0));
        assert!(shell_count(120.0) > shell_count(40.0));
        assert!(shell_count(40.0) > shell_count(4.0));
        assert!(shell_count(0.0) >= 2, "an oval drawn with fewer than two shells has no depth");
    }

    /// The contour is the number an operator compares their own latitude
    /// against, so it must land on the band the data actually has.
    #[test]
    fn the_edge_contour_lands_on_the_oval() {
        let (st, data) = earth_view_with_aurora(Some(test_oval()));
        let now = 1_784_937_600.0;
        let b = bodies(&st, now);
        let (_, _, ez) = b.earth_basis;
        let (plain, _) = earth_view_with_aurora(None);
        let base = build(&plain, Some(&SolarData::default()), now, [1600.0, 900.0], 0.0);
        let s = build(&st, Some(&data), now, [1600.0, 900.0], 0.0);

        let added = &s.lines[base.lines.len()..];
        // Two rings, sampled every 4°: 90 segments each.
        assert_eq!(added.len(), 180, "expected a closed ring in each hemisphere");
        let mut north = 0;
        for l in added {
            let p = v3(l.a[0], l.a[1], l.a[2]) - b.earth;
            let r = p.len() / b.earth_r;
            assert!((1.0..1.01).contains(&r), "contour at {r} Earth radii — not on the surface");
            let lat = (p.dot(ez) / p.len()).clamp(-1.0, 1.0).asin().to_degrees();
            // The band's equatorward edge, wherever the sampler put it.
            assert!((59.0..61.0).contains(&lat.abs()), "contour at {lat}°");
            north += (lat > 0.0) as usize;
        }
        assert_eq!(north, 90, "the two hemispheres should contribute equally");
    }

    #[test]
    fn the_aurora_layer_removes_the_oval_entirely() {
        let (mut st, data) = earth_view_with_aurora(Some(test_oval()));
        st.view.layers &= !layer::AURORA;
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| *p != Prim::Aurora));
    }

    /// A quiet night is not the same as no data: an all-zero grid must draw
    /// nothing at all rather than a ring of black shells over the poles.
    #[test]
    fn a_quiet_oval_draws_nothing() {
        let empty = AuroraOval {
            observed_unix: 0,
            forecast_unix: 0,
            grid: vec![0u8; aurora::GRID_W * aurora::GRID_H],
        };
        let (st, data) = earth_view_with_aurora(Some(empty));
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| *p != Prim::Aurora));
    }

    /// From out by the Sun the Earth is a fraction of a pixel, and a polar band
    /// on it cannot be seen — the shells there are pure overdraw.
    #[test]
    fn the_oval_fades_out_with_a_distant_earth() {
        let mut st = ui();
        st.view.focus = Focus::Sun.to_id();
        let mut data = SolarData::default();
        data.aurora = Some(std::sync::Arc::new(test_oval()));
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| *p != Prim::Aurora));
    }

    #[test]
    fn stars_are_a_fixed_reproducible_sphere() {
        let a = stars();
        let b = stars();
        assert_eq!(a.len(), 1500);
        assert!(a.iter().zip(&b).all(|(x, y)| x.center == y.center));
        for s in &a {
            let r = (s.center[0].powi(2) + s.center[1].powi(2) + s.center[2].powi(2)).sqrt();
            assert!((r - 300_000.0).abs() < 1.0, "star at radius {r}");
            assert_eq!(s.params[0], SPRITE_STAR);
        }
        // Roughly isotropic: no hemisphere should hold more than 60%.
        let up = a.iter().filter(|s| s.center[2] > 0.0).count();
        assert!((600..900).contains(&up), "{up}/1500 stars in the +Z hemisphere");
    }

    // ── Clouds ──────────────────────────────────────────────────────────────

    /// A synthetic field: an overcast band across the tropics with one deep
    /// convective cell in it, so these tests assert against a shape they know.
    fn test_field() -> CloudField {
        let n = clouds::GRID_W * clouds::GRID_H;
        let mut opacity = vec![0u8; n];
        let mut top = vec![0u8; n];
        for row in 0..clouds::GRID_H {
            let lat = clouds::row_lat(row);
            if lat.abs() > 25.0 {
                continue;
            }
            for col in 0..clouds::GRID_W {
                let i = row * clouds::GRID_W + col;
                opacity[i] = 200;
                // A tall tower over the Congo, a flat deck everywhere else.
                let lon = clouds::col_lon(col);
                let tall = (lat - 0.0).abs() < 5.0 && (lon - 20.0).abs() < 5.0;
                top[i] = if tall { 240 } else { 60 };
            }
        }
        CloudField {
            frame_unix: 1_784_937_600,
            opacity,
            top,
            cells: vec![clouds::ConvCell {
                lat: 0.0,
                lon: 20.0,
                radius_deg: 4.0,
                top_km: 16.0,
                flash_rate: 2.0,
            }],
            has_visible: true,
        }
    }

    fn earth_view_with_clouds(field: Option<CloudField>) -> (SolarUi, SolarData) {
        let mut st = ui();
        st.view.focus = Focus::Earth.to_id();
        st.view.dist = 0.5;
        let mut data = SolarData::default();
        data.clouds = field.map(std::sync::Arc::new);
        (st, data)
    }

    /// The deck is a stack of slices through the troposphere, and every one of
    /// them has to sit in the band the weather is actually in — ordered, tiling
    /// it, and glued to the surface.
    ///
    /// The loop over `body_scale` is the one that matters. Altitudes are stored
    /// as fractions of the radius the globe is *drawn* at, so the deck has to
    /// stay attached at any setting of the exaggeration slider; expressing them
    /// in absolute units instead would leave the clouds buried inside a
    /// twenty-times Earth and orbiting a life-size one.
    #[test]
    fn the_deck_is_a_stack_of_slices_through_the_troposphere() {
        for scale in [1.0f32, 20.0, 60.0] {
            let (mut st, data) = earth_view_with_clouds(Some(test_field()));
            st.view.body_scale = scale;
            let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
            let deck: Vec<_> =
                s.draws.iter().filter(|(p, _)| *p == Prim::Cloud).map(|(_, d)| *d).collect();
            assert!(deck.len() >= 4, "only {} shells at scale {scale}", deck.len());

            let mut last = 0.0f32;
            let mut spanned = 0.0f32;
            for d in &deck {
                let alt = d.params[0];
                let slab = d.params[1];
                assert!(
                    (CLOUD_BOTTOM_KM..=CLOUD_TOP_KM).contains(&alt),
                    "a shell at {alt} km is outside the troposphere"
                );
                assert!(alt >= last, "shells are not ordered bottom-up: {alt} after {last}");
                assert!(slab > 0.0, "a shell standing for no air at all");
                last = alt;
                spanned += slab;

                // Radius relative to the globe: a hair above the surface, and
                // never so far off it that the deck reads as a separate shell.
                let r = v3(d.model[0][0], d.model[0][1], d.model[0][2]).len();
                let earth_r = ephem::EARTH_R as f32 * scale;
                let rel = r / earth_r;
                assert!(
                    (1.0..=1.02).contains(&rel),
                    "shell at {rel}× the Earth's radius at scale {scale}"
                );
            }
            // The slabs have to add up to the band, or the deck's opacity would
            // depend on how many shells the frame happened to spend.
            let band = CLOUD_TOP_KM - CLOUD_BOTTOM_KM;
            assert!(
                (spanned - band).abs() < band * 0.35,
                "slabs sum to {spanned} km against a {band} km band"
            );
        }
    }

    #[test]
    fn the_clouds_layer_removes_the_deck_entirely() {
        let (mut st, data) = earth_view_with_clouds(Some(test_field()));
        st.view.layers &= !layer::CLOUDS;
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| !matches!(p, Prim::Cloud | Prim::CloudVolume)));
        assert_eq!(s.flashes, Flashes::default(), "lightning without a cloud layer");
    }

    /// No field, no deck. Drawing a clear sky before the first mosaic lands
    /// would be a claim about the weather rather than an absence of one.
    #[test]
    fn no_mosaic_draws_no_weather() {
        let (st, data) = earth_view_with_clouds(None);
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| !matches!(p, Prim::Cloud | Prim::CloudVolume)));
    }

    /// The volume switch replaces the whole stack with a single draw.
    #[test]
    fn the_volume_switch_swaps_the_stack_for_one_march() {
        let (mut st, data) = earth_view_with_clouds(Some(test_field()));
        st.view.cloud_march = true;
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        let marches: Vec<_> =
            s.draws.iter().filter(|(p, _)| *p == Prim::CloudVolume).map(|(_, d)| *d).collect();
        assert_eq!(marches.len(), 1);
        assert!(s.draws.iter().all(|(p, _)| *p != Prim::Cloud));
        // The step budget has to be a usable number of steps, and within the
        // ceiling the shader's loop is bounded by.
        let steps = marches[0].params[3];
        assert!((4.0..=40.0).contains(&steps), "step budget {steps}");
    }

    /// Cloud is under the aurora, and the aurora only adds light — so the deck
    /// has to be composited first or the oval ends up behind the weather.
    #[test]
    fn the_deck_is_drawn_under_the_aurora() {
        let mut st = ui();
        st.view.focus = Focus::Earth.to_id();
        st.view.dist = 0.5;
        let mut data = SolarData::default();
        data.clouds = Some(std::sync::Arc::new(test_field()));
        data.aurora = Some(std::sync::Arc::new(test_oval()));
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);

        let last_cloud = s.draws.iter().rposition(|(p, _)| *p == Prim::Cloud).expect("no deck");
        let first_aurora = s.draws.iter().position(|(p, _)| *p == Prim::Aurora).expect("no oval");
        assert!(last_cloud < first_aurora, "the aurora was drawn under the clouds");
        // Each run stays contiguous, or the draw loop rebinds a pipeline per
        // shell instead of once per stack.
        let first_cloud = s.draws.iter().position(|(p, _)| *p == Prim::Cloud).expect("no deck");
        assert!(s.draws[first_cloud..=last_cloud].iter().all(|(p, _)| *p == Prim::Cloud));
    }

    /// A cloud deck on a twenty-pixel Earth is a grey wash that only makes the
    /// planet harder to read, so it fades out before it gets there.
    #[test]
    fn the_deck_fades_out_with_a_distant_earth() {
        let mut st = ui();
        st.view.focus = Focus::Sun.to_id();
        let mut data = SolarData::default();
        data.clouds = Some(std::sync::Arc::new(test_field()));
        let s = build(&st, Some(&data), 1_784_937_600.0, [1600.0, 900.0], 0.0);
        assert!(s.draws.iter().all(|(p, _)| !matches!(p, Prim::Cloud | Prim::CloudVolume)));
        assert_eq!(s.flashes, Flashes::default());
    }

    /// Lightning is a function of the clock and nothing else.
    ///
    /// This is what lets the time chips scrub backwards and replay the same
    /// storm rather than a fresh one, and it is what makes the model testable at
    /// all. A frame counter or an RNG would give a different answer every time
    /// the same instant was drawn.
    #[test]
    fn lightning_is_a_function_of_the_clock() {
        let (st, data) = earth_view_with_clouds(Some(test_field()));
        let at = |t: f64| build(&st, Some(&data), t, [1600.0, 900.0], 0.0).flashes;

        let a = at(1_784_937_600.0);
        assert_eq!(a, at(1_784_937_600.0), "the same instant gave two different storms");

        // Over a minute at two flashes a second something has to strike, and the
        // picture has to change as the clock runs.
        let mut lit_frames = 0;
        let mut seen = std::collections::HashSet::new();
        for i in 0..600 {
            let f = at(1_784_937_600.0 + i as f64 * 0.1);
            let lit = f.items.iter().filter(|s| s[3] > 0.0).count();
            assert!(lit <= MAX_FLASHES, "{lit} flashes at once");
            if lit > 0 {
                lit_frames += 1;
                seen.insert(f.items[0][3].to_bits());
            }
        }
        assert!(lit_frames > 20, "only {lit_frames}/600 frames had any lightning");
        assert!(seen.len() > 10, "the flashes never changed brightness");
    }

    /// A flash is inside the cloud that is making it: within the slab, and near
    /// enough to the storm the field put there.
    #[test]
    fn lightning_strikes_inside_the_storm_that_made_it() {
        let (st, data) = earth_view_with_clouds(Some(test_field()));
        let b = bodies(&st, 1_784_937_600.0);
        let cell = data.clouds.as_ref().expect("field").cells[0];
        let want = b.surface_dir(cell.lat as f64, cell.lon as f64);

        let mut checked = 0;
        for i in 0..600 {
            let f = build(&st, Some(&data), 1_784_937_600.0 + i as f64 * 0.1, [1600.0, 900.0], 0.0)
                .flashes;
            for item in f.items.iter().filter(|s| s[3] > 0.0) {
                let p = v3(item[0], item[1], item[2]) - b.earth;
                let rel = p.len() / b.earth_r;
                let top = 1.0 + CLOUD_TOP_KM * CLOUD_LIFT / CLOUD_EARTH_R_KM;
                assert!((1.0..=top).contains(&rel), "a flash at {rel}× the Earth's radius");
                // Within the storm's own footprint, generously: the jitter is
                // 70% of its radius and the tower leans nowhere.
                let cos = p.normalize().dot(want);
                let limit = (cell.radius_deg * 1.2).to_radians().cos();
                assert!(cos > limit, "a flash {:.1}° from its storm", cos.acos().to_degrees());
                checked += 1;
            }
        }
        assert!(checked > 20, "only {checked} flashes to check");
    }

    /// A storm that is not working does not flash. The rate comes out of the
    /// imagery, so a zero there has to mean silence rather than a default.
    #[test]
    fn a_quiet_cell_never_flashes() {
        let mut field = test_field();
        field.cells[0].flash_rate = 0.0;
        let (st, data) = earth_view_with_clouds(Some(field));
        for i in 0..200 {
            let f =
                build(&st, Some(&data), 1_784_937_600.0 + i as f64 * 0.05, [1600.0, 900.0], 0.0)
                    .flashes;
            assert!(f.items.iter().all(|s| s[3] == 0.0), "a cell with no rate flashed");
        }
    }

    /// The flash block is a uniform, so its size is a contract with the shaders.
    #[test]
    fn the_flash_block_is_the_size_the_shaders_expect() {
        assert_eq!(std::mem::size_of::<Flashes>(), 16 * (MAX_FLASHES + 1));
        assert_eq!(std::mem::size_of::<Flashes>() % 16, 0);
    }

    /// `SUN OBS` is one chip standing for two layers, so it has to gate both —
    /// and a mask that a settings file left with only one of them set must read
    /// as lit and clear to nothing, not flip to the other one. That is the bug
    /// an XOR toggle would have.
    #[test]
    fn the_sun_observation_chip_covers_both_of_its_layers() {
        assert_eq!(layer::SUN_OBS, layer::SPOTS | layer::FLARES);

        let mut st = ui();
        st.view.layers = layer::SPOTS;
        assert!(st.layer(layer::SUN_OBS), "half a pair should still light the chip");
        st.set_layers(layer::SUN_OBS, false);
        assert!(!st.layer(layer::SPOTS) && !st.layer(layer::FLARES), "one click left half on");
        st.set_layers(layer::SUN_OBS, true);
        assert!(st.layer(layer::SPOTS) && st.layer(layer::FLARES), "the next click left half off");
    }
}
