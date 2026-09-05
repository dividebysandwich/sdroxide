//! The orbit camera.
//!
//! Yaw/pitch around a focus body with a **fixed world up** (ecliptic north)
//! rather than a free arcball. A tumbling arcball lets the user roll the
//! ecliptic to an arbitrary angle and never find "overhead" again; keeping the
//! ecliptic horizontal is most of what makes these views legible.

use super::math::{M4, V3, v3};
use super::scene::Bodies;
use super::state::{Focus, SolarUi};

pub const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;
/// Pitch limit, just short of the pole where look-at degenerates.
pub const PITCH_LIMIT: f32 = 1.552;
/// Farthest the camera may pull back, gigametres (≈80 AU).
///
/// Enough to frame Neptune's orbit — 30 AU across — with room to spare, which
/// is the outermost thing the view draws.
pub const MAX_DIST: f32 = 12_000.0;
/// Far plane. Reversed-Z makes this ratio to the near plane a non-issue.
const FAR: f32 = 1.0e6;

pub struct Camera {
    pub view_proj: M4,
    pub eye: V3,
    pub near: f32,
    height_px: f32,
}

/// Unit vector from the focus toward the eye.
pub fn orbit_dir(yaw: f32, pitch: f32) -> V3 {
    v3(pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), pitch.sin())
}

/// Distance limits for the current focus: never inside the body, never so far
/// that the scene degenerates to a point.
pub fn dist_range(focus_radius: f32) -> (f32, f32) {
    ((focus_radius * 1.6).max(1e-4), MAX_DIST)
}

impl Camera {
    /// A camera that is only an eye position, for tests of the geometry that
    /// depends on nothing else — [`crate::solar3d::scene::behind_earth`] and
    /// its like.
    #[cfg(test)]
    pub fn test_at(eye: V3) -> Camera {
        Camera { view_proj: M4::IDENTITY, eye, near: 1e-4, height_px: 900.0 }
    }

    pub fn from_view(st: &SolarUi, b: &Bodies, size_px: [f32; 2]) -> Camera {
        let v = &st.view;
        // While the tour is flying between two stations it supplies a blended
        // pivot. Without it the pivot would snap to the destination body on the
        // transition's first frame — Sun to Earth is 1 AU — and the camera would
        // teleport even though yaw, pitch and distance interpolate smoothly.
        let (focus, radius) = st.focus_override.unwrap_or_else(|| b.focus(st.focus()));
        let (lo, hi) = dist_range(radius);
        let dist = v.dist.clamp(lo, hi);
        let eye = focus + orbit_dir(v.yaw, v.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT)) * dist;

        // Track the near plane to the viewing distance: at 1 AU a fixed near
        // plane either clips the body being examined or throws away the
        // precision the close-up views need.
        let near = (dist * 0.0015).clamp(1e-5, 0.5);
        let aspect = size_px[0] / size_px[1].max(1.0);
        let proj = M4::perspective_reversed_z(FOV_Y, aspect, near, FAR);
        let view = M4::look_at(eye, focus, v3(0.0, 0.0, 1.0));

        Camera { view_proj: proj.mul(&view), eye, near, height_px: size_px[1].max(1.0) }
    }

    /// Apparent radius of a sphere, in pixels — used to give every body a
    /// minimum on-screen size regardless of the exaggeration setting.
    pub fn pixels_for(&self, pos: V3, radius: f32) -> f32 {
        let d = (pos - self.eye).len().max(1e-9);
        (radius / d) / (FOV_Y * 0.5).tan() * (self.height_px * 0.5)
    }
}

// ── The AUTO tour ───────────────────────────────────────────────────────────

/// One framed viewpoint in the tour.
///
/// Orientations are given *relative to a live direction* wherever the
/// composition depends on one (the Sun's bearing from the Earth, say), so a
/// station holds its framing as the bodies move rather than drifting out of it
/// over the minutes the loop takes.
pub struct Station {
    /// Read only by [`Tour::leg_name`], and so only by the tour's own tests now
    /// that no readout names the leg.
    #[allow(dead_code)]
    pub name: &'static str,
    pub focus: Focus,
    /// Yaw relative to `relative_to`, radians.
    pub yaw_offset: f32,
    /// Pitch above `relative_to`'s own elevation, radians — which is the
    /// ecliptic plane for every bearing but the operator's QTH.
    pub pitch: f32,
    /// What the yaw is measured from.
    pub relative_to: Bearing,
    /// Distance, as a multiple of the focus body's radius.
    pub radii: f32,
    pub dwell_s: f32,
    /// Slow yaw drift while holding the station, radians/second. Keeps a long
    /// dwell from reading as a frozen frame.
    pub drift: f32,
    /// Distance at the *end* of the dwell, in radii: the shot dollies from
    /// `radii` to this across the whole hold, eased at both ends so it leaves
    /// and arrives at rest. `None` holds the distance it flew in at.
    pub radii_end: Option<f32>,
    /// Pitch at the end of the dwell, the same way. This is how a station
    /// travels *towards* something during its hold rather than turning on the
    /// spot: the camera climbs from one side of a place to over the top of it.
    pub pitch_end: Option<f32>,
    /// How far the focus body is pushed below the centre of the frame, in its
    /// own radii.
    ///
    /// `1.0` lays its horizon exactly across the middle — the view axis then
    /// grazes the sphere, whatever the distance — so the body fills the lower
    /// half of the screen and space the upper. `0.0` centres it, which is what
    /// every wide shot wants.
    pub drop: f32,
    /// How much of the framing is drawn afresh each lap.
    pub jitter: Jitter,
}

/// The spread a station is composed within, rather than at.
///
/// A shot that is meant to be *a* descent rather than *the* descent gets one;
/// every other station is [`FIXED`] and frames identically every lap, the way a
/// scripted station should.
#[derive(Clone, Copy)]
pub struct Jitter {
    /// Azimuth spread, radians. Shared by both ends of a dolly, so the move
    /// happens at one bearing instead of swinging round mid-shot.
    pub yaw: f32,
    /// Pitch spread, radians, drawn separately for each end of a dolly.
    pub pitch: f32,
    /// Spread on how far out the shot *starts*, as a fraction of its own
    /// distance. Only the start: where a dolly is going is the point of it,
    /// and a descent that sometimes stopped short of orbit would just read as
    /// one that had lost its nerve.
    pub dist: f32,
}

/// A station that composes the same way on every lap.
pub const FIXED: Jitter = Jitter { yaw: 0.0, pitch: 0.0, dist: 0.0 };

/// The live direction a station's yaw is measured against.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bearing {
    /// The Sun's bearing as seen from the Earth.
    SunFromEarth,
    /// The Earth's bearing as seen from the Sun.
    EarthFromSun,
    /// The Moon's bearing as seen from the Earth.
    MoonFromEarth,
    /// The operator's own QTH, as seen from the Earth's centre.
    ///
    /// The only bearing that carries an **elevation** as well: a station hung
    /// off it is framed at the operator's latitude rather than out on the
    /// ecliptic equator, which is the entire point of aiming at somebody's
    /// grid square. With no grid entered it degenerates to a fixed direction in
    /// the ecliptic frame, which still composes — it is simply aimed at nobody.
    HomeFromEarth,
}

const DEG: f32 = std::f32::consts::PI / 180.0;

/// Where the day/night line lies, as a yaw offset from the Sun's bearing.
///
/// The Earth turns anticlockwise seen from the north, so a place 90° round from
/// the subsolar point *in the direction it is turning* has had its noon already
/// and is running out of daylight: that limb is the evening one. The other is
/// the morning. Both are exactly edge-on to the terminator plane at any pitch,
/// because the Sun's bearing lies in the ecliptic the camera measures from.
const DUSK: f32 = 90.0 * DEG;
const DAWN: f32 = -90.0 * DEG;

