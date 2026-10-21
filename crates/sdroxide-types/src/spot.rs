//! Network spot domain types, shared by the native engine, the wire protocol,
//! and the UI (native + WASM). Pure data + serde — the actual feed clients
//! (DX cluster telnet, POTA/SOTA HTTP, PSK Reporter) live in the native
//! `sdroxide-net` crate. Modelled on [`crate::SkimmerSpot`] so the panadapter
//! overlay and world map reuse the same rendering path.

use serde::{Deserialize, Serialize};

use crate::Mode;

/// Where a spot came from. Drives the overlay colour and the map marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpotKind {
    /// A DX-cluster telnet node (`DX de …`).
    DxCluster,
    /// Parks On The Air activator spot.
    Pota,
    /// Summits On The Air activator spot.
    Sota,
    /// A PSK Reporter reception report (someone heard us / a monitored call).
    PskReporter,
    /// A station connected to FreeDV Reporter (<https://qso.freedv.org/>).
    /// Appended last so the postcard discriminants above never move.
    FreeDv,
    /// A longwave/shortwave broadcast station from [`crate::broadcast`].
    ///
    /// Unlike every other kind this one is synthesised in the client from a
    /// bundled table rather than reported by a feed, so it never travels over
    /// the wire — but it shares `Spot` so the overlay, the list and the map
    /// render it through one path. Appended last, as above.
    Broadcast,
    /// A PSK Reporter reception report where *this* station was the sender —
    /// one reporter that heard us, placed at the **reporter's** own grid.
    ///
    /// Deliberately its own kind rather than folded into
    /// [`SpotKind::PskReporter`], which is the stations being heard: the two
    /// are opposite ends of the same reports, and a "who heard me" dot drawn
    /// or listed as if it were a station to tune would be a different thing
    /// wearing the same face. Kept out of the SPOTS list and the propagation
    /// field (see `spot_visible` and the snapshot in `sdroxide-net`); it is a
    /// map overlay only. Appended last, as above.
    HeardMe,
    /// A Reverse Beacon Network skimmer spot: a skimmer decoded a station at
    /// a known frequency. Appended last so the postcard discriminants above
    /// never move.
    Rbn,
}

impl SpotKind {
    /// Every kind, in the order the SPOTS window's filter chips, the settings
    /// colour pickers and [`SpotKind::index`] all use. Anything indexed by kind
    /// — the filter array, [`crate::UiSettings::spot_colors`] — is this wide.
    pub const ALL: [SpotKind; 8] = [
        SpotKind::DxCluster,
        SpotKind::Pota,
        SpotKind::Sota,
        SpotKind::PskReporter,
        SpotKind::FreeDv,
        SpotKind::Broadcast,
        SpotKind::HeardMe,
        SpotKind::Rbn,
    ];

    /// How many kinds there are, i.e. the width of every per-kind array.
    pub const COUNT: usize = SpotKind::ALL.len();

    /// This kind's position in [`SpotKind::ALL`].
    pub fn index(self) -> usize {
        match self {
            SpotKind::DxCluster => 0,
            SpotKind::Pota => 1,
            SpotKind::Sota => 2,
            SpotKind::PskReporter => 3,
            SpotKind::FreeDv => 4,
            SpotKind::Broadcast => 5,
            SpotKind::HeardMe => 6,
            SpotKind::Rbn => 7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SpotKind::DxCluster => "DX",
            SpotKind::Pota => "POTA",
            SpotKind::Sota => "SOTA",
            SpotKind::PskReporter => "PSK",
            SpotKind::FreeDv => "FREEDV",
            SpotKind::Broadcast => "BC",
            SpotKind::HeardMe => "HEARD ME",
            SpotKind::Rbn => "RBN",
        }
    }

