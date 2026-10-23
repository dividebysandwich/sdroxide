use serde::{Deserialize, Serialize};

use crate::Mode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryChannel {
    pub id: u32,
    pub name: String,
    pub freq_hz: f64,
    pub mode: Mode,
    pub filter_lo: f32,
    pub filter_hi: f32,
    /// The [`MemoryFolder`] this memory is filed under, `None` for the top
    /// level. Defaulted so a `memories.json` written before folders existed
    /// still loads. A dangling id (its folder gone from under it) reads as
    /// unfiled rather than as invisible.
    #[serde(default)]
    pub folder: Option<u32>,
    /// The RTTY modem setup this memory was stored with; `None` for a memory
    /// stored in any other mode (and for a `memories.json` written before
    /// this existed, which recalls on whatever the modem is already set to).
    #[serde(default)]
    pub rtty: Option<RttyMemory>,
    /// The repeater setup this memory was stored with — the shift, the tone
    /// that goes out under the voice, and whether the over opens on a 1750 Hz
    /// burst.
    ///
    /// `None` only for a `memories.json` written before this existed, and a
    /// recall reads that as plain simplex with no tone — the same as an
    /// explicit `Some(RepeaterState::default())`. It has to: nothing in the UI
    /// can draw the difference, so a channel with no stored setup looks exactly
    /// like a simplex one, and the operator who recalls it off a list that says
    /// "145.500 NFM" must not end up transmitting 600 kHz down with the last
    /// repeater's tone still going out (issue #204).
    ///
    /// Every memory stored since this existed carries an explicit setup,
    /// including a plainly simplex one — a channel that says "simplex, no tone"
    /// has to be able to take the radio *out* of the shift the last recall put
    /// it in, or working down a list of repeater memories would leave the shift
    /// standing on the simplex channel at the end of it.
    #[serde(default)]
    pub repeater: Option<crate::RepeaterState>,
    /// The antenna socket this channel was stored on, by the name the front end
    /// gives the port (`"ANT2"`, `"Antenna B"`).
    ///
    /// `None` — and that is every channel stored on a radio with one antenna,
    /// as well as every one written before this existed — means "leave the
    /// antenna alone", which is the opposite of how [`Self::repeater`] reads an
    /// absent field and deliberately so. A shift left standing from the last
    /// recall transmits on the wrong frequency; an antenna left standing is at
    /// worst the wrong one to listen on, and inventing "put it back on ANT1"
    /// for a list of channels that never said anything about antennas would
    /// move a relay every time one was recalled (issue #235).
    #[serde(default)]
    pub antenna: Option<String>,
    /// The colour this memory's label is painted in on the waterfall, as sRGB.
    /// `None` means "use the global memory label colour" — which is what every
    /// memory written before this existed means, so nothing changes for them.
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

/// The RTTY modem setup captured alongside a memory stored in RTTY mode.
///
/// The dial position is only half of an RTTY memory: a commercial broadcast
/// (DWD weather, 50 baud / 450 Hz shift, reverse) decodes as nonsense on the
/// amateur defaults, so recalling the frequency without the setup would hand
/// the operator a station they still have to configure from notes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RttyMemory {
    pub baud: f32,
    pub shift_hz: f32,
    pub reverse: bool,
    pub afc: bool,
}

/// A named folder in the memory list. One level deep — a folder holds
/// memories, not other folders — and deleting one moves its contents back to
/// the top level rather than deleting them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFolder {
    pub id: u32,
    pub name: String,
}

/// One entry of a band-stack register (PowerSDR-style: up to 3 per band,
/// pressing the band button again cycles them).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandStackEntry {
    pub freq_hz: f64,
    pub mode: Mode,
    pub filter_lo: f32,
    pub filter_hi: f32,
}

/// How the memory list is ordered on screen.
///
/// A view preference and nothing more: `memories.json` keeps its channels in
/// the order they were stored, each client draws them in the order its own
/// operator asked for, and a memory scan still works through the store as it
/// stands. So two screens on one station may list the same memories
/// differently, which is the point — the sort belongs to whoever is reading
/// the list. Persisted in `[ui]` beside the rest of the screen's preferences.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySort {
    /// By name, compared without regard to case, so `alpha` and `Alpha` file
    /// together rather than in two blocks.
    Name,
    /// By dial frequency.
    Freq,
    /// By band: every band's channels together in band order, general
    /// coverage last, and by frequency inside each.
    ///
    /// Not a slower spelling of [`MemorySort::Freq`] — a marine or airband
    /// channel sorts with the rest of the general coverage instead of landing
    /// between the amateur bands it happens to lie between.
    Band,
    /// The order they were stored in — the historic behaviour, and the order
    /// the file itself holds.
    ///
    /// Declared last and `#[serde(other)]` for the same reason
    /// [`crate::FontSize`] is: a typo in a hand-edited `config.toml` degrades
    /// to this order rather than throwing the whole `[ui]` table away.
    #[default]
    #[serde(other)]
    Stored,
}

impl MemorySort {
    /// Every order, as the picker offers them.
    pub const ALL: [MemorySort; 4] =
        [MemorySort::Stored, MemorySort::Name, MemorySort::Freq, MemorySort::Band];