/// The tour, in order. Between them the camera eases for [`TRANSITION_S`].
///
/// One shot of the Sun itself, and the rest of the loop over the planet the
/// operator is actually working from — which is the one the log, the greyline
/// and the satellites are all about.
pub const STATIONS: &[Station] = &[
    Station {
        name: "SUNSIDE",
        focus: Focus::Sun,
        // Face-on to the disk SDO photographs, from the Earth's direction.
        yaw_offset: 0.0,
        pitch: 6.0 * DEG,
        relative_to: Bearing::EarthFromSun,
        radii: 3.4,
        dwell_s: 16.0,
        drift: 0.5 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "EARTH SHOULDER",
        focus: Focus::Earth,
        // Behind the Earth, looking past it at the Sun.
        yaw_offset: 180.0 * DEG,
        pitch: 22.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 14.0,
        dwell_s: 12.0,
        drift: 0.9 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "DUSK LINE",
        focus: Focus::Earth,
        // Past the evening limb, so the frame is mostly the night coming on and
        // the sunset line runs down the lit edge of it.
        yaw_offset: DUSK + 15.0 * DEG,
        pitch: 37.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 2.1,
        dwell_s: 11.0,
        drift: -0.8 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "HOME ORBIT",
        focus: Focus::Earth,
        // Low over the operator's own meridian, closing on the grid square from
        // well south of it across the whole dwell. It stops short rather than
        // flying over the top: a place directly beneath the camera is off the
        // bottom of a frame whose horizon sits on the middle of the screen.
        yaw_offset: 0.0,
        pitch: -40.0 * DEG,
        relative_to: Bearing::HomeFromEarth,
        radii: 0.78,
        dwell_s: 20.0,
        drift: 0.3 * DEG,
        radii_end: Some(0.64),
        pitch_end: Some(-16.0 * DEG),
        drop: 1.0,
        jitter: FIXED,
    },
    Station {
        name: "LUNAR DIAGONAL",
        focus: Focus::EarthMoon,
        yaw_offset: 55.0 * DEG,
        pitch: 34.0 * DEG,
        relative_to: Bearing::MoonFromEarth,
        radii: 2.6,
        dwell_s: 10.0,
        drift: 1.3 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "TERMINATOR",
        focus: Focus::Earth,
        // Side-on to the day/night line, shallow and wide — the greyline as the
        // propagation panels draw it.
        yaw_offset: DUSK,
        pitch: 14.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 3.4,
        dwell_s: 12.0,
        drift: 1.1 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "ORBIT DESCENT",
        focus: Focus::Earth,
        // The long fall in: from far enough out that the Earth is a marble, down
        // to orbit, on a bearing and a starting height drawn fresh each lap so
        // it is never twice the same descent. Kept off the far night side, so
        // there is always daylight on the globe it is falling towards.
        yaw_offset: 20.0 * DEG,
        pitch: 44.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 26.0,
        dwell_s: 21.0,
        drift: 0.45 * DEG,
        radii_end: Some(1.15),
        pitch_end: Some(7.0 * DEG),
        drop: 0.55,
        jitter: Jitter { yaw: 100.0 * DEG, pitch: 13.0 * DEG, dist: 0.45 },
    },
    Station {
        name: "DAWN LINE",
        focus: Focus::Earth,
        // The other side of the same line, from below the ecliptic and out over
        // the dark: the ground in frame is the ground about to have sunrise.
        yaw_offset: DAWN - 15.0 * DEG,
        pitch: -31.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 2.3,
        dwell_s: 11.0,
        drift: 0.8 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "SOLAR VANTAGE",
        focus: Focus::Earth,
        // From out by the Sun, looking back at the Earth.
        yaw_offset: 0.0,
        pitch: 3.0 * DEG,
        relative_to: Bearing::SunFromEarth,
        radii: 90.0,
        dwell_s: 12.0,
        drift: 0.35 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
    Station {
        name: "SYSTEM ANGLED",
        focus: Focus::Sun,
        // Back off far enough for the outer planets' orbits to be in frame, and
        // hold it well off the ecliptic so the orbits read as rings rather than
        // as the single line an edge-on view collapses them to.
        yaw_offset: 25.0 * DEG,
        pitch: 30.0 * DEG,
        relative_to: Bearing::EarthFromSun,
        radii: 2400.0,
        dwell_s: 14.0,
        drift: 0.55 * DEG,
        radii_end: None,
        pitch_end: None,
        drop: 0.0,
        jitter: FIXED,
    },
];

/// The closest the eye may come to the body it is orbiting, in that body's
/// radii, for a station that deliberately flies inside the camera's ordinary
/// framing limit.
///
/// [`dist_range`] keeps the camera 1.6 radii out, which is what it takes to
/// hold a whole globe in frame. An orbital shot is *meant* to be inside that —
/// but not inside the atmosphere.
const ORBIT_FLOOR: f32 = 1.08;

/// Ease between stations. Long enough to read as a camera move, short enough
/// not to be most of the loop.
pub const TRANSITION_S: f32 = 3.2;

// ── The contact being worked ────────────────────────────────────────────────

/// The two ends of a contact to frame: the operator's QTH and the station being
/// worked, each (latitude, longitude) in degrees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct QsoPath {
    pub home: (f64, f64),
    pub dx: (f64, f64),
}

/// How far off vertical the camera looks down on the arc's midpoint.
///
/// Straight overhead — 0° — is the worst possible view of a path: the arc's
/// rise is then entirely along the line of sight, so it collapses onto the
/// surface, and the globe reads as a flat disc. Fully side-on, 90°, shows the
/// rise at its full height but leaves both stations sitting exactly on the
/// limb. This sits nearer the side-on end: a shallow enough angle that the
/// horizon curves visibly across the frame and the arc plainly springs off the
/// surface, while both ends stay inside the limb.
const QSO_TILT: f32 = 62.0 * DEG;

/// Fraction of the frame's half-height the framed path may fill. The remainder
/// is margin — which is what keeps the two station markers clear of the edges.
const QSO_FILL: f32 = 0.66;

/// Closest the contact view may get, as a multiple of the Earth's rendered
/// radius. Two stations a few hundred kilometres apart would otherwise pull the
/// camera down to a patch of surface with no horizon in it at all.
const QSO_MIN_DIST: f32 = 0.6;

/// Dwell sway for the contact view: amplitude, and phase rate in radians per
/// second (a round trip every ~24 s).
///
/// The stations drift by a constant yaw, which is fine for a fixed dwell. A
/// contact is held for as long as it lasts, so a constant drift would walk the
/// path out of the frame; swaying keeps the shot alive without leaving it.
const QSO_SWAY: f32 = 5.0 * DEG;
const QSO_SWAY_RATE: f32 = 0.26;

/// Camera pose and pivot that frame a contact: both stations and the arc
/// between them, seen from off to one side of the great circle they lie on.
///
/// `None` when the two ends coincide — which is also when no arc is drawn.
pub fn qso_frame(p: QsoPath, b: &Bodies) -> Option<(Pose, (V3, f32))> {
    let a = b.surface_dir(p.home.0, p.home.1);
    let c = b.surface_dir(p.dx.0, p.dx.1);
    let omega = a.dot(c).clamp(-1.0, 1.0).acos();
    if !omega.is_finite() || omega < 1e-3 {
        return None;
    }

    // The plane the path lies in. Both of these degenerate for an exactly
    // antipodal pair — which has no unique great circle between its ends — so
    // each falls back to an arbitrary but well-defined choice rather than
    // handing the camera a normalised zero vector.
    let n = a.cross(c);
    let normal = if n.len() > 1e-5 { n.normalize() } else { any_perp(a) };
    let s = a + c;
    // Midpoint of the arc: the direction its apex sits over.
    let mid = if s.len() > 1e-5 { s.normalize() } else { normal.cross(a).normalize() };

    let bulge = super::scene::arc_bulge(omega as f64);
    // Pivot half way up the bulge, so the arc's peak and the two ends it
    // springs from straddle the centre of the frame rather than one of them
    // taking it.
    let pivot = b.earth + mid * (b.earth_r * (1.0 + bulge * 0.5));
    // Tilting towards the plane's *normal* is what keeps the two ends
    // equidistant from the view axis: they sit either side of the frame's
    // centre however long the path is.
    let dir = mid * QSO_TILT.cos() + normal * QSO_TILT.sin();

    // Everything the shot has to hold: the two stations, and the top of the arc.
    let span = [a * b.earth_r, c * b.earth_r, mid * (b.earth_r * (1.0 + bulge))]
        .iter()
        .map(|p| (b.earth + *p - pivot).len())
        .fold(0.0f32, f32::max);
    let dist = (span / ((FOV_Y * 0.5).tan() * QSO_FILL)).max(b.earth_r * QSO_MIN_DIST);

    Some((
        Pose::new(dir.y.atan2(dir.x), dir.z.clamp(-1.0, 1.0).asin(), dist),
        // The clamp radius is the size of what is being framed rather than the
        // Earth's, or a short hop could never be approached closely enough to
        // see.
        (pivot, span.max(b.earth_r * 0.15)),
    ))
}

// ── The satellite being worked ──────────────────────────────────────────────

/// The two ends of a satellite lock to frame: the operator's QTH (latitude,
/// longitude, degrees) and the satellite's position in world space — computed
/// by the caller, which is the only place the tracker's data is at hand.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SatPath {
    pub home: (f64, f64),
    pub sat_world: V3,
}

/// Shortest slant range the satellite view frames as if it were this long, as
/// a multiple of the Earth's rendered radius. An overhead LEO pass is a chord
/// a few hundred kilometres long — invisible at any distance that also shows
/// the planet it is happening over.
const SAT_MIN_SPAN: f32 = 0.35;

/// Camera pose and pivot that frame a satellite lock: the QTH, the satellite,
/// and the line between them, seen from off to one side — the satellite
/// sibling of [`qso_frame`], without the arc bulge (the line is a sightline,
/// not a great circle).
///
/// `None` for a satellite the tracker has lost (inside the Earth, NaN).
pub fn sat_frame(p: SatPath, b: &Bodies) -> Option<(Pose, (V3, f32))> {
    let a = b.surface_dir(p.home.0, p.home.1);
    let home = b.earth + a * b.earth_r;
    let rel = p.sat_world - b.earth;
    let r = rel.len();
    if !r.is_finite() || r < b.earth_r * 0.5 {
        return None;
    }
    let c = rel * (1.0 / r);

    // The plane holding both ends. Degenerate when the satellite stands
    // exactly over the QTH — any side view is then as good as any other.
    let n = a.cross(c);
    let normal = if n.len() > 1e-4 { n.normalize() } else { any_perp(a) };
    let s = a + c;
    let mid = if s.len() > 1e-4 { s.normalize() } else { normal.cross(a).normalize() };

    // Pivot on the sightline's midpoint, so QTH and satellite straddle the
    // frame's centre — for QO-100 that is 3⅓ Earth radii out, for an overhead
    // ISS it is barely off the surface, and both are correct.
    let pivot = (home + p.sat_world) * 0.5;
    let dir = mid * QSO_TILT.cos() + normal * QSO_TILT.sin();
    let span = (home - pivot).len().max((p.sat_world - pivot).len()).max(b.earth_r * SAT_MIN_SPAN);
    let dist = (span / ((FOV_Y * 0.5).tan() * QSO_FILL)).max(b.earth_r * QSO_MIN_DIST);

    Some((
        Pose::new(dir.y.atan2(dir.x), dir.z.clamp(-1.0, 1.0).asin(), dist),
        (pivot, span.max(b.earth_r * 0.15)),
    ))
}

/// Some unit vector perpendicular to `v`, for the cases where the geometry
/// itself singles none out.
fn any_perp(v: V3) -> V3 {
    let alt = if v.z.abs() < 0.9 { v3(0.0, 0.0, 1.0) } else { v3(1.0, 0.0, 0.0) };
    v.cross(alt).normalize()
}

/// A camera pose in the space the tour interpolates.
///
/// Distance is carried as its **logarithm**: a linear ramp across a
/// hundredfold zoom spends nearly all its time at the far end and then lurches,
/// whereas equal steps in log space read as a constant rate of approach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub yaw: f32,
    pub pitch: f32,
    pub ln_dist: f32,
}

impl Pose {
    fn new(yaw: f32, pitch: f32, dist: f32) -> Pose {
        Pose { yaw, pitch, ln_dist: dist.max(1e-6).ln() }
    }