    /// The stock per-kind RGB tint for the overlay/marker (r, g, b).
    ///
    /// Where a label starts, not where it has to stay: the operator can retint
    /// any kind from the UI settings tab, and what they chose is kept in
    /// [`crate::UiSettings::spot_colors`]. Clients read that, not this.
    pub fn default_color(self) -> (u8, u8, u8) {
        match self {
            SpotKind::DxCluster => (120, 220, 255),   // cyan
            SpotKind::Pota => (120, 230, 140),        // green
            SpotKind::Sota => (255, 190, 90),         // amber
            SpotKind::PskReporter => (210, 150, 255), // violet
            SpotKind::FreeDv => (255, 130, 160),      // pink
            // Yellow-green. This used to be the same burnt orange the band-plan
            // overlay shades broadcast allocations with, so that a label matched
            // the band it sat in — but a broadcast box is *text*, and at that
            // saturation on the near-black waterfall it read as brown and was
            // hard to make out (issue #145). The band shading keeps the orange;
            // the label takes the brightest hue the other five leave free, which
            // is the gap between SOTA's amber and POTA's green.
            SpotKind::Broadcast => (190, 240, 120),
            // Magenta: the hue the other six leave free, and far enough from
            // PSK Reporter's violet that a reporter ring is not read as a band
            // spot when both are on the map at once.
            SpotKind::HeardMe => (255, 100, 220),
            SpotKind::Rbn => (100, 220, 180),
        }
    }
}

/// One spot from a network feed: a station heard/announced at a frequency, with
/// enough context to place a clickable marker on the panadapter and world map
/// and to pre-fill a log entry on click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    /// Stable id so the UI can merge/update a marker in place and fade it out.
    /// Derived deterministically from (kind, call, rounded freq) by the feed.
    pub id: u64,
    pub kind: SpotKind,
    /// Absolute RF frequency of the spotted station (Hz).
    pub freq_hz: f64,
    /// The spotted (DX / activator / monitored) callsign, uppercased.
    pub call: String,
    /// Who reported it (cluster spotter, or the receiving station for PSK).
    pub spotter: String,
    /// Operating mode as announced ("FT8", "CW", "SSB", …); may be empty.
    pub mode: String,
    /// Free-text: cluster comment, park/summit name, etc.
    pub comment: String,
    /// POTA park ("K-1234") or SOTA summit ("W7W/…") reference, if any.
    pub reference: Option<String>,
    /// Maidenhead grid, if the feed provides one.
    pub grid: Option<String>,
    /// (lat, lon) in degrees for the world map, if known.
    pub loc: Option<(f64, f64)>,
    /// Spot time (unix seconds).
    pub when_utc: i64,
    /// SNR in dB (PSK Reporter), if any.
    pub snr_db: Option<i16>,
}

impl Spot {
    /// Best-effort radio [`Mode`] for click-to-tune from the spot's mode string.
    /// `SSB`/`PHONE` resolve to LSB below 10 MHz and USB above (ham convention);
    /// unknown data modes fall back to USB.
    pub fn radio_mode(&self) -> Option<Mode> {
        let m = self.mode.trim().to_ascii_uppercase();
        match m.as_str() {
            "" => None,
            "SSB" | "PHONE" | "USB/LSB" => {
                Some(if self.freq_hz < 10_000_000.0 { Mode::Lsb } else { Mode::Usb })
            }
            "DATA" | "DIGI" | "DIGITAL" | "MFSK" | "JT65" | "JS8" | "FT8CALL" => Some(Mode::Usb),
            other => other.parse::<Mode>().ok(),
        }
    }

    /// A stable id from the spot's identity, so re-reports of the same station
    /// on (nearly) the same frequency update one marker rather than piling up.
    /// Frequency is bucketed to 100 Hz.
    pub fn make_id(kind: SpotKind, call: &str, freq_hz: f64) -> u64 {
        // FNV-1a over (kind, call, freq bucket) — deterministic, no hashing deps.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |bytes: &[u8]| {
            for &b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        mix(&[kind as u8]);
        mix(call.as_bytes());
        let bucket = (freq_hz / 100.0).round() as i64;
        mix(&bucket.to_le_bytes());
        h
    }
}

// ── KML export ────────────────────────────────────────────────────────────

