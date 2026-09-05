use serde::{Deserialize, Serialize};

use crate::Region;

/// Amateur bands plus general coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Band {
    M160,
    M80,
    M60,
    M40,
    M30,
    M20,
    M17,
    M15,
    M12,
    M10,
    M6,
    M2,
    Gen,
    /// 70 cm. Appended rather than placed after [`Band::M2`] because `Band` is
    /// postcard-encoded by declaration index and stored in band stacks and
    /// memories; [`Band::ALL`] puts it where it belongs on screen.
    M70,
    /// 4 m — 70 MHz, between 6 m and 2 m. Appended for the same reason as
    /// [`Band::M70`], and the first band here that a region can simply not
    /// have: 70 MHz is an amateur allocation in Region 1 alone.
    M4,
    /// 1.25 m — 220 MHz, between 2 m and 70 cm. Region 2's alone.
    ///
    /// `M125` reads "1.25 m" — the way every band plan and every ADIF file
    /// writes this band — and not 125 metres, which is not a band at all.
    M125,
    /// 33 cm — 902 MHz. Region 2's alone, like 1.25 m.
    ///
    /// The first of the centimetre bands to take a `Cm` prefix. [`Band::M70`]
    /// keeps its name because it shipped with it and that name is in every
    /// `bandplan.json` already written, but the scheme could not be carried on:
    /// `M6` is already 6 m, so 6 cm cannot be `M6`, and half a naming scheme is
    /// worse than a new one. Everything from here up is `Cm`.
    Cm33,
    /// 23 cm — 1240–1300 MHz. The first of the microwave bands all three
    /// regions share.
    Cm23,
    /// 13 cm — 2300–2450 MHz.
    Cm13,
    /// 9 cm — 3.4 GHz in Region 1, 3.3–3.5 GHz elsewhere.
    Cm9,
    /// 6 cm — 5650 MHz up. Called 6 cm by the IARU Region 1 VHF handbook, the
    /// RSGB, the WIA and the NRRL, and 5 cm in the Americas; see
    /// [`Band::label_in`], which says whichever the station's own region does.
    Cm6,
    /// 3 cm — 10.0–10.5 GHz, the same allocation in all three regions.
    ///
    /// Appended for the reason [`Band::M70`] gives, and reached by a
    /// transverter in every station but one: the IC-905 carries Icom's own
    /// 10 GHz unit *inside* it, so its 3 cm is the radio's own band and not an
    /// entry in the transverter table (issue #326).
    Cm3,
}

impl Band {
    pub const ALL: [Band; 22] = [
        Band::M160,
        Band::M80,
        Band::M60,
        Band::M40,
        Band::M30,
        Band::M20,
        Band::M17,
        Band::M15,
        Band::M12,
        Band::M10,
        Band::M6,
        Band::M4,
        Band::M2,
        Band::M125,
        Band::M70,
        Band::Cm33,
        Band::Cm23,
        Band::Cm13,
        Band::Cm9,
        Band::Cm6,
        Band::Cm3,
        Band::Gen,
    ];

    /// This band's place in [`Band::ALL`] — the order the bands are shown in,
    /// and a stable index for a table with one entry per band.
    ///
    /// Not the declaration order, which is the postcard wire order and has 70
    /// cm, 4 m and 1.25 m appended out of place; [`Band::ALL`] is the order an
    /// operator reads.
    pub fn index(self) -> usize {
        Band::ALL.iter().position(|b| *b == self).unwrap_or(Band::ALL.len() - 1)
    }