    fn apply(self, view: &mut crate::view::Solar3dView) {
        view.yaw = self.yaw;
        view.pitch = self.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        view.dist = self.ln_dist.exp().clamp(1e-5, MAX_DIST);
    }

    fn scaled(self, k: f32) -> Pose {
        Pose { yaw: self.yaw * k, pitch: self.pitch * k, ln_dist: self.ln_dist * k }
    }

    fn plus(self, o: Pose) -> Pose {
        Pose {
            yaw: self.yaw + o.yaw,
            pitch: self.pitch + o.pitch,
            ln_dist: self.ln_dist + o.ln_dist,
        }
    }
}

/// Uniform Catmull-Rom through `p1` and `p2`, shaped by their neighbours.
///
/// This is what makes the tour read as one continuous camera move rather than a
/// series of separate ones: the curve arrives at each station already heading
/// towards the next, so the path bends through the stations instead of forming
/// a corner at each.
fn catmull_rom(p0: Pose, p1: Pose, p2: Pose, p3: Pose, t: f32) -> Pose {
    let t2 = t * t;
    let t3 = t2 * t;
    // 0.5 · [ 2p1 + (−p0+p2)t + (2p0−5p1+4p2−p3)t² + (−p0+3p1−3p2+p3)t³ ]
    p1.scaled(2.0)
        .plus(p2.plus(p0.scaled(-1.0)).scaled(t))
        .plus(
            p0.scaled(2.0)
                .plus(p1.scaled(-5.0))
                .plus(p2.scaled(4.0))
                .plus(p3.scaled(-1.0))
                .scaled(t2),
        )
        .plus(p0.scaled(-1.0).plus(p1.scaled(3.0)).plus(p2.scaled(-3.0)).plus(p3).scaled(t3))
        .scaled(0.5)
}

/// `t³(6t² − 15t + 10)` — zero first *and* second derivative at both ends.
///
/// Used to reparameterise time along the spline, so the camera eases out of one
/// station and settles into the next with no visible kick, while still
/// following the spline's curved path in between.
fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Shortest signed angular path from `a` to `b`.
fn short_angle(a: f32, b: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

/// What the camera is currently flying to and holding.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Leg {
    /// The scripted tour; [`Tour::index`] says which station.
    Station,
    /// The contact being worked, which pre-empts the tour for as long as it
    /// lasts.
    Qso,
}

/// Where the tour is, and where the camera was when it started moving.
#[derive(Clone, Copy)]
pub struct Tour {
    /// The station currently being flown to (and then dwelt at).
    pub index: usize,
    /// Seconds since the move to `index` began.
    pub elapsed: f32,
    /// Pose the current move started from — normally the previous station, but
    /// an arbitrary one when the user re-enables AUTO mid-flight.
    from: Pose,
    /// Pivot the current move started from: position and the radius the
    /// distance clamp uses. Interpolated alongside the pose so the camera flies
    /// between bodies instead of jumping to the next one.
    from_focus: (V3, f32),
    /// True when `from` is a station pose, so the spline can use a real
    /// preceding control point instead of a duplicated one.
    from_is_station: bool,
    started: bool,
    /// Set when AUTO is switched back on: the next `step` picks up at the
    /// nearest station instead of flinging the camera across the system.
    resume_pending: bool,
    /// Which leg the camera is on. Switching between them starts a new move.
    leg: Leg,
    /// The pivot the last step handed back, so a switch between legs can fly
    /// from wherever the camera actually is rather than from the station it
    /// happened to be heading for.
    last_pivot: (V3, f32),
    /// How many times the loop has come round, and so which draw the stations
    /// with a [`Jitter`] are composing from.
    lap: u32,
    /// The operator's QTH, latitude and longitude in degrees — what
    /// [`Bearing::HomeFromEarth`] is measured against. Handed in every step,
    /// because the grid can be edited while the tour is flying.
    home: Option<(f64, f64)>,
}

impl Default for Tour {
    fn default() -> Self {
        Tour {
            index: 0,
            elapsed: 0.0,
            from: Pose { yaw: 0.0, pitch: 0.0, ln_dist: 0.0 },
            from_focus: (V3::ZERO, 1.0),
            from_is_station: false,
            started: false,
            resume_pending: false,
            leg: Leg::Station,
            last_pivot: (V3::ZERO, 1.0),
            lap: 0,
            home: None,
        }
    }
}