/// XML text escaping for a KML element's contents.
///
/// KML is XML, so the five entities and nothing else. A `&` in a spotter's
/// comment is common enough ("G4 & friends") and unescaped it makes every map
/// that opens the file reject the whole document. `<` matters as much for the
/// same reason: a spotter's text containing HTML would otherwise open an element
/// rather than sit inside one.
///
/// Deliberately **not** the CSV field helper: CSV quotes, XML escapes, and one
/// function that does both is a function nobody can trust for either.
fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// The spots as a **KML** document, for Google Earth and anything else that
/// reads it (issue #458).
///
/// Pure text, in this crate, for the same reason [`crate::swl_log_to_csv`] is:
/// it is a pure function over data we already hold, so the browser client
/// exports the identical bytes without growing a second implementation.
///
/// **Only spots carrying a position are exported.** `loc` is optional — a
/// cluster spotter's grid may be unknown and a PSK Reporter line may have no
/// grid at all — and a KML pin with no coordinates is not a pin: it is an
/// error, or a pin at null island. Those spots still appear in the CSV the same
/// window offers, so nothing is lost by leaving them off the map.
///
/// Grouped by [`SpotKind`] in [`SpotKind::ALL`] order, so this exporter's
/// layer list and the program's own filter chips agree without either knowing
/// about the other. Each folder carries a style in that kind's stock colour, so
/// Google Earth's layer panel reads like this program's map.
///
/// Coordinates are written **`lon,lat,0` — longitude first**. That is the one
/// thing a hand-written KML gets wrong most often, and the reason pins land in
/// the sea instead of on the operator.
pub fn spots_to_kml(spots: &[Spot]) -> String {
    let mut out = String::with_capacity(spots.len() * 320 + 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
         <Document>\n\
         <name>sdroxide spots</name>\n\
         <description>Stations heard or announced on the network feeds.</description>\n",
    );

    for kind in SpotKind::ALL {
        let placed: Vec<&Spot> = spots
            .iter()
            .filter(|s| s.kind == kind)
            .filter(|s| s.loc.is_some_and(|(lat, lon)| usable(lat, lon)))
            .collect();
        if placed.is_empty() {
            continue;
        }
        let (r, g, b) = kind.default_color();
        // The style id is the kind's index rather than its label, so a label
        // with a space or a colon in it cannot produce an invalid id reference.
        out.push_str(&format!(
            "<Style id=\"spot-{}\">\n\
             <IconStyle><color>ff{r:02x}{g:02x}{b:02x}</color><scale>0.8</scale></IconStyle>\n\
             </Style>\n",
            kind.index(),
        ));
        out.push_str(&format!(
            "<Folder>\n<name>{}</name>\n<styleUrl>#spot-{}</styleUrl>\n",
            xml_text(kind.label()),
            kind.index(),
        ));
        for s in placed {
            let (lat, lon) = s.loc.expect("filtered above");
            out.push_str(&format!(
                "<Placemark>\n\
                 <name>{}</name>\n\
                 <description>{}</description>\n\
                 <styleUrl>#spot-{}</styleUrl>\n\
                 <Point><coordinates>{lon:.6},{lat:.6},0</coordinates></Point>\n\
                 </Placemark>\n",
                xml_text(&s.call),
                xml_text(&spot_description(s)),
                kind.index(),
            ));
        }
        out.push_str("</Folder>\n");
    }

    out.push_str("</Document>\n</kml>\n");
    out
}

/// Whether a spot's position can be written as a pin.
///
/// Both coordinates must be finite and the latitude must be inside the globe.
/// A grid square decoded at a pole can land on exactly ±90, which KML will
/// happily accept and Google Earth will happily draw off the coast of Africa,
/// so the check is not being fussy — it is refusing a position the operator
/// never had.
fn usable(lat: f64, lon: f64) -> bool {
    lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat)
}