    /// The band's name, as the station's configured region writes it.
    pub fn label(self) -> &'static str {
        self.label_in(crate::region())
    }

    /// The band's name as `region` writes it.
    ///
    /// Only one band needs the parameter, and it needs it badly: the 5650 MHz
    /// band is **6 cm** to the IARU Region 1 VHF handbook, the RSGB, the WIA and
    /// the NRRL, and **5 cm** to plans across the other two regions. Neither is
    /// a nickname — each is the name in a band plan — so the honest thing is to
    /// print the one the operator's own plan uses rather than to pick a winner.
    ///
    /// The log is a separate question: ADIF defines `6cm` for 5.65–5.925 GHz and
    /// nothing called `5cm`, so [`crate::adif_band`] says `6cm` wherever the
    /// station is. This is what the operator reads; that is what the file says.
    pub fn label_in(self, region: Region) -> &'static str {
        match self {
            Band::M160 => "160M",
            Band::M80 => "80M",
            Band::M60 => "60M",
            Band::M40 => "40M",
            Band::M30 => "30M",
            Band::M20 => "20M",
            Band::M17 => "17M",
            Band::M15 => "15M",
            Band::M12 => "12M",
            Band::M10 => "10M",
            Band::M6 => "6M",
            Band::M4 => "4M",
            Band::M2 => "2M",
            Band::M125 => "1.25M",
            Band::M70 => "70CM",
            Band::Cm33 => "33CM",
            Band::Cm23 => "23CM",
            Band::Cm13 => "13CM",
            Band::Cm9 => "9CM",
            Band::Cm6 => match region {
                Region::R1 => "6CM",
                Region::R2 | Region::R3 => "5CM",
            },
            Band::Cm3 => "3CM",
            Band::Gen => "GEN",
        }
    }

    /// Band edges in Hz for the station's configured region (see
    /// [`crate::region`]). `None` for general coverage.
    pub fn edges(self) -> Option<(f64, f64)> {
        self.edges_in(crate::region())
    }

    /// Band edges in Hz for `region`, or `None` for general coverage — and for
    /// a band the station's [`crate::BandPlan`] does not give that region.
    ///
    /// Read from the installed band plan, which is `bandplan.json` in the
    /// config directory once the operator has one and
    /// [`Band::iaru_default_edges_in`] until then.
    pub fn edges_in(self, region: Region) -> Option<(f64, f64)> {
        crate::band_plan().region(region).edges(self)
    }

    /// The built-in IARU edges for `region` — the seed for a fresh
    /// `bandplan.json`, and what is used until one is loaded.
    ///
    /// The *allocation*, not any one country's licence conditions: a national
    /// administration may grant less (Germany's 10 m stops at 29.700 like
    /// everyone's, but its 2 m ends at 146 while Region 1 as a whole varies) and
    /// occasionally more. These are the widest edges the region's amateurs
    /// share, which is what a shipped default can honestly be — an operator who
    /// needs their own licence's edges puts them in the file.
    ///
    /// Where the bands differ:
    /// - **160 m** starts at 1.810 in Region 1 and at 1.800 in Regions 2 and 3.
    /// - **80 m** ends at 3.800 / 4.000 / 3.900 in Regions 1 / 2 / 3.
    /// - **40 m** ends at 7.200 outside Region 2, which has the whole
    ///   7.000–7.300 to itself.
    /// - **6 m** ends at 52 MHz in Region 1 and 54 MHz elsewhere.
    /// - **4 m** is Region 1's alone. Regions 2 and 3 have no 70 MHz amateur
    ///   allocation at all, so there the band is *absent* rather than narrower.
    /// - **2 m** ends at 146 MHz in Region 1 and 148 MHz elsewhere.
    /// - **1.25 m** and **33 cm** are Region 2's alone, for the same reason 4 m
    ///   is Region 1's: 220 MHz and 902 MHz are not amateur allocations in the
    ///   other two regions at all.
    /// - **70 cm** is 430–440 in Region 1, 420–450 in Region 2 and 430–450 in
    ///   Region 3.
    /// - **9 cm** is 3400–3475 in Region 1 and 3300–3500 elsewhere.
    /// - **6 cm** (5 cm in the Americas — see [`Band::label_in`]) starts at
    ///   5650 everywhere and ends at 5850 in Region 1, 5925 elsewhere.
    ///
    /// 30 m through 10 m, the WRC-15 60 m allocation, and 23 cm and 13 cm, are
    /// the same everywhere.
    pub fn iaru_default_edges_in(self, region: Region) -> Option<(f64, f64)> {
        let by_region = |r1: (f64, f64), r2: (f64, f64), r3: (f64, f64)| match region {
            Region::R1 => Some(r1),
            Region::R2 => Some(r2),
            Region::R3 => Some(r3),
        };
        match self {
            Band::M160 => by_region(
                (1_810_000.0, 2_000_000.0),
                (1_800_000.0, 2_000_000.0),
                (1_800_000.0, 2_000_000.0),
            ),
            Band::M80 => by_region(
                (3_500_000.0, 3_800_000.0),
                (3_500_000.0, 4_000_000.0),
                (3_500_000.0, 3_900_000.0),
            ),
            // The WRC-15 secondary allocation, identical in all three regions.
            // Region 2's channelised 60 m (five 2.8 kHz channels between 5.332
            // and 5.405) is a US national arrangement inside a different slice
            // of spectrum, not a regional allocation, so it is not modelled
            // here — a licence that grants those channels grants them outside
            // this band whichever region it was issued in.
            Band::M60 => Some((5_351_500.0, 5_366_500.0)),
            Band::M40 => by_region(
                (7_000_000.0, 7_200_000.0),
                (7_000_000.0, 7_300_000.0),
                (7_000_000.0, 7_200_000.0),
            ),
            Band::M30 => Some((10_100_000.0, 10_150_000.0)),
            Band::M20 => Some((14_000_000.0, 14_350_000.0)),
            Band::M17 => Some((18_068_000.0, 18_168_000.0)),
            Band::M15 => Some((21_000_000.0, 21_450_000.0)),
            Band::M12 => Some((24_890_000.0, 24_990_000.0)),
            Band::M10 => Some((28_000_000.0, 29_700_000.0)),
            Band::M6 => by_region(
                (50_000_000.0, 52_000_000.0),
                (50_000_000.0, 54_000_000.0),
                (50_000_000.0, 54_000_000.0),
            ),
            // The IARU Region 1 band plan's 70.000–70.500. Several national
            // licences grant a slice of that rather than the whole (Germany's
            // 4 m is 70.150–70.200), which is what a hand-edited bandplan.json
            // is for. Regions 2 and 3 get `None`: 70 MHz is not an amateur
            // allocation there, and an operator in the Americas pointed at a
            // "band" their licence does not contain is worse served than one
            // told they have no such band.
            Band::M4 => (region == Region::R1).then_some((70_000_000.0, 70_500_000.0)),
            Band::M2 => by_region(
                (144_000_000.0, 146_000_000.0),
                (144_000_000.0, 148_000_000.0),
                (144_000_000.0, 148_000_000.0),
            ),
            // 1.25 m, Region 2's alone. The IARU Region 2 band plan covers
            // 220–225; the US licence grants 222–225 of it, plus 219–220 for
            // point-to-point digital links only, and Canada 219–220 and
            // 222–225. So this is one of the places where the *allocation* is
            // wider than any one licence in it, and a US or Canadian operator
            // who wants the lockout to know that puts 222–225 in their
            // `bandplan.json`.
            Band::M125 => (region == Region::R2).then_some((220_000_000.0, 225_000_000.0)),
            Band::M70 => by_region(
                (430_000_000.0, 440_000_000.0),
                (420_000_000.0, 450_000_000.0),
                (430_000_000.0, 450_000_000.0),
            ),
            // 33 cm, Region 2's alone, and shared with everything from cordless
            // telephones to ISM devices — amateur use is secondary throughout.
            Band::Cm33 => (region == Region::R2).then_some((902_000_000.0, 928_000_000.0)),
            Band::Cm23 => Some((1_240_000_000.0, 1_300_000_000.0)),
            Band::Cm13 => Some((2_300_000_000.0, 2_450_000_000.0)),
            // 9 cm. Region 1's 3400–3475 is the narrower allocation, and several
            // national licences in it stop at 3410 — 5G in the 3.4–3.8 GHz range
            // has been taking the top of this band across Europe, which is
            // exactly the sort of thing a hand-edited `bandplan.json` is for.
            Band::Cm9 => by_region(
                (3_400_000_000.0, 3_475_000_000.0),
                (3_300_000_000.0, 3_500_000_000.0),
                (3_300_000_000.0, 3_500_000_000.0),
            ),
            // 6 cm in Region 1, 5 cm in the other two — the same band under two
            // names, which is [`Band::label_in`]'s business, not this table's.
            Band::Cm6 => by_region(
                (5_650_000_000.0, 5_850_000_000.0),
                (5_650_000_000.0, 5_925_000_000.0),
                (5_650_000_000.0, 5_925_000_000.0),
            ),
            // 3 cm. The one microwave band the three regions agree on exactly,
            // 10.0 to 10.5 GHz throughout — though several national licences
            // stop at 10.45 or carve the middle out, which is what a
            // hand-edited `bandplan.json` is for.
            Band::Cm3 => Some((10_000_000_000.0, 10_500_000_000.0)),
            Band::Gen => None,
        }
    }

    /// The band containing `hz` in the station's configured region, or `Gen` if
    /// none does.
    pub fn containing(hz: f64) -> Band {
        Band::containing_in(hz, crate::region())
    }

    /// The band containing `hz` in `region`, or `Gen` if none does.
    pub fn containing_in(hz: f64, region: Region) -> Band {
        crate::band_plan().region(region).containing(hz)
    }

    /// A reasonable default frequency/mode when jumping to a band with no stack
    /// history.
    ///
    /// One set for every region: each of these sits inside the band in every
    /// region that has the band at all, and inside a part of it the mode belongs
    /// in — which is the most a starting point has to do, since the band stack
    /// replaces it the moment the operator tunes.
    pub fn default_entry(self) -> (f64, crate::Mode) {
        use crate::Mode;
        match self {
            Band::M160 => (1_840_000.0, Mode::Lsb),
            Band::M80 => (3_700_000.0, Mode::Lsb),
            Band::M60 => (5_357_000.0, Mode::Usb),
            Band::M40 => (7_100_000.0, Mode::Lsb),
            Band::M30 => (10_120_000.0, Mode::Cw),
            Band::M20 => (14_200_000.0, Mode::Usb),
            Band::M17 => (18_120_000.0, Mode::Usb),
            Band::M15 => (21_250_000.0, Mode::Usb),
            Band::M12 => (24_940_000.0, Mode::Usb),
            Band::M10 => (28_400_000.0, Mode::Usb),
            Band::M6 => (50_150_000.0, Mode::Usb),
            // 70.200 is the 4 m SSB/CW calling frequency, in the narrow-band
            // part of a band whose bottom 100 kHz is beacons only.
            Band::M4 => (70_200_000.0, Mode::Usb),
            Band::M2 => (145_500_000.0, Mode::Nfm),
            // 223.500 is the 1.25 m national FM simplex calling frequency.
            Band::M125 => (223_500_000.0, Mode::Nfm),
            // 70 cm opens on the RIFP calling frequency: it is the band this
            // mode is meant for, and the band stack overrides this the moment
            // the operator tunes anywhere else.
            Band::M70 => (crate::RIFP_CALLING_HZ, Mode::Rifp),
            // The microwave bands open in their narrow-band segment, on
            // upper sideband, which is where the weak-signal work is: these are
            // bands an operator reaches with a transverter or a Pluto and a
            // dish, not ones they tune across looking for activity.
            //
            // Each of these sits inside the band in every region that has it,
            // which is what `default_entry` has to promise. Where a region works
            // a different part of the band the band stack fixes it on the first
            // tune — 33 cm and 1.25 m exist in Region 2 alone, so there is only
            // one plan to satisfy, but 23 cm through 6 cm are shared and the
            // narrow-band segments do not line up: the Americas call 100 kHz
            // below Region 1 on 23 cm and in a different megahertz on 13 cm.
            Band::Cm33 => (902_100_000.0, Mode::Usb),
            Band::Cm23 => (1_296_200_000.0, Mode::Usb),
            Band::Cm13 => (2_320_200_000.0, Mode::Usb),
            Band::Cm9 => (3_400_100_000.0, Mode::Usb),
            Band::Cm6 => (5_760_100_000.0, Mode::Usb),
            // The 3 cm narrow-band calling frequency, and the same one
            // everywhere: 10368.100 is where a 3 cm contact starts.
            Band::Cm3 => (10_368_100_000.0, Mode::Usb),
            Band::Gen => (7_200_000.0, Mode::Am),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #326: an IC-905 carries Icom's 10 GHz transverter inside the
    /// radio, so 3 cm is a band it *has* rather than one reached through the
    /// transverter table — and it has to be on the band bar like any other.
    #[test]
    fn three_centimetres_is_a_band_in_every_region() {
        for r in Region::ALL {
            assert_eq!(
                Band::Cm3.edges_in(r),
                Some((10_000_000_000.0, 10_500_000_000.0)),
                "3 cm in {r:?}"
            );
            assert_eq!(Band::Cm3.label_in(r), "3CM");
        }
        // The narrow-band calling frequency, which is where a 3 cm contact
        // starts wherever you are.
        assert_eq!(Band::Cm3.default_entry(), (10_368_100_000.0, crate::Mode::Usb));
        // On the bar between 6 cm and general coverage, and nowhere else.
        assert_eq!(Band::ALL.iter().filter(|b| **b == Band::Cm3).count(), 1);
        assert!(Band::Cm3.index() > Band::Cm6.index());
        assert!(Band::Cm3.index() < Band::Gen.index());
    }

    /// Region 1 is the default, and it must still be exactly the band table
    /// sdroxide shipped before regions existed — an installation that never
    /// opens the setting may not find its bands moved underneath it.
    #[test]
    fn region_1_is_unchanged() {
        for (band, edges) in [
            (Band::M160, (1_810_000.0, 2_000_000.0)),
            (Band::M80, (3_500_000.0, 3_800_000.0)),
            (Band::M60, (5_351_500.0, 5_366_500.0)),
            (Band::M40, (7_000_000.0, 7_200_000.0)),
            (Band::M30, (10_100_000.0, 10_150_000.0)),
            (Band::M20, (14_000_000.0, 14_350_000.0)),
            (Band::M17, (18_068_000.0, 18_168_000.0)),
            (Band::M15, (21_000_000.0, 21_450_000.0)),
            (Band::M12, (24_890_000.0, 24_990_000.0)),
            (Band::M10, (28_000_000.0, 29_700_000.0)),
            (Band::M6, (50_000_000.0, 52_000_000.0)),
            (Band::M2, (144_000_000.0, 146_000_000.0)),
            (Band::M70, (430_000_000.0, 440_000_000.0)),
        ] {
            assert_eq!(band.edges_in(Region::R1), Some(edges), "{band:?}");
        }
        assert_eq!(Band::Gen.edges_in(Region::R1), None);
    }

    /// The frequencies that are in one region's band and outside another's.
    /// This is what the setting is *for*, so it is asserted rather than left to
    /// the edge table to imply.
    #[test]
    fn the_regional_differences_are_the_ones_that_matter() {
        // The operator's own example: 446 MHz is 70 cm in the Americas and out
        // of band across most of Europe.
        assert_eq!(Band::containing_in(446_000_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(446_000_000.0, Region::R2), Band::M70);
        assert_eq!(Band::containing_in(446_000_000.0, Region::R3), Band::M70);
        // 70 cm's lower edge: Region 2 alone starts at 420.
        assert_eq!(Band::containing_in(425_000_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(425_000_000.0, Region::R2), Band::M70);
        assert_eq!(Band::containing_in(425_000_000.0, Region::R3), Band::Gen);
        // 40 m above 7.200 is Region 2's alone.
        assert_eq!(Band::containing_in(7_250_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(7_250_000.0, Region::R2), Band::M40);
        assert_eq!(Band::containing_in(7_250_000.0, Region::R3), Band::Gen);
        // 80 m: Region 2 to 4.000, Region 3 to 3.900, Region 1 to 3.800.
        assert_eq!(Band::containing_in(3_850_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(3_850_000.0, Region::R2), Band::M80);
        assert_eq!(Band::containing_in(3_850_000.0, Region::R3), Band::M80);
        assert_eq!(Band::containing_in(3_950_000.0, Region::R3), Band::Gen);
        assert_eq!(Band::containing_in(3_950_000.0, Region::R2), Band::M80);
        // 160 m's lower edge.
        assert_eq!(Band::containing_in(1_805_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(1_805_000.0, Region::R2), Band::M160);
        // 4 m is Region 1's alone — a band that is absent elsewhere rather
        // than merely a different width. 70.174 is the FT8 frequency on it.
        assert_eq!(Band::containing_in(70_174_000.0, Region::R1), Band::M4);
        assert_eq!(Band::containing_in(70_174_000.0, Region::R2), Band::Gen);
        assert_eq!(Band::containing_in(70_174_000.0, Region::R3), Band::Gen);
        assert_eq!(Band::M4.edges_in(Region::R1), Some((70_000_000.0, 70_500_000.0)));
        assert_eq!(Band::M4.edges_in(Region::R2), None);
        assert_eq!(Band::M4.edges_in(Region::R3), None);
        // 6 m and 2 m are 2 MHz wider outside Region 1.
        assert_eq!(Band::containing_in(53_000_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(53_000_000.0, Region::R2), Band::M6);
        assert_eq!(Band::containing_in(147_000_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(147_000_000.0, Region::R3), Band::M2);
        // 1.25 m and 33 cm are Region 2's alone, the mirror image of 4 m: absent
        // elsewhere rather than narrower.
        for (hz, band) in [(223_500_000.0, Band::M125), (903_000_000.0, Band::Cm33)] {
            assert_eq!(Band::containing_in(hz, Region::R2), band);
            for r in [Region::R1, Region::R3] {
                assert_eq!(Band::containing_in(hz, r), Band::Gen, "{band:?} in {r:?}");
                assert_eq!(band.edges_in(r), None, "{band:?} in {r:?}");
            }
        }
        // 9 cm: Region 1 has 3400–3475, the other two 3300–3500, so a frequency
        // either side of Region 1's window is 9 cm elsewhere and nothing there.
        for hz in [3_350_000_000.0, 3_490_000_000.0] {
            assert_eq!(Band::containing_in(hz, Region::R1), Band::Gen);
            assert_eq!(Band::containing_in(hz, Region::R2), Band::Cm9);
            assert_eq!(Band::containing_in(hz, Region::R3), Band::Cm9);
        }
        assert_eq!(Band::containing_in(3_400_100_000.0, Region::R1), Band::Cm9);
        // 6 cm / 5 cm: the top 75 MHz is outside the Region 1 allocation.
        assert_eq!(Band::containing_in(5_900_000_000.0, Region::R1), Band::Gen);
        assert_eq!(Band::containing_in(5_900_000_000.0, Region::R2), Band::Cm6);
        assert_eq!(Band::containing_in(5_900_000_000.0, Region::R3), Band::Cm6);
        // 23 cm and 13 cm are the same everywhere, which is worth saying: they
        // are the two microwave bands an operator can move between regions with.
        for r in Region::ALL {
            assert_eq!(Band::Cm23.edges_in(r), Some((1_240_000_000.0, 1_300_000_000.0)));
            assert_eq!(Band::Cm13.edges_in(r), Some((2_300_000_000.0, 2_450_000_000.0)));
            assert_eq!(Band::containing_in(1_296_174_000.0, r), Band::Cm23, "{r:?}");
            assert_eq!(Band::containing_in(2_320_174_000.0, r), Band::Cm13, "{r:?}");
        }
    }

    /// The 5650 MHz band is 6 cm in Region 1 and 5 cm in the other two, and both
    /// names are in published band plans — so the label follows the region
    /// rather than picking one and being wrong half the time.
    #[test]
    fn the_5650_band_is_named_the_way_the_region_names_it() {
        assert_eq!(Band::Cm6.label_in(Region::R1), "6CM");
        assert_eq!(Band::Cm6.label_in(Region::R2), "5CM");
        assert_eq!(Band::Cm6.label_in(Region::R3), "5CM");
        // Every other band reads the same wherever the station is.
        for band in Band::ALL {
            if band == Band::Cm6 {
                continue;
            }
            assert_eq!(band.label_in(Region::R1), band.label_in(Region::R2), "{band:?}");
            assert_eq!(band.label_in(Region::R1), band.label_in(Region::R3), "{band:?}");
        }
        // And no two bands share a label, or a band button would name two.
        for region in Region::ALL {
            let mut seen: Vec<&str> = Vec::new();
            for band in Band::ALL {
                let l = band.label_in(region);
                assert!(!seen.contains(&l), "{l} is used twice in {region:?}");
                seen.push(l);
            }
        }
    }

    /// Every band's edges have to be the right way round and disjoint from
    /// every other band's, in every region — `containing` returns the first
    /// match, so an overlap would silently hide a band.
    #[test]
    fn edges_are_ordered_and_disjoint_in_every_region() {
        for region in Region::ALL {
            let mut spans: Vec<(Band, (f64, f64))> =
                Band::ALL.iter().filter_map(|&b| b.edges_in(region).map(|e| (b, e))).collect();
            for (b, (lo, hi)) in &spans {
                assert!(lo < hi, "{b:?} in {region:?}: {lo} >= {hi}");
            }
            spans.sort_by(|a, b| a.1.0.total_cmp(&b.1.0));
            for w in spans.windows(2) {
                assert!(w[0].1.1 < w[1].1.0, "{:?} and {:?} overlap in {region:?}", w[0].0, w[1].0);
            }
        }
    }

    /// A starting frequency outside its own band would drop the operator into
    /// general coverage — and, with `tx_ham_only` set, into a transmit lockout
    /// — the moment they pressed a band button.
    #[test]
    fn every_default_entry_is_inside_its_band_in_every_region() {
        for region in Region::ALL {
            for band in Band::ALL {
                let Some((lo, hi)) = band.edges_in(region) else { continue };
                let (hz, _) = band.default_entry();
                assert!(
                    (lo..=hi).contains(&hz),
                    "{band:?} opens on {hz} Hz, outside {lo}..{hi} in {region:?}"
                );
            }
        }
    }
}
