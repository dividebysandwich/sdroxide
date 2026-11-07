//! Published transponder, beacon and telemetry frequencies for the satellites
//! the tracker knows about.
//!
//! This is reference data, not something derived from the element sets: a TLE
//! says where a satellite is, never what it listens on. The table below is
//! transcribed from the AMSAT frequency list and the operators' own status
//! pages, keyed by NORAD catalogue number so it survives a satellite being
//! renamed in the CelesTrak listing.
//!
//! Frequencies move. Transponders get switched to a different mode, FM birds
//! are put into schedule-only operation, and a bird that has been quiet for a
//! year comes back on a different passband. Nothing here is authoritative
//! against the satellite itself, which is why this table is only half the
//! story: the operator can add and correct entries of their own in
//! [`sdroxide_types::SatConfig`], and those win over anything in this file.

use std::sync::OnceLock;

pub use sdroxide_types::{Passband, SatFreqs, SatLink, fmt_sat_mhz as fmt_mhz};

/// The built-in table, built once and shared.
pub fn builtin() -> &'static [SatFreqs] {
    static TABLE: OnceLock<Vec<SatFreqs>> = OnceLock::new();
    TABLE.get_or_init(build)
}

/// Look a satellite up in the built-in table.
pub fn builtin_for(norad_id: u64) -> Option<&'static SatFreqs> {
    builtin().iter().find(|s| s.norad_id == norad_id)
}