/// The description under a placemark: what a spotter would want to read about a
/// pin once it is out in Google Earth, where this program's own tooltips are a
/// long way away.
fn spot_description(s: &Spot) -> String {
    let mut d = String::new();
    if !s.mode.is_empty() {
        d.push_str(&format!("{} · ", s.mode));
    }
    d.push_str(&format!("{:.3} MHz · ", s.freq_hz / 1e6));
    if !s.spotter.is_empty() {
        d.push_str(&format!("via {}", s.spotter));
    }
    if let Some(r) = s.reference.as_deref().filter(|r| !r.is_empty()) {
        d.push_str(&format!(" · {r}"));
    }
    if !s.comment.is_empty() {
        d.push_str(&format!(" · {}", s.comment));
    }
    d
}

#[cfg(test)]
mod kml_tests {
    use super::{Spot, SpotKind, spots_to_kml, usable, xml_text};

    fn spot(kind: SpotKind, call: &str, loc: Option<(f64, f64)>) -> Spot {
        Spot {
            id: 1,
            kind,
            freq_hz: 14_074_000.0,
            call: call.into(),
            spotter: "DX de G4XYZ".into(),
            mode: "FT8".into(),
            comment: String::new(),
            reference: None,
            grid: None,
            loc,
            when_utc: 1_700_000_000,
            snr_db: Some(-7),
        }
    }

    /// The whole point of a KML export: the coordinates land where the station
    /// is. **Longitude first** is the classic mistake here — swapped, the pin
    /// is in the sea and the file still opens, so nothing complains.
    #[test]
    fn a_placemark_puts_longitude_before_latitude() {
        let kml = spots_to_kml(&[spot(SpotKind::Pota, "K5ABC", Some((52.5, -13.25)))]);
        assert!(
            kml.contains("<coordinates>-13.250000,52.500000,0</coordinates>"),
            "KML coordinates are lon,lat,0 — not lat,lon:\n{kml}"
        );
    }