impl Tour {
    pub fn station(&self) -> &'static Station {
        &STATIONS[self.index % STATIONS.len()]
    }

    /// What the leg the camera is on would be called. Nothing on screen names
    /// it any more; the tour's tests are what read this, and it is how "the
    /// camera reached the contact and left again" is asserted at all.
    #[allow(dead_code)]
    pub fn leg_name(&self) -> &'static str {
        match self.leg {
            Leg::Qso => "QSO PATH",
            Leg::Station => self.station().name,
        }
    }

    /// Whether the camera is currently moving rather than holding a station.
    /// Test-only, like [`Tour::leg_name`].
    #[allow(dead_code)]
    pub fn in_transit(&self) -> bool {
        self.elapsed < TRANSITION_S
    }

    /// Advance the tour and write the camera pose into `view`.
    /// Ask for the tour to pick up near the current view on its next step.
    pub fn request_resume(&mut self) {
        self.resume_pending = true;
    }

    /// Advance the tour. Returns the pivot the camera should use this frame:
    /// during a transition an interpolated point between the two stations'
    /// bodies, and at rest simply the station's own body.
    ///
    /// `qso` is the contact being worked, if any. A contact pre-empts the tour
    /// entirely — the camera flies to it and holds it until the contact ends,
    /// because a path being worked *now* is the one thing on this globe worth
    /// watching more than the scripted loop. `sat` is the satellite lock, and
    /// it outranks even the contact: an operator locked onto a bird is
    /// working *through* it, so the bird is the show. `home` is the operator's
    /// own QTH, which the low pass is aimed at.
    pub fn step(
        &mut self,
        view: &mut crate::view::Solar3dView,
        b: &Bodies,
        dt: f32,
        qso: Option<QsoPath>,
        sat: Option<SatPath>,
        home: Option<(f64, f64)>,
    ) -> (V3, f32) {
        self.home = home;
        if std::mem::take(&mut self.resume_pending) {
            self.resume_near(view, b);
        }
        if !self.started {
            self.from = Pose::new(view.yaw, view.pitch, view.dist);
            // Start the flight from whatever the camera was actually looking
            // at, so enabling AUTO mid-view does not jump either.
            self.from_focus = b.focus(Focus::from_id(view.focus));
            self.last_pivot = self.from_focus;
            self.from_is_station = false;
            self.started = true;
            self.elapsed = 0.0;
        }

        // A contact that has no framing — both ends in the same place — is no
        // contact as far as the camera is concerned; the arc is not drawn for
        // one either. The satellite leg reuses the whole contact mechanism:
        // same hand-over, same sway, same held-forever dwell.
        let frame = sat.and_then(|p| sat_frame(p, b)).or_else(|| qso.and_then(|p| qso_frame(p, b)));
        let want = if frame.is_some() { Leg::Qso } else { Leg::Station };
        if want != self.leg {
            // Hand over from wherever the camera is at this instant, mid-flight
            // or not, so picking up a contact and giving it back are both
            // ordinary camera moves.
            self.from = Pose::new(view.yaw, view.pitch, view.dist);
            self.from_focus = self.last_pivot;
            self.from_is_station = false;
            self.elapsed = 0.0;
            if want == Leg::Station {
                // Rejoin the loop at whatever station is nearest rather than
                // where it was left, which by now may be on the far side of the
                // system.
                self.index = self.nearest_station(view, b);
            }
            self.leg = want;
        }

        // Clamped, so a stalled frame (a resize, a GPU hitch) does not teleport
        // the camera halfway through a move.
        self.elapsed += dt.clamp(0.0, 0.25);

        let station = self.station();
        let (target, target_focus) = match frame {
            Some(f) => f,
            None => {
                let p = self.pose_of(station, b);
                (p, self.focus_of(station, b, p))
            }
        };

        let pivot;
        if self.elapsed < TRANSITION_S {
            // p1 → p2 is this move; p0 and p3 shape its curvature.
            let p1 = self.from;
            let p2 = unwrap_to(p1, target);
            let p0 = if self.from_is_station {
                unwrap_to(p1, self.pose_of(self.station_at(self.index as isize - 2), b))
            } else {
                // Started from a manual pose: no history, so duplicate p1,
                // which gives the spline a zero incoming tangent (it eases out
                // of where the user left the camera).
                p1
            };
            // On the contact leg there is no station after this one to bend
            // towards; duplicating p2 gives a zero outgoing tangent, so the
            // camera settles onto the path instead of sweeping past it.
            let p3 = match frame {
                Some(_) => p2,
                None => unwrap_to(p2, self.pose_of(self.station_at(self.index as isize + 1), b)),
            };
            let k = smootherstep(self.elapsed / TRANSITION_S);
            catmull_rom(p0, p1, p2, p3, k).apply(view);
            // The pivot is eased linearly rather than splined: a Catmull-Rom
            // through Sun→Earth→Sun would overshoot past the bodies, and a
            // camera that flies *beyond* its target and comes back reads as a
            // mistake. Smootherstep still gives zero velocity at both ends.
            pivot = (
                self.from_focus.0 + (target_focus.0 - self.from_focus.0) * k,
                self.from_focus.1 + (target_focus.1 - self.from_focus.1) * k,
            );
        } else {
            // Dwell. A slow drift keeps the frame alive rather than freezing.
            let held = self.elapsed - TRANSITION_S;
            match frame {
                Some(_) => {
                    Pose { yaw: target.yaw + QSO_SWAY * (held * QSO_SWAY_RATE).sin(), ..target }
                        .apply(view);
                    pivot = target_focus;
                }
                None => {
                    // A station may travel while it is held — a dolly in, a
                    // climb — so where it pivots has to be recomputed from the
                    // pose it is actually at, not from the one it arrived at.
                    let held_pose = self.held_pose(station, b, held);
                    held_pose.apply(view);
                    pivot = self.focus_of(station, b, held_pose);
                }
            }
            // A contact is held for as long as it lasts; only the tour's own
            // stations time out.
            if frame.is_none() && held >= station.dwell_s {
                self.index = (self.index + 1) % STATIONS.len();
                if self.index == 0 {
                    // Round again: the stations that compose themselves afresh
                    // draw new numbers.
                    self.lap = self.lap.wrapping_add(1);
                }
                self.from = Pose::new(view.yaw, view.pitch, view.dist);
                // Hand the *current* pivot to the next move, so the flight
                // starts exactly where this one ended.
                self.from_focus = pivot;
                self.from_is_station = true;
                self.elapsed = 0.0;
            }
        }
        // The contact view pivots around a point above the Earth's surface, but
        // the Earth is what the user gets if they take the controls back.
        view.focus = match frame {
            Some(_) => Focus::Earth.to_id(),
            None => station.focus.to_id(),
        };
        self.last_pivot = pivot;
        pivot
    }

    /// Resume at whichever station is closest to the current view, so
    /// re-enabling AUTO does not fling the camera across the system.
    pub fn resume_near(&mut self, view: &crate::view::Solar3dView, b: &Bodies) {
        self.index = self.nearest_station(view, b);
        self.started = false;
    }

    /// The station the camera would have the shortest flight to from here.
    fn nearest_station(&self, view: &crate::view::Solar3dView, b: &Bodies) -> usize {
        let here = Pose::new(view.yaw, view.pitch, view.dist);
        let mut best = (f32::MAX, 0usize);
        for (i, s) in STATIONS.iter().enumerate() {
            let p = self.pose_of(s, b);
            let cost = short_angle(here.yaw, p.yaw).abs()
                + (p.pitch - here.pitch).abs()
                + (p.ln_dist - here.ln_dist).abs();
            if cost < best.0 {
                best = (cost, i);
            }
        }
        best.1
    }

    fn station_at(&self, i: isize) -> &'static Station {
        let n = STATIONS.len() as isize;
        &STATIONS[(i.rem_euclid(n)) as usize]
    }

    /// The pose a station is flown to: how it composes at the instant it is
    /// reached, which for a moving shot is the start of the move.
    fn pose_of(&self, s: &Station, b: &Bodies) -> Pose {
        self.end_pose(s, b, s.radii, s.pitch, s.jitter, 0)
    }

    /// The pose it composes to by the end of its dwell — the same one again for
    /// a station that holds still.
    fn dolly_of(&self, s: &Station, b: &Bodies) -> Pose {
        let (radii, pitch) = (s.radii_end.unwrap_or(s.radii), s.pitch_end.unwrap_or(s.pitch));
        self.end_pose(s, b, radii, pitch, Jitter { dist: 0.0, ..s.jitter }, 2)
    }

    /// One end of a station's framing. `salt` separates the two ends' draws
    /// from the jitter, so a dolly starts high and wide *and* finishes
    /// somewhere slightly different, rather than sliding the same move about.
    fn end_pose(
        &self,
        s: &Station,
        b: &Bodies,
        radii: f32,
        pitch: f32,
        j: Jitter,
        salt: u32,
    ) -> Pose {
        let (bearing, elev) = self.bearing_of(s.relative_to, b);
        let (_, radius) = b.focus(s.focus);
        let dist = (radius * radii * (1.0 + j.dist * lap_noise(self.lap, salt)))
            .clamp(dist_floor(s, radius), MAX_DIST);
        // A station's pitch is the latitude the camera flies **over**, which is
        // the pose's own pitch only for a centred shot. Dropping the pivot by a
        // body radius lifts the eye off the pose direction by exactly the angle
        // the axis grazes the sphere at, and a shot aimed at somebody's grid
        // square has to be aimed in the frame that grid square is in.
        let tilt = if s.drop > 0.0 { (radius * s.drop / dist).atan() } else { 0.0 };
        Pose::new(
            // One draw for the azimuth, shared by both ends: a descent that
            // swung round the planet on the way down would read as an orbit,
            // not as an approach.
            bearing + s.yaw_offset + j.yaw * lap_noise(self.lap, 9),
            (elev + pitch + j.pitch * lap_noise(self.lap, salt + 1) - tilt)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT),
            dist,
        )
    }

    /// The pose a station holds `held` seconds into its dwell: the dolly, and
    /// the drift on top of it.
    fn held_pose(&self, s: &Station, b: &Bodies, held: f32) -> Pose {
        let from = self.pose_of(s, b);
        let mut p = from;
        if s.radii_end.is_some() || s.pitch_end.is_some() {
            let to = self.dolly_of(s, b);
            // Eased at both ends, like a transition: the shot leaves the pose it
            // flew in at with no kick, and is at rest again when the dwell times
            // out and the next flight takes over.
            let k = smootherstep(held / s.dwell_s.max(1e-3));
            p = Pose {
                yaw: from.yaw + short_angle(from.yaw, to.yaw) * k,
                pitch: from.pitch + (to.pitch - from.pitch) * k,
                ln_dist: from.ln_dist + (to.ln_dist - from.ln_dist) * k,
            };
        }
        Pose { yaw: p.yaw + s.drift * held, ..p }
    }

    /// Where the camera pivots for a station at `p`, and the radius its
    /// distance clamp is measured in.
    fn focus_of(&self, s: &Station, b: &Bodies, p: Pose) -> (V3, f32) {
        let (pos, radius) = b.focus(s.focus);
        if s.drop <= 0.0 {
            return (pos, radius);
        }
        // Pushing the pivot along the screen's up by one body radius puts the
        // view axis exactly tangent to the sphere — the horizon lands on the
        // middle of the frame at *any* distance, which is what makes this a
        // framing rather than a magic number per station.
        //
        // The clamp radius has to be relaxed to let the shot stay this low, and
        // by more than the bare minimum: it is interpolated linearly across a
        // transition while the distance is interpolated in log space, and the
        // log curve runs below the straight line between the same two ends.
        let dist = p.ln_dist.exp();
        (pos + screen_up(orbit_dir(p.yaw, p.pitch)) * (radius * s.drop), radius.min(dist / 2.0))
    }

    /// The live direction a station's framing hangs off: a bearing in the
    /// ecliptic plane, and an elevation above it.
    ///
    /// Only the QTH contributes an elevation. Everything else is a body seen
    /// from another body, and those lie in that plane to within the Moon's 5°
    /// of orbital inclination — which the lunar station is composed *with*,
    /// having been framed against the flat bearing since it was written.
    fn bearing_of(&self, r: Bearing, b: &Bodies) -> (f32, f32) {
        let dir = match r {
            Bearing::SunFromEarth => V3::ZERO - b.earth,
            Bearing::EarthFromSun => b.earth,
            Bearing::MoonFromEarth => b.moon - b.earth,
            Bearing::HomeFromEarth => match self.home {
                Some((lat, lon)) => {
                    let d = b.surface_dir(lat, lon);
                    return (yaw_of(d), d.z.clamp(-1.0, 1.0).asin());
                }
                // No grid entered: a low pass over nowhere in particular, which
                // is still a better shot than no shot.
                None => return (0.0, 0.0),
            },
        };
        (yaw_of(dir), 0.0)
    }
}

/// How close a station may bring the camera to its focus body, in gigametres.
///
/// Normally the camera's own limit. A station with a `drop` has the body off
/// the view axis, so what must stay clear of the surface is the eye's true
/// distance — `hypot(dist, drop · r)` — and demanding 1.6 radii along the axis
/// as well would put every orbital shot straight back out into space.
fn dist_floor(s: &Station, radius: f32) -> f32 {
    if s.drop <= 0.0 {
        return dist_range(radius).0;
    }
    radius * (ORBIT_FLOOR * ORBIT_FLOOR - s.drop * s.drop).max(0.04).sqrt()
}

/// Which way is up on screen for a camera whose eye lies along `dir` from what
/// it is looking at: the world up with the view axis taken out of it, which is
/// what [`M4::look_at`] resolves the fixed ecliptic up-hint to.
fn screen_up(dir: V3) -> V3 {
    let z = v3(0.0, 0.0, 1.0);
    let u = z - dir * dir.dot(z);
    if u.len() > 1e-4 { u.normalize() } else { any_perp(dir) }
}

/// Deterministic per-lap noise in −1..1.
///
/// Not a clock and not a generator: the spline asks a station for the same pose
/// several frames running and has to be told the same thing each time, and the
/// tour is stepped by tests that have to be reproducible. The lap counter is
/// the only thing that moves.
fn lap_noise(lap: u32, salt: u32) -> f32 {
    let mut h = lap.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_add(1).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    h = h.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 16;
    h as f32 / (u32::MAX as f32 * 0.5) - 1.0
}

/// Re-express `p`'s yaw as the branch nearest `near`.
///
/// The spline is evaluated on raw numbers, so every control point has to be on
/// one continuous branch first — otherwise a pair straddling ±π sends the
/// camera the long way round, or worse, spinning.
fn unwrap_to(near: Pose, p: Pose) -> Pose {
    Pose { yaw: near.yaw + short_angle(near.yaw, p.yaw), ..p }
}