fn build() -> Vec<SatFreqs> {
    use Passband as P;
    let (down, duplex) = (SatLink::down, SatLink::duplex);

    vec![
        // The geostationary one. Its narrowband transponder is the only
        // amateur satellite passband that can be left tuned and worked all day.
        SatFreqs::new(
            crate::satellites::QO100_NORAD,
            "QO-100 / Es'hail-2",
            vec![
                duplex(
                    "NB transponder",
                    "SSB/CW/digital",
                    P::range(2400.000, 2400.500),
                    P::range(10489.500, 10490.000),
                )
                .note("non-inverting; 2.4 GHz up, 10 GHz down"),
                // Beacon blocks as in the AMSAT-DL band plan.
                down("NB lower beacon", "CW", P::range(10489.500, 10489.505)),
                down("NB middle beacon", "BPSK 400bd", P::at(10489.750))
                    .note("multimedia beacon; the frequency reference"),
                down("NB upper beacon", "CW", P::range(10489.990, 10490.000)),
                duplex(
                    "WB transponder",
                    "DVB-S2",
                    P::range(2401.500, 2409.500),
                    P::range(10491.000, 10499.000),
                )
                .note("amateur television"),
            ],
        ),
        SatFreqs::new(
            25544,
            "ISS",
            vec![
                duplex("FM voice", "FM", P::at(145.200), P::at(145.800))
                    .note("uplink 144.490 in ITU regions 2 and 3"),
                // ARISS has run its SSTV events on 70 cm since 2026 (#622); the
                // 2 m channel is kept for the older ones.
                down("SSTV 70 cm", "SSTV FM / PD120", P::at(437.550))
                    .note("event-driven; check ARISS"),
                down("SSTV 2 m", "SSTV FM / PD120", P::at(145.800))
                    .note("event-driven; check ARISS"),
                duplex("APRS digipeater", "AX.25 1k2 AFSK", P::at(145.825), P::at(145.825))
                    .note("simplex; path ARISS"),
                duplex("Cross-band repeater", "FM", P::at(145.990), P::at(437.800))
                    .note("CTCSS 67.0 Hz on the uplink"),
            ],
        ),
        SatFreqs::new(
            7530,
            "AO-7",
            vec![
                duplex(
                    "Mode A transponder",
                    "SSB/CW",
                    P::range(145.850, 145.950),
                    P::range(29.400, 29.500),
                )
                .note("non-inverting; alternates with mode B roughly daily"),
                duplex(
                    "Mode B transponder",
                    "SSB/CW",
                    P::range(432.125, 432.175),
                    P::range(145.925, 145.975),
                )
                .note("inverting")
                .inverting(),
                down("Mode A beacon", "CW", P::at(29.502)),
                down("Mode B beacon", "CW", P::at(145.972)),
            ],
        ),
        SatFreqs::new(
            24278,
            "FO-29",
            vec![
                duplex(
                    "V/U transponder",
                    "SSB/CW",
                    P::range(145.900, 146.000),
                    P::range(435.800, 435.900),
                )
                .note("inverting; LSB up, USB down")
                .inverting(),
                down("Beacon", "CW", P::at(435.795)),
            ],
        ),
        SatFreqs::new(
            27607,
            "SO-50",
            vec![
                duplex("FM repeater", "FM", P::at(145.850), P::at(436.795))
                    .note("CTCSS 67.0 Hz; arm the 10-minute timer with a 74.4 Hz burst"),
            ],
        ),
        SatFreqs::new(
            39444,
            "AO-73 / FUNcube-1",
            vec![
                duplex(
                    "U/V transponder",
                    "SSB/CW",
                    P::range(435.130, 435.150),
                    P::range(145.950, 145.970),
                )
                .note("inverting; transponder runs in eclipse, telemetry in sunlight")
                .inverting(),
                down("Telemetry", "BPSK 1k2", P::at(145.935)),
            ],
        ),
        SatFreqs::new(
            43803,
            "JO-97 / JY1SAT",
            vec![
                duplex(
                    "U/V transponder",
                    "SSB/CW",
                    P::range(435.100, 435.140),
                    P::range(145.855, 145.895),
                )
                .note("inverting")
                .inverting(),
                down("Telemetry / SSDV", "BPSK 1k2", P::at(145.840)),
            ],
        ),
        SatFreqs::new(
            44909,
            "RS-44",
            vec![
                duplex(
                    "U/V transponder",
                    "SSB/CW",
                    P::range(145.935, 145.995),
                    P::range(435.610, 435.670),
                )
                .note("inverting; a high, slow orbit, so passes run long")
                .inverting(),
                down("Beacon", "CW", P::at(435.605)),
            ],
        ),
        SatFreqs::new(
            50466,
            "XW-3 / CAS-9",
            vec![
                duplex(
                    "U/V transponder",
                    "SSB/CW",
                    P::range(435.170, 435.190),
                    P::range(145.860, 145.880),
                )
                .note("inverting")
                .inverting(),
                down("CW beacon", "CW", P::at(145.845)),
            ],
        ),
        SatFreqs::new(
            53109,
            "IO-117 / GreenCube",
            vec![
                duplex("Digipeater", "GMSK 1k2", P::at(435.310), P::at(435.310))
                    .note("simplex store-and-forward; a MEO orbit, so it is workable for hours"),
            ],
        ),
        SatFreqs::new(
            43017,
            "AO-91 / RadFxSat",
            vec![
                duplex("FM repeater", "FM", P::at(435.250), P::at(145.960))
                    .note("CTCSS 67.0 Hz; sunlit passes only"),
            ],
        ),
        SatFreqs::new(
            43678,
            "PO-101 / Diwata-2",
            vec![
                duplex("FM repeater", "FM", P::at(437.500), P::at(145.900))
                    .note("CTCSS 141.3 Hz; scheduled operation only"),
            ],
        ),
        SatFreqs::new(
            44881,
            "TO-108 / CAS-6",
            vec![
                duplex(
                    "U/V transponder",
                    "SSB/CW",
                    P::range(435.270, 435.290),
                    P::range(145.915, 145.935),
                )
                .note("inverting")
                .inverting(),
                down("CW beacon", "CW", P::at(145.910)),
            ],
        ),
        // Not amateur, but they are the reason half of us own a 137 MHz
        // antenna, and their downlinks feed the same weather-fax habit as the
        // HF charts. Their element sets are not in the amateur group, so they
        // only appear once the operator adds them in the TLE manager.
        SatFreqs::new(
            25338,
            "NOAA-15",
            vec![down("APT", "FM / APT 2400 Hz", P::at(137.620)).note("polar, ~102 min")],
        ),
        SatFreqs::new(
            28654,
            "NOAA-18",
            vec![down("APT", "FM / APT 2400 Hz", P::at(137.9125)).note("polar, ~102 min")],
        ),
        SatFreqs::new(
            33591,
            "NOAA-19",
            vec![down("APT", "FM / APT 2400 Hz", P::at(137.100)).note("polar, ~102 min")],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_keyed_by_a_unique_catalogue_number() {
        let mut ids: Vec<u64> = builtin().iter().map(|s| s.norad_id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "two entries share a NORAD id");
        assert!(n >= 15, "only {n} satellites in the table");
    }

    /// Every row has to be usable: a name, at least one link, and every link
    /// with a direction and a plausible amateur/weather-satellite frequency.
    #[test]
    fn every_entry_is_complete_and_in_a_real_band() {
        for s in builtin() {
            assert!(!s.name.is_empty(), "{} has no name", s.norad_id);
            assert!(!s.links.is_empty(), "{} has no links", s.name);
            for l in &s.links {
                assert!(!l.label.is_empty(), "{}: a link has no label", s.name);
                assert!(!l.mode.is_empty(), "{}: {} has no mode", s.name, l.label);
                assert!(!l.is_empty(), "{}: {} has neither direction", s.name, l.label);
                for band in [l.uplink, l.downlink].into_iter().flatten() {
                    assert!(band.lo_mhz <= band.hi_mhz, "{}: {} inverted", s.name, l.label);
                    // 29 MHz mode A through the 10 GHz QO-100 downlink.
                    assert!(
                        (28.0..=11_000.0).contains(&band.lo_mhz),
                        "{}: {} at {} MHz",
                        s.name,
                        l.label,
                        band.lo_mhz
                    );
                    // No transponder is a megahertz wide; a typo in the high
                    // end is the failure this catches.
                    assert!(
                        band.width_khz() <= 10_000.0,
                        "{}: {} is {} kHz wide",
                        s.name,
                        l.label,
                        band.width_khz()
                    );
                }
            }
        }
    }

    /// The reference cases, spot-checked against the published lists.
    #[test]
    fn the_well_known_satellites_are_on_their_published_frequencies() {
        let qo = builtin_for(crate::satellites::QO100_NORAD).expect("QO-100 in the table");
        let nb = &qo.links[0];
        assert_eq!(nb.downlink.unwrap().lo_mhz, 10489.500);
        assert_eq!(nb.uplink.unwrap().lo_mhz, 2400.000);
        assert_eq!(nb.downlink.unwrap().width_khz(), 500.0);

        let iss = builtin_for(25544).expect("ISS in the table");
        assert!(iss.links.iter().any(|l| l.downlink == Some(Passband::at(145.800))));
        assert!(iss.links.iter().any(|l| l.downlink == Some(Passband::at(437.550))));
        assert!(iss.links.iter().any(|l| l.label.contains("APRS")));

        let so50 = builtin_for(27607).expect("SO-50 in the table");
        assert_eq!(so50.links[0].downlink, Some(Passband::at(436.795)));
        assert!(so50.links[0].note.contains("67.0"), "SO-50 needs its CTCSS tone");

        // The three APT birds, which is what the 137 MHz antenna is for.
        assert_eq!(builtin_for(25338).unwrap().links[0].downlink, Some(Passband::at(137.620)));
        assert_eq!(builtin_for(33591).unwrap().links[0].downlink, Some(Passband::at(137.100)));
    }

    /// The inversion flag drives the satellite lock's transmit mapping, so a
    /// link whose note says "inverting" without the structured flag would tune
    /// the uplink to the wrong end of the passband.
    #[test]
    fn every_note_that_says_inverting_sets_the_flag() {
        for s in builtin() {
            for l in &s.links {
                let says = l.note.starts_with("inverting");
                assert_eq!(l.inverting, says, "{}: {} note vs flag", s.name, l.label);
            }
        }
        // The reference cases, both ways round.
        let rs44 = builtin_for(44909).unwrap();
        assert!(rs44.links[0].inverting, "RS-44's transponder is inverting");
        let qo = builtin_for(crate::satellites::QO100_NORAD).unwrap();
        assert!(!qo.links[0].inverting, "QO-100's NB transponder is not");
    }

    /// Every satellite in [`crate::satellites::POPULAR`] — the ones drawn by
    /// default — has to have frequencies, or clicking the most prominent
    /// markers in the view would open a pass table with nothing to tune to.
    #[test]
    fn every_default_satellite_has_frequencies() {
        for (id, name) in crate::satellites::POPULAR {
            assert!(builtin_for(*id).is_some(), "{name} ({id}) has no frequencies");
        }
    }
}