    /// A spotter's text is somebody's free-form comment, and "&" is in it more
    /// often than not. Unescaped, the document is malformed and **every** map
    /// that opens it rejects the whole file — one stray ampersand loses all the
    /// pins, not just that one.
    #[test]
    fn xml_metacharacters_in_a_spot_are_escaped() {
        assert_eq!(xml_text("G4 & friends"), "G4 &amp; friends");
        assert_eq!(xml_text("<b>bold</b>"), "&lt;b&gt;bold&lt;/b&gt;");
        assert_eq!(xml_text(r#"say "hi""#), "say &quot;hi&quot;");
        assert_eq!(xml_text("it's"), "it&apos;s");

        let mut s = spot(SpotKind::DxCluster, "G4ABC", Some((51.0, -1.0)));
        s.comment = "POTA <urgent> & wet".into();
        let kml = spots_to_kml(&[s]);
        assert!(kml.contains("POTA &lt;urgent&gt; &amp; wet"), "{kml}");
        assert!(!kml.contains("<urgent>"), "a comment must not open an element");
    }

    /// A spot with no grid is common — a cluster line with no position, a PSK
    /// report with none. It cannot be a pin, and writing it as one puts a
    /// marker at null island rather than omitting it.
    #[test]
    fn a_spot_with_no_position_is_left_out_rather_than_pinned_at_null_island() {
        let kml = spots_to_kml(&[
            spot(SpotKind::Pota, "K5ABC", Some((52.5, -13.25))),
            spot(SpotKind::Pota, "K9XYZ", None),
        ]);
        assert!(kml.contains("K5ABC"), "the positioned spot must be there");
        assert!(!kml.contains("K9XYZ"), "an unpositioned spot must not be pinned");
        assert_eq!(kml.matches("<Placemark>").count(), 1, "exactly one pin");
    }

    /// A grid that decodes onto a pole is not a position the operator ever had,
    /// and Google Earth will draw it off the coast of Africa without complaint.
    #[test]
    fn a_position_off_the_globe_is_refused() {
        assert!(usable(52.5, -13.25), "an ordinary position passes");
        assert!(usable(0.0, 0.0), "Null Island is a real place");
        assert!(!usable(91.0, 0.0), "a latitude past the pole is not a position");
        assert!(!usable(-90.5, 0.0), "nor past the south pole");
        assert!(!usable(f64::NAN, 0.0), "nor a NaN latitude");
        assert!(!usable(52.5, f64::INFINITY), "nor an infinite longitude");
        assert!(usable(90.0, 0.0), "the pole itself is a latitude, and is kept");

        let kml = spots_to_kml(&[spot(SpotKind::Sota, "W1/AM", Some((120.0, -71.0)))]);
        assert!(!kml.contains("W1/AM"), "an impossible latitude is dropped");
    }

    /// The folders, the styles and the pins have to agree, or Google Earth's
    /// layer panel offers a legend that does not describe the map.
    #[test]
    fn each_kind_gets_its_own_labelled_folder_in_the_canonical_order() {
        let kml = spots_to_kml(&[
            spot(SpotKind::Sota, "W1AM", Some((44.0, -71.0))),
            spot(SpotKind::Pota, "K5ABC", Some((52.5, -13.25))),
        ]);
        // POTA is index 1, SOTA index 2: ALL order, so POTA's folder comes first.
        let pota = kml.find("<name>POTA</name>").expect("a POTA folder");
        let sota = kml.find("<name>SOTA</name>").expect("a SOTA folder");
        assert!(pota < sota, "folders must follow SpotKind::ALL, not the input order");
        assert!(kml.contains("<styleUrl>#spot-1</styleUrl>"), "POTA's style");
        assert!(kml.contains("<styleUrl>#spot-2</styleUrl>"), "SOTA's style");
        assert!(kml.contains("<Style id=\"spot-1\">"), "POTA's style is defined");
        assert!(kml.contains("<Style id=\"spot-2\">"), "SOTA's style is defined");
        // A kind with nothing placed must not leave an empty folder behind.
        assert!(!kml.contains("<name>FREEDV</name>"), "no pins, no folder");
    }

    /// What Google Earth insists on: an XML declaration, the OGC namespace, and
    /// balanced elements. A file missing the namespace opens in some tools and
    /// is a grey square in others.
    #[test]
    fn the_document_is_well_formed_with_the_ogc_namespace() {
        let kml = spots_to_kml(&[spot(SpotKind::FreeDv, "G0ABC", Some((51.5, -0.1)))]);
        assert!(kml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(kml.contains("xmlns=\"http://www.opengis.net/kml/2.2\""));
        for (open, close) in [
            ("<kml", "</kml>"),
            ("<Document>", "</Document>"),
            ("<Folder>", "</Folder>"),
            ("<Placemark>", "</Placemark>"),
            ("<Point>", "</Point>"),
        ] {
            assert_eq!(kml.matches(open).count(), kml.matches(close).count(), "{open}/{close}");
        }
        assert!(kml.trim_end().ends_with("</kml>"), "the document must be closed");
    }

    /// No spots at all is a real state — a listener with no feed — and it must
    /// still produce a file an editor will open, not an empty string and not a
    /// panic.
    #[test]
    fn no_spots_still_produces_a_document() {
        let kml = spots_to_kml(&[]);
        assert!(kml.contains("</kml>"), "an empty document is still a document");
        assert!(!kml.contains("<Placemark>"));
        assert!(!kml.contains("<Folder>"));
    }

    /// The description carries what a spotter wants in Google Earth, where this
    /// program's tooltips are a long way away.
    #[test]
    fn the_description_carries_the_frequency_the_mode_and_the_spotter() {
        let s = spot(SpotKind::Pota, "K5ABC", Some((52.5, -13.25)));
        let kml = spots_to_kml(&[s]);
        // The Document carries a description of its own, so the placemark's is
        // the one inside it.
        let placemark = kml.split("<Placemark>").nth(1).expect("a placemark");
        let d = placemark
            .split("<description>")
            .nth(1)
            .and_then(|r| r.split("</description>").next())
            .expect("a description");
        assert!(d.contains("14.074"), "the frequency: {d}");
        assert!(d.contains("FT8"), "the mode: {d}");
        assert!(d.contains("via "), "the spotter: {d}");
    }
}