/// Bearing of a direction in the ecliptic plane.
fn yaw_of(v: V3) -> f32 {
    v.y.atan2(v.x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solar3d::scene;
    use crate::view::Solar3dView;

    /// The operator's QTH for every test that needs one: JN88, mid-latitude
    /// north, well off both the ecliptic equator and the prime meridian, so a
    /// station aimed at it is aimed somewhere specific.
    const VIENNA: (f64, f64) = (48.2, 16.4);

    /// One whole lap of the tour: every dwell, every transition, and margin.
    fn loop_s() -> f32 {
        STATIONS.iter().map(|s| s.dwell_s + TRANSITION_S).sum::<f32>() + 6.0
    }

    /// The camera, its bodies, and the point it is pivoting around.
    fn cam_at(dist: f32, yaw: f32, pitch: f32) -> (Camera, Bodies, V3) {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.dist = dist;
        st.view.yaw = yaw;
        st.view.pitch = pitch;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let focus = b.focus(st.focus()).0;
        (Camera::from_view(&st, &b, [1600.0, 900.0]), b, focus)
    }

    #[test]
    fn the_eye_sits_at_the_requested_distance_from_the_focus() {
        let (c, _, focus) = cam_at(300.0, 0.6, 0.55);
        assert!(((c.eye - focus).len() - 300.0).abs() < 1e-2);
        // Positive pitch looks down from above the ecliptic.
        assert!(c.eye.z > 0.0);
    }

    #[test]
    fn distance_is_clamped_outside_the_focused_body() {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.focus = crate::solar3d::state::Focus::Earth.to_id();
        st.view.dist = 1e-9;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let c = Camera::from_view(&st, &b, [800.0, 600.0]);
        let focus = b.focus(st.focus()).0;
        assert!((c.eye - focus).len() > b.earth_r, "camera ended up inside the Earth");
    }

    #[test]
    fn the_focus_projects_to_the_centre_of_the_screen() {
        let (c, _, focus) = cam_at(300.0, 1.1, -0.3);
        let m = &c.view_proj;
        let p = focus;
        let mut o = [0.0f32; 4];
        for (r, out) in o.iter_mut().enumerate() {
            *out = m.cols[0][r] * p.x + m.cols[1][r] * p.y + m.cols[2][r] * p.z + m.cols[3][r];
        }
        assert!(o[3] > 0.0, "focus behind the camera");
        assert!((o[0] / o[3]).abs() < 1e-4 && (o[1] / o[3]).abs() < 1e-4, "focus at {o:?}");
    }

    #[test]
    fn apparent_size_falls_off_with_distance() {
        let (near, b, _) = cam_at(2.0, 0.0, 0.0);
        let (far, _, _) = cam_at(20.0, 0.0, 0.0);
        let a = near.pixels_for(V3::ZERO, b.sun_r);
        let z = far.pixels_for(V3::ZERO, b.sun_r);
        assert!(a > z * 5.0, "{a} px at 2 Gm vs {z} px at 20 Gm");
    }

    // ── The AUTO tour ───────────────────────────────────────────────────────

    fn pose(yaw: f32, pitch: f32, ln_dist: f32) -> Pose {
        Pose { yaw, pitch, ln_dist }
    }

    #[test]
    fn catmull_rom_passes_through_its_middle_control_points() {
        let (p0, p1) = (pose(0.0, 0.0, 0.0), pose(1.0, 2.0, 3.0));
        let (p2, p3) = (pose(4.0, 1.0, 5.0), pose(9.0, -1.0, 2.0));
        let a = catmull_rom(p0, p1, p2, p3, 0.0);
        let b = catmull_rom(p0, p1, p2, p3, 1.0);
        for (got, want) in [(a, p1), (b, p2)] {
            assert!((got.yaw - want.yaw).abs() < 1e-5, "{got:?} vs {want:?}");
            assert!((got.pitch - want.pitch).abs() < 1e-5);
            assert!((got.ln_dist - want.ln_dist).abs() < 1e-5);
        }
    }

    /// The spline must actually curve: a straight lerp between the same two
    /// points would sit exactly on the chord, and this asserts it does not.
    #[test]
    fn catmull_rom_bends_towards_its_neighbours() {
        let (p0, p1) = (pose(0.0, 0.0, 0.0), pose(1.0, 0.0, 0.0));
        let (p2, p3) = (pose(2.0, 0.0, 0.0), pose(3.0, 4.0, 0.0));
        let mid = catmull_rom(p0, p1, p2, p3, 0.5);
        let chord_pitch = 0.0; // p1.pitch and p2.pitch are both zero
        assert!(
            (mid.pitch - chord_pitch).abs() > 0.05,
            "midpoint pitch {} sits on the chord — this is a lerp, not a spline",
            mid.pitch
        );
    }

    fn run_tour(seconds: f32, dt: f32) -> (Vec<Pose>, Vec<&'static str>) {
        let (poses, names, _) = run_tour_eyes(seconds, dt);
        (poses, names)
    }

    /// Run the tour, recording the pose, the station sequence, and — crucially —
    /// the resulting **eye position**, which is what the viewer actually sees
    /// move.
    fn run_tour_eyes(seconds: f32, dt: f32) -> (Vec<Pose>, Vec<&'static str>, Vec<V3>) {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let (mut poses, mut names, mut eyes) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..(seconds / dt) as usize {
            let pivot = tour.step(&mut st.view, &b, dt, None, None, Some(VIENNA));
            st.focus_override = Some(pivot);
            poses.push(Pose::new(st.view.yaw, st.view.pitch, st.view.dist));
            eyes.push(Camera::from_view(&st, &b, [1600.0, 900.0]).eye);
            let n = tour.station().name;
            if names.last() != Some(&n) {
                names.push(n);
            }
        }
        (poses, names, eyes)
    }

    /// The whole point of the spline: the camera must never jump. Sampled at
    /// 60 fps across a full loop, every per-frame step in yaw, pitch and log
    /// distance has to stay small.
    #[test]
    fn the_tour_path_is_continuous() {
        let dt = 1.0 / 60.0;
        let (poses, _) = run_tour(loop_s(), dt);
        assert!(poses.len() > 8000);
        let mut worst = (0.0f32, 0usize);
        for (i, w) in poses.windows(2).enumerate() {
            let dyaw = short_angle(w[0].yaw, w[1].yaw).abs();
            let dpitch = (w[1].pitch - w[0].pitch).abs();
            let ddist = (w[1].ln_dist - w[0].ln_dist).abs();
            let step = dyaw.max(dpitch).max(ddist);
            if step > worst.0 {
                worst = (step, i);
            }
        }
        // A frame of the fastest move covers a few degrees at most; anything
        // approaching a radian is a visible snap.
        assert!(
            worst.0 < 0.12,
            "frame {} jumped by {} (rad or ln-units) — the path is not smooth",
            worst.1,
            worst.0
        );
    }

    /// ...and it must be smooth in acceleration too, or the moves read as
    /// starting and stopping abruptly even though the positions are continuous.
    #[test]
    fn the_tour_has_no_velocity_discontinuities() {
        let dt = 1.0 / 60.0;
        let (poses, _) = run_tour(loop_s(), dt);
        let vel: Vec<f32> = poses.windows(2).map(|w| short_angle(w[0].yaw, w[1].yaw)).collect();
        let worst = vel.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(worst < 0.02, "yaw acceleration spike of {worst} rad/frame²");
    }

    /// The test that matters: the **eye** must not jump.
    ///
    /// Yaw, pitch and distance being continuous is not sufficient. The camera
    /// pivots around a body, and stations focus different bodies — so switching
    /// focus at the start of a transition teleports the eye by up to 1 AU while
    /// every interpolated value stays perfectly smooth. Measuring the pose alone
    /// misses that completely.
    #[test]
    fn the_eye_never_jumps_between_stations() {
        let dt = 1.0 / 60.0;
        let (_, _, eyes) = run_tour_eyes(loop_s(), dt);
        assert!(eyes.len() > 9000);

        // The criterion is the *smoothness* of the step profile, not its
        // magnitude: the tour deliberately crosses 200 Gm in 3.2 s, so a 7 Gm
        // frame is normal mid-flight. A teleport is instead one enormous step
        // between two tiny ones, so it shows up as the step size changing by
        // about its own size from one frame to the next.
        let steps: Vec<f32> = eyes.windows(2).map(|w| (w[1] - w[0]).len()).collect();
        let peak = steps.iter().copied().fold(0.0f32, f32::max);
        let (jerk, at) = steps
            .windows(2)
            .enumerate()
            .map(|(i, w)| ((w[1] - w[0]).abs(), i))
            .fold((0.0f32, 0usize), |acc, x| if x.0 > acc.0 { x } else { acc });
        // A smooth ease measures about 0.025; an instant focus switch is ~1.0.
        assert!(
            jerk < peak * 0.15,
            "eye step changed by {jerk} Gm at frame {at} against a {peak} Gm peak \
             ({:.2} of it) — that is a jump, not a flight",
            jerk / peak
        );

        // And nothing pathological: no NaNs, nothing flung outside the system.
        for e in &eyes {
            assert!(e.x.is_finite() && e.y.is_finite() && e.z.is_finite());
            assert!(e.len() < 3000.0, "eye at {} Gm from the Sun", e.len());
        }
    }

    /// A focus change on its own — the exact case the pivot interpolation
    /// exists for. Stepping from a Sun-focused station to an Earth-focused one
    /// must move the pivot gradually, not snap it 1 AU.
    #[test]
    fn the_pivot_eases_between_bodies() {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();

        // Settle at station 0 (Sun), then advance to station 1 (Earth).
        let mut pivot = tour.step(&mut st.view, &b, 1.0 / 60.0, None, None, Some(VIENNA));
        while tour.index == 0 {
            pivot = tour.step(&mut st.view, &b, 0.05, None, None, Some(VIENNA));
        }
        assert_eq!(tour.station().focus, Focus::Earth);
        // The first frame of the new move must still be at the Sun, not at the
        // Earth: that is the jump this guards against.
        assert!(pivot.0.len() < 1.0, "pivot already {} Gm out", pivot.0.len());

        let mut prev = pivot.0;
        let mut worst = 0.0f32;
        for _ in 0..(TRANSITION_S / 0.02) as usize + 4 {
            let p = tour.step(&mut st.view, &b, 0.02, None, None, Some(VIENNA)).0;
            worst = worst.max((p - prev).len());
            prev = p;
        }
        // Arrived at the Earth...
        assert!(
            (prev - b.earth).len() < 0.01,
            "pivot ended {} Gm off the Earth",
            (prev - b.earth).len()
        );
        // ...having covered 1 AU in steps no larger than a smooth ease implies.
        assert!(worst < 3.0, "pivot moved {worst} Gm in a single 20 ms step");
    }

    #[test]
    fn the_tour_visits_every_station_and_loops() {
        let (_, names) = run_tour(loop_s(), 1.0 / 30.0);
        let mut seen: Vec<&str> = Vec::new();
        for n in &names {
            if !seen.contains(n) {
                seen.push(n);
            }
        }
        assert_eq!(seen.len(), STATIONS.len(), "only visited {seen:?}");
        // Order is the table's order, and it wraps.
        assert_eq!(names[0], STATIONS[0].name);
        assert_eq!(names[1], STATIONS[1].name);
    }

    #[test]
    fn the_tour_stays_within_the_camera_limits() {
        let (poses, _) = run_tour(loop_s(), 1.0 / 30.0);
        for p in &poses {
            assert!(p.pitch.abs() <= PITCH_LIMIT + 1e-4, "pitch {} out of range", p.pitch);
            let d = p.ln_dist.exp();
            assert!(d > 0.0 && d <= MAX_DIST + 1.0, "distance {d} out of range");
            assert!(p.yaw.is_finite() && p.pitch.is_finite() && p.ln_dist.is_finite());
        }
    }

    /// Re-enabling AUTO after the user has moved the camera must pick the
    /// nearest station, not restart the loop from the beginning.
    #[test]
    fn resuming_picks_up_at_the_nearest_station() {
        let mut st = SolarUi::new(Solar3dView::default());
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();

        // Park the camera at station 4's pose, then resume.
        let target = tour.pose_of(&STATIONS[4], &b);
        st.view.yaw = target.yaw;
        st.view.pitch = target.pitch;
        st.view.dist = target.ln_dist.exp();
        tour.index = 0;
        tour.request_resume();
        tour.step(&mut st.view, &b, 1.0 / 60.0, None, None, Some(VIENNA));
        assert_eq!(tour.index, 4, "resumed at {} instead", tour.station().name);
    }

    /// A stalled frame must not teleport the camera: the step is clamped, so a
    /// one-second hitch advances the move by at most a quarter second.
    #[test]
    fn a_long_frame_does_not_teleport_the_camera() {
        let mut st = SolarUi::new(Solar3dView::default());
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        tour.step(&mut st.view, &b, 1.0 / 60.0, None, None, Some(VIENNA));
        let before = Pose::new(st.view.yaw, st.view.pitch, st.view.dist);
        tour.step(&mut st.view, &b, 5.0, None, None, Some(VIENNA));
        let after = Pose::new(st.view.yaw, st.view.pitch, st.view.dist);
        assert!(tour.elapsed <= 0.3, "elapsed jumped to {}", tour.elapsed);
        assert!(
            short_angle(before.yaw, after.yaw).abs() < 0.5,
            "camera jumped {} rad on a stalled frame",
            short_angle(before.yaw, after.yaw).abs()
        );
    }

    #[test]
    fn unwrapping_keeps_the_spline_on_one_branch() {
        let near = pose(3.0, 0.0, 0.0);
        // 3.0 and −3.0 are 0.28 rad apart the short way, 6 rad apart the long way.
        let far = unwrap_to(near, pose(-3.0, 0.0, 0.0));
        assert!((far.yaw - near.yaw).abs() < 0.4, "unwrapped to {}", far.yaw);
        assert!(far.yaw > std::f32::consts::PI, "took the long way: {}", far.yaw);
    }

    // ── The earth-side stations ─────────────────────────────────────────────

    /// Fly the tour, from cold, until it is `into` seconds into the named
    /// station's dwell — which is what a viewer who leaves AUTO running sees,
    /// as opposed to the pose the table asks for.
    fn settled_at(name: &str, into: f32, home: Option<(f64, f64)>) -> (SolarUi, Bodies, Camera) {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let dt = 1.0 / 60.0;
        let want = STATIONS.iter().position(|s| s.name == name).expect("no such station");
        for _ in 0..(loop_s() * 2.0 / dt) as usize {
            let pivot = tour.step(&mut st.view, &b, dt, None, None, home);
            st.focus_override = Some(pivot);
            if tour.index == want && tour.elapsed >= TRANSITION_S + into {
                let cam = Camera::from_view(&st, &b, [1600.0, 900.0]);
                return (st, b, cam);
            }
        }
        panic!("the tour never reached {name}");
    }

    /// Directions of the globe's visible cap: everything the camera can see of
    /// it, sampled evenly enough to find its silhouette.
    fn visible_cap(cam: &Camera, b: &Bodies) -> Vec<V3> {
        let d = (cam.eye - b.earth).len();
        let u = (cam.eye - b.earth).normalize();
        let (e1, e2) = (any_perp(u), u.cross(any_perp(u)));
        let alpha = (b.earth_r / d).clamp(-1.0, 1.0).acos();
        let mut out = Vec::new();
        for i in 0..=40 {
            let theta = alpha * i as f32 / 40.0;
            for k in 0..120 {
                let phi = k as f32 / 120.0 * std::f32::consts::TAU;
                out.push(u * theta.cos() + (e1 * phi.cos() + e2 * phi.sin()) * theta.sin());
            }
        }
        out
    }

    /// The low pass has to look like one: the planet across the bottom of the
    /// frame, its horizon on the middle of it, and space above.
    #[test]
    fn the_home_orbit_lays_the_earth_across_the_lower_half() {
        for into in [0.0, 9.0, 19.4] {
            let (_, b, cam) = settled_at("HOME ORBIT", into, Some(VIENNA));
            let mut top = -2.0f32;
            let mut reaches_bottom = false;
            for dir in visible_cap(&cam, &b) {
                let Some((x, y)) = ndc(&cam, b.earth + dir * b.earth_r) else { continue };
                if x.abs() < 0.2 {
                    top = top.max(y);
                    reaches_bottom |= y < -0.95;
                }
            }
            // The horizon crosses the middle of the frame...
            assert!(
                top.abs() < 0.12,
                "{into} s in: the globe's silhouette tops out at {top:.2}, not on the middle",
            );
            // ...and the globe runs off the bottom of it rather than floating.
            assert!(
                reaches_bottom,
                "{into} s in: the globe does not reach the bottom of the frame"
            );
            // Low, and still in space.
            let h = (cam.eye - b.earth).len();
            assert!(
                (b.earth_r * 1.05..b.earth_r * 1.45).contains(&h),
                "{into} s in: flying at {:.2} radii — that is not a low orbit",
                h / b.earth_r,
            );
        }
    }

    /// ...and it has to be going somewhere: the operator's own grid square,
    /// which it closes on across the whole dwell and never overshoots.
    #[test]
    fn the_home_orbit_closes_on_the_operators_grid() {
        let qth = |b: &Bodies| b.surface_dir(VIENNA.0, VIENNA.1);
        let separation = |cam: &Camera, b: &Bodies| {
            let sub = (cam.eye - b.earth).normalize();
            sub.dot(qth(b)).clamp(-1.0, 1.0).acos() / DEG
        };

        let mut prev = f32::MAX;
        for into in [0.0, 5.0, 10.0, 15.0, 19.4] {
            let (_, b, cam) = settled_at("HOME ORBIT", into, Some(VIENNA));
            let sep = separation(&cam, &b);
            assert!(sep < prev - 0.2, "{into} s in: {sep:.0}° from the QTH, was {prev:.0}°");
            prev = sep;
        }
        // It closes most of the way and then stops, leaving the grid square in
        // the lower half of the frame — under the horizon the drop laid across
        // the middle of it, which is where the ground is.
        let (_, b, cam) = settled_at("HOME ORBIT", 19.4, Some(VIENNA));
        assert!((10.0..24.0).contains(&prev), "the pass finished {prev:.0}° from the QTH");
        let (x, y) =
            ndc(&cam, b.earth + qth(&b) * b.earth_r).expect("the QTH is behind the camera");
        assert!(x.abs() < 0.5 && (-1.0..0.0).contains(&y), "the QTH sits at ({x:.2}, {y:.2})");
    }

    /// No grid entered is not a reason to break the loop: the pass still flies,
    /// it is just aimed at nobody.
    #[test]
    fn the_home_orbit_flies_without_a_grid() {
        let (st, b, cam) = settled_at("HOME ORBIT", 10.0, None);
        let h = (cam.eye - b.earth).len();
        assert!(h > b.earth_r * 1.05, "eye {h} inside a {} globe", b.earth_r);
        assert!(st.view.yaw.is_finite() && st.view.pitch.is_finite() && st.view.dist.is_finite());
        assert!(cam.eye.x.is_finite() && cam.eye.y.is_finite() && cam.eye.z.is_finite());
    }

    /// How many of the visible surface samples on screen are in daylight, and
    /// how many are in the dark.
    fn lit_and_dark_on_screen(cam: &Camera, b: &Bodies) -> (usize, usize) {
        let sun = (V3::ZERO - b.earth).normalize();
        let (mut lit, mut dark) = (0, 0);
        for dir in visible_cap(cam, b) {
            let on = ndc(cam, b.earth + dir * b.earth_r)
                .is_some_and(|(x, y)| x.abs() <= 1.0 && y.abs() <= 1.0);
            if !on {
                continue;
            }
            if dir.dot(sun) > 0.0 { lit += 1 } else { dark += 1 }
        }
        (lit, dark)
    }

    /// The two greyline stations must be on *opposite* sides of the line, and
    /// each on the side its name claims.
    ///
    /// Which limb is which follows from the way the Earth turns: a place 90°
    /// round from the subsolar point in the direction of rotation is losing its
    /// daylight, and the one 90° back is about to get it. The test measures
    /// exactly that — the rate at which the ground in the middle of the frame is
    /// gaining or losing sun, `d(r·ŝ)/dt` with `r` turning about the pole.
    #[test]
    fn the_dusk_and_dawn_stations_take_the_right_sides_of_the_line() {
        for (name, want_darkening) in [("DUSK LINE", true), ("DAWN LINE", false)] {
            let (_, b, cam) = settled_at(name, 5.0, Some(VIENNA));
            let sun = (V3::ZERO - b.earth).normalize();
            let middle = (cam.eye - b.earth).normalize();
            let rate = b.earth_basis.2.cross(middle).dot(sun);
            assert!(
                (rate < 0.0) == want_darkening,
                "{name}: the ground in frame is {} — wrong side of the terminator",
                if rate < 0.0 { "going dark" } else { "coming into the light" },
            );
            // Both sides of the line have to be in the frame, or there is no
            // border in the shot at all.
            let (lit, dark) = lit_and_dark_on_screen(&cam, &b);
            assert!(lit > 200 && dark > 200, "{name}: {lit} lit and {dark} dark samples on screen");
            // Angled, not the flat side-on view the greyline station already is.
            let (st, _, _) = settled_at(name, 5.0, Some(VIENNA));
            assert!(
                st.view.pitch.abs() > 25.0 * DEG,
                "{name}: {}° off the ecliptic",
                st.view.pitch
            );
        }
    }

    /// The descent is a descent: a long way out at the top of the dwell, in
    /// orbit by the end of it, and never through the surface on the way.
    #[test]
    fn the_orbit_descent_falls_from_far_out_into_orbit() {
        let mut last = f32::MAX;
        for into in [0.0, 7.0, 14.0, 20.6] {
            let (_, b, cam) = settled_at("ORBIT DESCENT", into, Some(VIENNA));
            let h = (cam.eye - b.earth).len() / b.earth_r;
            assert!(h < last, "{into} s in: {h:.1} radii out, was {last:.1} — not descending");
            assert!(h > 1.05, "{into} s in: {h:.1} radii — inside the globe");
            last = h;
        }
        assert!(last < 1.5, "the descent finished {last:.1} radii out, nowhere near orbit");

        let (_, b, cam) = settled_at("ORBIT DESCENT", 0.0, Some(VIENNA));
        let start = (cam.eye - b.earth).len() / b.earth_r;
        assert!(start > 8.0, "the descent starts only {start:.1} radii out");
    }

    /// ...and it is not the *same* descent twice: the bearing, the height it
    /// starts from and the angle it comes in at are all drawn afresh each lap.
    #[test]
    fn the_orbit_descent_is_composed_afresh_each_lap() {
        let st = SolarUi::new(Solar3dView::default());
        let b = scene::bodies(&st, 1_784_937_600.0);
        let s = STATIONS.iter().find(|s| s.name == "ORBIT DESCENT").expect("the descent");
        let mut tour = Tour::default();

        let mut seen: Vec<Pose> = Vec::new();
        for lap in 0..8u32 {
            tour.lap = lap;
            let p = tour.pose_of(s, &b);
            assert!(p.yaw.is_finite() && p.pitch.is_finite() && p.ln_dist.is_finite());
            // Within the spread it was given, not wandering off it.
            let (bearing, _) = tour.bearing_of(s.relative_to, &b);
            let radius = b.focus(s.focus).1;
            assert!(short_angle(bearing + s.yaw_offset, p.yaw).abs() <= s.jitter.yaw + 1e-3);
            assert!(
                (p.ln_dist - (radius * s.radii).ln()).abs() <= (1.0 + s.jitter.dist).ln() + 1e-3,
                "lap {lap} starts {:.1} radii out",
                p.ln_dist.exp() / radius,
            );
            // The two ends of one lap's move share a bearing: it is an approach,
            // not an orbit.
            assert!(short_angle(p.yaw, tour.dolly_of(s, &b).yaw).abs() < 1e-4);
            seen.push(p);
        }
        for (i, a) in seen.iter().enumerate() {
            for (j, c) in seen.iter().enumerate().skip(i + 1) {
                let d = short_angle(a.yaw, c.yaw).abs() + (a.ln_dist - c.ln_dist).abs();
                assert!(d > 0.02, "laps {i} and {j} fly the same descent");
            }
        }
    }

    /// One shot of the Sun's own disk, and only one — the rest of the loop is
    /// the planet the operator is working from.
    #[test]
    fn only_one_station_frames_the_solar_disk() {
        let close: Vec<&str> = STATIONS
            .iter()
            .filter(|s| s.focus == Focus::Sun && s.radii < 100.0)
            .map(|s| s.name)
            .collect();
        assert_eq!(close, ["SUNSIDE"], "solar stations in the loop: {close:?}");
        // And the wide one is a system view, far enough out for the planets'
        // orbits rather than the Sun's surface.
        let system = STATIONS.iter().find(|s| s.name == "SYSTEM ANGLED").expect("the system view");
        assert!(system.radii > 1000.0 && system.pitch > 20.0 * DEG);
    }

    // ── The contact view ────────────────────────────────────────────────────

    /// A path of `sep` degrees, from a mid-latitude QTH along a meridian.
    fn path_of(sep: f64) -> QsoPath {
        QsoPath { home: VIENNA, dx: (VIENNA.0 - sep, VIENNA.1) }
    }

    /// Fly the tour with a contact in progress until it has settled on it.
    fn settled_on(path: QsoPath) -> (SolarUi, Bodies, Camera) {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let dt = 1.0 / 60.0;
        // Long enough for the flight in (3.2 s) and a good part of the sway.
        for _ in 0..(12.0 / dt) as usize {
            let pivot = tour.step(&mut st.view, &b, dt, Some(path), None, Some(VIENNA));
            st.focus_override = Some(pivot);
        }
        assert_eq!(tour.leg_name(), "QSO PATH");
        let cam = Camera::from_view(&st, &b, [1600.0, 900.0]);
        (st, b, cam)
    }

    /// Normalised device coordinates, or `None` when the point is behind the
    /// camera. The viewport is everything within ±1 on both axes.
    fn ndc(cam: &Camera, p: V3) -> Option<(f32, f32)> {
        let m = &cam.view_proj;
        let mut o = [0.0f32; 4];
        for (r, out) in o.iter_mut().enumerate() {
            *out = m.cols[0][r] * p.x + m.cols[1][r] * p.y + m.cols[2][r] * p.z + m.cols[3][r];
        }
        (o[3] > 0.0).then(|| (o[0] / o[3], o[1] / o[3]))
    }

    /// A point on (or above) the globe, the way the arc is drawn.
    fn on_globe(b: &Bodies, lat: f64, lon: f64, lift: f32) -> V3 {
        b.earth + b.surface_dir(lat, lon) * (b.earth_r * lift)
    }

    // ── The satellite view ──────────────────────────────────────────────────

    /// Fly the tour with a satellite lock in progress until it has settled.
    fn settled_on_sat(path: SatPath) -> (SolarUi, Bodies, Camera) {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let dt = 1.0 / 60.0;
        for _ in 0..(12.0 / dt) as usize {
            let pivot = tour.step(&mut st.view, &b, dt, None, Some(path), Some(VIENNA));
            st.focus_override = Some(pivot);
        }
        let cam = Camera::from_view(&st, &b, [1600.0, 900.0]);
        (st, b, cam)
    }

    /// A satellite `lift` Earth radii from the centre, `sep` degrees of ground
    /// track away from the Vienna QTH along its meridian.
    fn sat_at(b: &Bodies, sep: f64, lift: f32) -> SatPath {
        SatPath { home: (48.2, 16.4), sat_world: on_globe(b, 48.2 - sep, 16.4, lift) }
    }

    /// Both ends of the lock — the QTH and the satellite — have to be on
    /// screen, from an overhead LEO pass through a horizon-grazing one to a
    /// geostationary bird a fifth of the way to the Moon.
    #[test]
    fn the_satellite_view_frames_the_qth_and_the_bird() {
        let probe = SolarUi::new(Solar3dView::default());
        let bp = scene::bodies(&probe, 1_784_937_600.0);
        for (what, sep, lift) in [
            ("ISS overhead", 0.3, 1.066),
            ("ISS mid-pass", 8.0, 1.066),
            ("LEO near the horizon", 18.0, 1.13),
            ("QO-100", 42.0, 6.61),
        ] {
            let path = sat_at(&bp, sep, lift);
            let (_, b, cam) = settled_on_sat(path);
            let home = on_globe(&b, path.home.0, path.home.1, 1.0);
            for (name, p) in [("QTH", home), ("satellite", path.sat_world)] {
                let (x, y) = ndc(&cam, p)
                    .unwrap_or_else(|| panic!("{what}: the {name} is behind the camera"));
                assert!(
                    x.abs() < 0.98 && y.abs() < 0.98,
                    "{what}: the {name} is off screen at ({x:.2}, {y:.2})"
                );
            }
            assert!((cam.eye - b.earth).len() > b.earth_r * 1.05, "{what}: eye inside the globe");
        }
    }

    /// A locked satellite outranks a contact in progress: the operator is
    /// working *through* the bird, so the bird is the show.
    #[test]
    fn the_satellite_lock_preempts_the_contact() {
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let qso = Some(path_of(60.0));
        let sat = Some(sat_at(&b, 8.0, 1.066));
        let dt = 1.0 / 60.0;
        for _ in 0..(8.0 / dt) as usize {
            let pivot = tour.step(&mut st.view, &b, dt, qso, sat, Some(VIENNA));
            st.focus_override = Some(pivot);
        }
        // The pivot sits on the QTH–satellite sightline, not up on the
        // contact arc's midpoint 30° south.
        let want = (on_globe(&b, 48.2, 16.4, 1.0) + sat.unwrap().sat_world) * 0.5;
        let got = st.focus_override.unwrap().0;
        assert!(
            (got - want).len() < b.earth_r * 0.1,
            "pivot is {} Gm from the sightline midpoint — the contact won",
            (got - want).len()
        );
    }

    /// Whether any part of the Earth's horizon is in the frame — the test of
    /// "the curvature is visible" that survives the camera being close enough
    /// for the globe to overflow the viewport.
    fn limb_on_screen(cam: &Camera, b: &Bodies) -> bool {
        let d = (cam.eye - b.earth).len();
        let u = (cam.eye - b.earth).normalize();
        let (e1, e2) = (any_perp(u), u.cross(any_perp(u)));
        // The horizon is the circle at this angle from the sub-camera point.
        let alpha = (b.earth_r / d).clamp(-1.0, 1.0).acos();
        (0..180).any(|k| {
            let phi = k as f32 / 180.0 * std::f32::consts::TAU;
            let dir = u * alpha.cos() + (e1 * phi.cos() + e2 * phi.sin()) * alpha.sin();
            ndc(cam, b.earth + dir * b.earth_r)
                .is_some_and(|(x, y)| x.abs() <= 1.0 && y.abs() <= 1.0)
        })
    }

    /// The point of the whole thing: both stations and the arc between them
    /// have to be on screen, at every path length from a local contact to a
    /// near-antipodal one.
    #[test]
    fn the_contact_view_frames_both_stations_and_the_arc() {
        for sep in [1.0, 12.0, 60.0, 120.0, 178.0] {
            let path = path_of(sep);
            let (_, b, cam) = settled_on(path);
            let ends = [path.home, path.dx].map(|(lat, lon)| {
                ndc(&cam, on_globe(&b, lat, lon, 1.0))
                    .unwrap_or_else(|| panic!("{sep}° path: an end is behind the camera"))
            });
            for (i, (x, y)) in ends.iter().enumerate() {
                assert!(
                    x.abs() < 0.98 && y.abs() < 0.98,
                    "{sep}° path: end {i} is off screen at ({x:.2}, {y:.2})",
                );
            }
            // ...and framed, not merely present: a shot where the two ends sit
            // on top of each other has not zoomed in on anything. Only a path
            // long enough to *have* a shape gets held to the tighter bound —
            // two stations 100 km apart are close together on any globe with a
            // horizon still in it.
            let gap = ((ends[0].0 - ends[1].0).powi(2) + (ends[0].1 - ends[1].1).powi(2)).sqrt();
            let want = if sep >= 12.0 { 0.35 } else { 0.03 };
            assert!(gap > want, "{sep}° path: the two ends are {gap:.3} apart on screen");
        }
    }

    /// Shallow enough that the globe reads as a sphere and the arc's rise is
    /// plainly visible, rather than the overhead view that flattens both.
    #[test]
    fn the_contact_view_is_a_shallow_one() {
        for sep in [12.0, 60.0, 120.0] {
            let path = path_of(sep);
            let (st, b, cam) = settled_on(path);
            let pivot = st.focus_override.expect("the tour supplies a pivot").0;

            // How far the line of sight is off the vertical where it lands.
            // Straight down is 0°, along the surface is 90°.
            let vertical = (pivot - b.earth).normalize();
            let ray = (cam.eye - pivot).normalize();
            let tilt = ray.dot(vertical).clamp(-1.0, 1.0).acos() / DEG;
            assert!(
                (40.0..80.0).contains(&tilt),
                "{sep}° path: looking down at {tilt:.0}° off the vertical",
            );
            // Shallow is only half of it: the horizon has to be in the frame,
            // or there is no curvature on screen to see.
            assert!(limb_on_screen(&cam, &b), "{sep}° path: no horizon in the frame");

            // And the rise itself: the apex of the arc must project clear of
            // the surface directly below it, which is exactly what an overhead
            // shot loses.
            let omega = (sep as f32) * DEG;
            let bulge = scene::arc_bulge(omega as f64);
            let (mlat, mlon) = (path.home.0 - sep * 0.5, path.home.1);
            let apex = ndc(&cam, on_globe(&b, mlat, mlon, 1.0 + bulge)).expect("apex on screen");
            let below = ndc(&cam, on_globe(&b, mlat, mlon, 1.0)).expect("surface on screen");
            let rise = ((apex.0 - below.0).powi(2) + (apex.1 - below.1).powi(2)).sqrt();
            assert!(rise > 0.06, "{sep}° path: the arc rises only {rise:.3} on screen");
        }
    }

    /// The same, for paths that lie the way real ones do rather than along a
    /// meridian: the framing is built on the plane of the path, so a path over
    /// the pole and one along the equator put the camera in quite different
    /// places relative to the ecliptic the camera's "up" is fixed to.
    #[test]
    fn the_contact_view_holds_up_for_real_paths() {
        let vienna = (48.2, 16.4);
        for (what, path) in [
            ("Vienna–Tokyo", QsoPath { home: vienna, dx: (35.7, 139.7) }),
            ("Vienna–Sydney", QsoPath { home: vienna, dx: (-33.9, 151.2) }),
            ("Vienna–Buenos Aires", QsoPath { home: vienna, dx: (-34.6, -58.4) }),
            // Along the equator: the plane of the path is the one furthest
            // from the ecliptic the camera keeps level.
            ("Nairobi–Singapore", QsoPath { home: (-1.3, 36.8), dx: (1.4, 103.8) }),
            // And straight over the pole.
            ("Reykjavik–Anchorage", QsoPath { home: (64.1, -21.9), dx: (61.2, -149.9) }),
        ] {
            let (st, b, cam) = settled_on(path);
            let ends = [path.home, path.dx].map(|(lat, lon)| {
                ndc(&cam, on_globe(&b, lat, lon, 1.0))
                    .unwrap_or_else(|| panic!("{what}: an end is behind the camera"))
            });
            for (i, (x, y)) in ends.iter().enumerate() {
                assert!(x.abs() < 0.98 && y.abs() < 0.98, "{what}: end {i} at ({x:.2}, {y:.2})");
            }
            let gap = ((ends[0].0 - ends[1].0).powi(2) + (ends[0].1 - ends[1].1).powi(2)).sqrt();
            assert!(gap > 0.35, "{what}: the two ends are {gap:.3} apart on screen");

            let pivot = st.focus_override.expect("the tour supplies a pivot").0;
            let tilt = (cam.eye - pivot)
                .normalize()
                .dot((pivot - b.earth).normalize())
                .clamp(-1.0, 1.0)
                .acos()
                / DEG;
            assert!((40.0..80.0).contains(&tilt), "{what}: {tilt:.0}° off the vertical");
            assert!(limb_on_screen(&cam, &b), "{what}: no horizon in the frame");
            assert!((cam.eye - b.earth).len() > b.earth_r * 1.05, "{what}: eye inside the globe");
        }
    }

    /// However the path lies, the camera must stay out in space — a pivot that
    /// sits just above the surface makes this easy to get wrong.
    #[test]
    fn the_contact_view_stays_outside_the_globe() {
        for sep in [0.5, 5.0, 45.0, 90.0, 150.0, 179.9] {
            let path = path_of(sep);
            let (_, b, cam) = settled_on(path);
            let h = (cam.eye - b.earth).len();
            assert!(h > b.earth_r * 1.05, "{sep}° path: eye {h} inside a {} globe", b.earth_r);
            assert!(cam.eye.x.is_finite() && cam.eye.y.is_finite() && cam.eye.z.is_finite());
        }
    }

    /// Two stations in the same place have no path between them and no arc is
    /// drawn for them either, so the tour must carry on rather than framing a
    /// point.
    #[test]
    fn a_contact_with_itself_is_not_framed() {
        let mut st = SolarUi::new(Solar3dView::default());
        let b = scene::bodies(&st, 1_784_937_600.0);
        assert!(qso_frame(QsoPath { home: (48.2, 16.4), dx: (48.2, 16.4) }, &b).is_none());

        let mut tour = Tour::default();
        let path = QsoPath { home: (48.2, 16.4), dx: (48.200_01, 16.400_01) };
        for _ in 0..600 {
            tour.step(&mut st.view, &b, 1.0 / 60.0, Some(path), None, Some(VIENNA));
        }
        assert_ne!(tour.leg_name(), "QSO PATH");
    }

    /// A contact takes the camera over for as long as it lasts — well past the
    /// dwell any station gets — and hands it back when it ends. Neither
    /// handover may jump the eye.
    #[test]
    fn a_contact_pre_empts_the_tour_and_hands_it_back() {
        let dt = 1.0 / 60.0;
        let mut st = SolarUi::new(Solar3dView::default());
        st.view.auto = true;
        let b = scene::bodies(&st, 1_784_937_600.0);
        let mut tour = Tour::default();
        let path = path_of(75.0);

        let mut eyes: Vec<V3> = Vec::new();
        let frame = |tour: &mut Tour, st: &mut SolarUi, qso, eyes: &mut Vec<V3>| {
            let pivot = tour.step(&mut st.view, &b, dt, qso, None, Some(VIENNA));
            st.focus_override = Some(pivot);
            eyes.push(Camera::from_view(st, &b, [1600.0, 900.0]).eye);
        };

        // Settle into the tour first, so the contact arrives mid-dwell.
        for _ in 0..(10.0 / dt) as usize {
            frame(&mut tour, &mut st, None, &mut eyes);
        }
        assert!(!tour.in_transit(), "the tour should be dwelling by now");
        let at_pickup = eyes.len();

        // The contact runs for 40 s — longer than any station's dwell.
        for _ in 0..(40.0 / dt) as usize {
            frame(&mut tour, &mut st, Some(path), &mut eyes);
        }
        assert_eq!(tour.leg_name(), "QSO PATH", "the tour walked off the contact");
        let (target, pivot) = qso_frame(path, &b).expect("a 75° path is framable");
        assert!(
            short_angle(st.view.yaw, target.yaw).abs() < QSO_SWAY * 1.1,
            "settled {} rad off the framing",
            short_angle(st.view.yaw, target.yaw).abs(),
        );
        assert!((st.view.dist - target.ln_dist.exp()).abs() < target.ln_dist.exp() * 0.02);
        assert!((st.focus_override.expect("pivot").0 - pivot.0).len() < b.earth_r * 0.01);
        let at_handback = eyes.len();

        // Contact over: the tour takes back over.
        for _ in 0..(20.0 / dt) as usize {
            frame(&mut tour, &mut st, None, &mut eyes);
        }
        assert_ne!(tour.leg_name(), "QSO PATH", "the camera never left the contact");

        // Both handovers start a flight from rest, so the frame either side of
        // one may not move further than the frame before it did.
        for (what, i) in [("pick-up", at_pickup), ("hand-back", at_handback)] {
            let before = (eyes[i - 1] - eyes[i - 2]).len();
            let after = (eyes[i] - eyes[i - 1]).len();
            assert!(
                after <= before * 2.0 + 1e-6,
                "{what}: the eye jumped {after} Gm against {before} Gm the frame before",
            );
        }
        for e in &eyes {
            assert!(e.x.is_finite() && e.y.is_finite() && e.z.is_finite());
        }
    }
}