    pub fn label(self) -> &'static str {
        match self {
            MemorySort::Name => "Name",
            MemorySort::Freq => "Freq",
            MemorySort::Band => "Band",
            MemorySort::Stored => "Stored",
        }
    }

    /// The order `memories` reads in, as indices into it, reversed when
    /// `descending`.
    ///
    /// Indices rather than a sorted copy because the caller is a list being
    /// redrawn every frame, and cloning every name to draw them in a different
    /// order would be paying for the sort sixty times a second.
    ///
    /// Every comparison ends on the id, so the order is total: no two channels
    /// ever compare equal, and reversing it is therefore exactly the descending
    /// order rather than something that shuffles whatever tied.
    pub fn order(self, memories: &[MemoryChannel], descending: bool) -> Vec<usize> {
        let mut order: Vec<usize> = (0..memories.len()).collect();
        match self {
            // Already in it.
            MemorySort::Stored => {}
            MemorySort::Name => {
                let keys: Vec<String> = memories.iter().map(|m| m.name.to_lowercase()).collect();
                order.sort_by(|&a, &b| {
                    keys[a]
                        .cmp(&keys[b])
                        .then(memories[a].freq_hz.total_cmp(&memories[b].freq_hz))
                        .then(memories[a].id.cmp(&memories[b].id))
                });
            }
            MemorySort::Freq => order.sort_by(|&a, &b| {
                memories[a]
                    .freq_hz
                    .total_cmp(&memories[b].freq_hz)
                    .then(memories[a].id.cmp(&memories[b].id))
            }),
            MemorySort::Band => {
                // The rank is the band's place in `Band::ALL`, which is the
                // order the band buttons are in and puts `Gen` last. Worked out
                // once per channel rather than once per comparison: each one
                // costs a band-plan lookup.
                let rank = |hz: f64| {
                    let band = crate::Band::containing(hz);
                    crate::Band::ALL.iter().position(|&b| b == band).unwrap_or(usize::MAX)
                };
                let keys: Vec<usize> = memories.iter().map(|m| rank(m.freq_hz)).collect();
                order.sort_by(|&a, &b| {
                    keys[a]
                        .cmp(&keys[b])
                        .then(memories[a].freq_hz.total_cmp(&memories[b].freq_hz))
                        .then(memories[a].id.cmp(&memories[b].id))
                });
            }
        }
        if descending {
            order.reverse();
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chan(id: u32, name: &str, freq_hz: f64) -> MemoryChannel {
        MemoryChannel {
            id,
            name: name.into(),
            freq_hz,
            mode: Mode::Usb,
            filter_lo: 300.0,
            filter_hi: 2700.0,
            folder: None,
            rtty: None,
            repeater: None,
            antenna: None,
            color: None,
        }
    }

    /// The four orders, on a list stored in none of them.
    #[test]
    fn each_order_is_the_one_it_says() {
        let mems = vec![
            chan(1, "Zulu", 145_500_000.0),
            chan(2, "alpha", 14_070_000.0),
            // In no band at all — the gap between the airband (top 137) and
            // 2 m (bottom 144): sorts between the other two by frequency, and
            // last of all by band.
            chan(3, "Mike", 140_000_000.0),
        ];
        let names = |sort: MemorySort, desc: bool| -> Vec<&str> {
            sort.order(&mems, desc).into_iter().map(|i| mems[i].name.as_str()).collect()
        };
        assert_eq!(names(MemorySort::Stored, false), ["Zulu", "alpha", "Mike"]);
        assert_eq!(names(MemorySort::Stored, true), ["Mike", "alpha", "Zulu"]);
        // Case-insensitively, or "Zulu" and "Mike" would both sort ahead of
        // "alpha" on their capitals alone.
        assert_eq!(names(MemorySort::Name, false), ["alpha", "Mike", "Zulu"]);
        assert_eq!(names(MemorySort::Freq, false), ["alpha", "Mike", "Zulu"]);
        assert_eq!(names(MemorySort::Freq, true), ["Zulu", "Mike", "alpha"]);
        assert_eq!(names(MemorySort::Band, false), ["alpha", "Zulu", "Mike"]);
    }

    /// Every order is total — no two channels ever compare equal — so the
    /// descending list is the ascending one backwards, ties and all, rather
    /// than something that shuffles whatever tied.
    #[test]
    fn ties_break_on_the_id() {
        let mems: Vec<MemoryChannel> = (1..=4).map(|id| chan(id, "net", 7_100_000.0)).collect();
        for sort in MemorySort::ALL {
            assert_eq!(sort.order(&mems, false), [0, 1, 2, 3], "{sort:?} ascending");
            assert_eq!(sort.order(&mems, true), [3, 2, 1, 0], "{sort:?} descending");
        }
    }

    /// A hand-edited `config.toml` with an order nobody has heard of degrades
    /// to the stored order rather than throwing the whole `[ui]` table away.
    #[test]
    fn an_unknown_order_reads_as_stored() {
        let sort: MemorySort = serde_json::from_str("\"Ascending\"").expect("degrades");
        assert_eq!(sort, MemorySort::Stored);
    }
}
