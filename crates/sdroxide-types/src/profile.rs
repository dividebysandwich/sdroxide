//! Per-mode settings: the operator values that change with the mode.
//!
//! AGC, squelch, noise reduction and the rest are settings, not properties of
//! the waveform, so the demodulator cannot simply know them. What *is* a
//! property of the waveform is which ones are worth having on, and that is what
//! [`Mode::default_profile`](crate::Mode::default_profile) encodes: a mode
//! worked at the noise floor wants a slow AGC, and changing mode should get it
//! without a trip to the menus. The defaults stay conservative — noise
//! reduction, which carries a make-up gain and an operator's taste with it, is
//! never on unless it has been asked for.
//!
//! Two layers make that work:
//!
//! * The **defaults**, baked into the mode (`Mode::default_profile`). They are
//!   what an operator who has never touched these settings gets.
//! * The **overrides**, what this station has changed them to, per mode, in
//!   `modeprofiles.json`. A field equal to the default is not stored, so the
//!   file only holds real departures, and putting a setting back where it
//!   started forgets it on its own.
//!
//! The effective profile is the overrides laid over the defaults. That is what
//! the engine applies on a mode change, and what the UI compares the live
//! settings against to decide whether there is anything to revert.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{AgcMode, Mode, NrLevel};

/// The values a mode starts with, and what the operator has changed them to.
///
/// Every field is optional: `None` means "no opinion", which is what a stored
/// override says about a setting the operator has not touched, and what a
/// comparison against another profile is testing for. A [`Mode::default_profile`]
/// fills every field.
///
/// A field with no opinion is left out of `modeprofiles.json` rather than
/// written as `null`, so the file shows only what the operator changed.
/// That makes this a JSON-only type: postcard numbers fields by position, and
/// a skipped one would desynchronise every field after it, so it must not be
/// put on the wire as it stands.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agc: Option<AgcMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agc_max_gain_db: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_gain_db: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub squelch_db: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_reduction: Option<NrLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_notch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wfm_stereo: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fm_decrackle: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fm_deemph: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fm_decrackle_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binaural: Option<bool>,
    // The NR menu's filters, appended after the rest: HUM and the SK
    // post-filter with its settings, remembered per mode like the NR engine
    // and FM-DECRACKLE beside them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hum_filter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_tau_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_open_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_close_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_width: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_after_nr: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sk_filter_boost_db: Option<f32>,
}

/// Float equality by representation, for [`ModeProfile::trim_against`].
///
/// These are values that have been round-tripped through a slider, so "the
/// operator put it back" means the same `f32`, not the same number to within a
/// tolerance. Comparing bits keeps the linter quiet about `==` on floats.
fn same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

impl ModeProfile {
    /// True when this profile has no opinion about anything.
    pub fn is_empty(&self) -> bool {
        self.agc.is_none()
            && self.agc_max_gain_db.is_none()
            && self.manual_gain_db.is_none()
            && self.squelch_db.is_none()
            && self.noise_reduction.is_none()
            && self.auto_notch.is_none()
            && self.wfm_stereo.is_none()
            && self.fm_decrackle.is_none()
            && self.fm_deemph.is_none()
            && self.fm_decrackle_dip.is_none()
            && self.binaural.is_none()
            && self.hum_filter.is_none()
            && self.sk_filter.is_none()
            && self.sk_filter_tau_s.is_none()
            && self.sk_filter_open_s.is_none()
            && self.sk_filter_close_s.is_none()
            && self.sk_filter_width.is_none()
            && self.sk_filter_after_nr.is_none()
            && self.sk_filter_boost_db.is_none()
    }

    /// This profile with `base` filling in every field it does not speak to.
    pub fn over(self, base: ModeProfile) -> ModeProfile {
        ModeProfile {
            agc: self.agc.or(base.agc),
            agc_max_gain_db: self.agc_max_gain_db.or(base.agc_max_gain_db),
            manual_gain_db: self.manual_gain_db.or(base.manual_gain_db),
            squelch_db: self.squelch_db.or(base.squelch_db),
            noise_reduction: self.noise_reduction.or(base.noise_reduction),
            auto_notch: self.auto_notch.or(base.auto_notch),
            wfm_stereo: self.wfm_stereo.or(base.wfm_stereo),
            fm_decrackle: self.fm_decrackle.or(base.fm_decrackle),
            fm_deemph: self.fm_deemph.or(base.fm_deemph),
            fm_decrackle_dip: self.fm_decrackle_dip.or(base.fm_decrackle_dip),
            binaural: self.binaural.or(base.binaural),
            hum_filter: self.hum_filter.or(base.hum_filter),
            sk_filter: self.sk_filter.or(base.sk_filter),
            sk_filter_tau_s: self.sk_filter_tau_s.or(base.sk_filter_tau_s),
            sk_filter_open_s: self.sk_filter_open_s.or(base.sk_filter_open_s),
            sk_filter_close_s: self.sk_filter_close_s.or(base.sk_filter_close_s),
            sk_filter_width: self.sk_filter_width.or(base.sk_filter_width),
            sk_filter_after_nr: self.sk_filter_after_nr.or(base.sk_filter_after_nr),
            sk_filter_boost_db: self.sk_filter_boost_db.or(base.sk_filter_boost_db),
        }
    }

    /// Drop every field that is already what the mode starts with.
    ///
    /// A setting put back to the default is not a preference any more, so it is
    /// not remembered as one — which is also what makes a profile with nothing
    /// left in it removable.
    pub fn trim_against(&mut self, default: &ModeProfile) {
        if self.agc == default.agc {
            self.agc = None;
        }
        if self.agc_max_gain_db.zip(default.agc_max_gain_db).is_some_and(|(a, b)| same(a, b)) {
            self.agc_max_gain_db = None;
        }
        if self.manual_gain_db.zip(default.manual_gain_db).is_some_and(|(a, b)| same(a, b)) {
            self.manual_gain_db = None;
        }
        if self.squelch_db.zip(default.squelch_db).is_some_and(|(a, b)| same(a, b)) {
            self.squelch_db = None;
        }
        if self.noise_reduction == default.noise_reduction {
            self.noise_reduction = None;
        }
        if self.auto_notch == default.auto_notch {
            self.auto_notch = None;
        }
        if self.wfm_stereo == default.wfm_stereo {
            self.wfm_stereo = None;
        }
        if self.fm_decrackle == default.fm_decrackle {
            self.fm_decrackle = None;
        }
        if self.fm_deemph == default.fm_deemph {
            self.fm_deemph = None;
        }
        if self.fm_decrackle_dip == default.fm_decrackle_dip {
            self.fm_decrackle_dip = None;
        }
        if self.binaural == default.binaural {
            self.binaural = None;
        }
        if self.hum_filter == default.hum_filter {
            self.hum_filter = None;
        }
        if self.sk_filter == default.sk_filter {
            self.sk_filter = None;
        }
        if self.sk_filter_tau_s.zip(default.sk_filter_tau_s).is_some_and(|(a, b)| same(a, b)) {
            self.sk_filter_tau_s = None;
        }
        if self.sk_filter_open_s.zip(default.sk_filter_open_s).is_some_and(|(a, b)| same(a, b)) {
            self.sk_filter_open_s = None;
        }
        if self.sk_filter_close_s.zip(default.sk_filter_close_s).is_some_and(|(a, b)| same(a, b)) {
            self.sk_filter_close_s = None;
        }
        if self.sk_filter_width == default.sk_filter_width {
            self.sk_filter_width = None;
        }
        if self.sk_filter_after_nr == default.sk_filter_after_nr {
            self.sk_filter_after_nr = None;
        }
        if self.sk_filter_boost_db.zip(default.sk_filter_boost_db).is_some_and(|(a, b)| same(a, b))
        {
            self.sk_filter_boost_db = None;
        }
    }

    /// Whether the live settings of `rx` all match this profile.
    ///
    /// A profile with no opinion matches anything — there is nothing in it to
    /// disagree with.
    pub fn agrees_with(&self, rx: &crate::RxState) -> bool {
        self.agc.is_none_or(|v| v == rx.agc)
            && self.agc_max_gain_db.is_none_or(|v| same(v, rx.agc_max_gain_db))
            && self.manual_gain_db.is_none_or(|v| same(v, rx.manual_gain_db))
            && self.squelch_db.is_none_or(|v| same(v, rx.squelch_db))
            && self.noise_reduction.is_none_or(|v| v == rx.noise_reduction)
            && self.auto_notch.is_none_or(|v| v == rx.auto_notch)
            && self.wfm_stereo.is_none_or(|v| v == rx.wfm_stereo)
            && self.fm_decrackle.is_none_or(|v| v == rx.fm_decrackle)
            && self.fm_deemph.is_none_or(|v| v == rx.fm_deemph)
            && self.fm_decrackle_dip.is_none_or(|v| v == rx.fm_decrackle_dip)
            && self.binaural.is_none_or(|v| v == rx.binaural)
            && self.hum_filter.is_none_or(|v| v == rx.hum_filter)
            && self.sk_filter.is_none_or(|v| v == rx.sk_filter)
            && self.sk_filter_tau_s.is_none_or(|v| same(v, rx.sk_filter_tau_s))
            && self.sk_filter_open_s.is_none_or(|v| same(v, rx.sk_filter_open_s))
            && self.sk_filter_close_s.is_none_or(|v| same(v, rx.sk_filter_close_s))
            && self.sk_filter_width.is_none_or(|v| v == rx.sk_filter_width)
            && self.sk_filter_after_nr.is_none_or(|v| v == rx.sk_filter_after_nr)
            && self.sk_filter_boost_db.is_none_or(|v| same(v, rx.sk_filter_boost_db))
    }

    /// Write every field this profile has an opinion about into `rx`.
    ///
    /// An effective profile (defaults overlaid with overrides) speaks to all of
    /// them; a partial one leaves the rest alone, which is what makes this the
    /// same operation as applying a change.
    pub fn apply_to(&self, rx: &mut crate::RxState) {
        if let Some(v) = self.agc {
            rx.agc = v;
        }
        if let Some(v) = self.agc_max_gain_db {
            rx.agc_max_gain_db = v;
        }
        if let Some(v) = self.manual_gain_db {
            rx.manual_gain_db = v;
        }
        if let Some(v) = self.squelch_db {
            rx.squelch_db = v;
        }
        if let Some(v) = self.noise_reduction {
            rx.noise_reduction = v;
        }
        if let Some(v) = self.auto_notch {
            rx.auto_notch = v;
        }
        if let Some(v) = self.wfm_stereo {
            rx.wfm_stereo = v;
        }
        if let Some(v) = self.fm_decrackle {
            rx.fm_decrackle = v;
        }
        if let Some(v) = self.fm_deemph {
            rx.fm_deemph = v;
        }
        if let Some(v) = self.fm_decrackle_dip {
            rx.fm_decrackle_dip = v;
        }
        if let Some(v) = self.binaural {
            rx.binaural = v;
        }
        if let Some(v) = self.hum_filter {
            rx.hum_filter = v;
        }
        if let Some(v) = self.sk_filter {
            rx.sk_filter = v;
        }
        if let Some(v) = self.sk_filter_tau_s {
            rx.sk_filter_tau_s = v;
        }
        if let Some(v) = self.sk_filter_open_s {
            rx.sk_filter_open_s = v;
        }
        if let Some(v) = self.sk_filter_close_s {
            rx.sk_filter_close_s = v;
        }
        if let Some(v) = self.sk_filter_width {
            rx.sk_filter_width = v;
        }
        if let Some(v) = self.sk_filter_after_nr {
            rx.sk_filter_after_nr = v;
        }
        if let Some(v) = self.sk_filter_boost_db {
            rx.sk_filter_boost_db = v;
        }
    }
}

/// Every mode's remembered overrides, as `modeprofiles.json`.
///
/// The map is keyed by the mode's serialized name (`"Ft8"`), and unknown keys
/// are kept rather than rejected: a file written by a build that knows a mode
/// this one does not survives a round trip through this one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeProfiles {
    pub modes: BTreeMap<String, ModeProfile>,
}

impl ModeProfiles {
    /// The key a mode is stored under, from its own serde name so the file and
    /// the wire agree.
    fn key(mode: Mode) -> String {
        serde_json::to_value(mode)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    /// The remembered overrides for `mode`, if any.
    pub fn overrides(&self, mode: Mode) -> Option<ModeProfile> {
        self.modes.get(&Self::key(mode)).copied()
    }

    /// Remember `profile` for `mode`, or forget the mode when there is nothing
    /// left in it.
    pub fn set(&mut self, mode: Mode, profile: ModeProfile) {
        let key = Self::key(mode);
        if profile.is_empty() {
            self.modes.remove(&key);
        } else {
            self.modes.insert(key, profile);
        }
    }

    /// Forget one mode's overrides.
    pub fn clear(&mut self, mode: Mode) {
        self.modes.remove(&Self::key(mode));
    }

    /// Forget every mode's overrides.
    pub fn clear_all(&mut self) {
        self.modes.clear();
    }

    /// The defaults for `mode` with this station's overrides laid over them.
    pub fn effective(&self, mode: Mode) -> ModeProfile {
        self.overrides(mode).unwrap_or_default().over(mode.default_profile())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_laid_over_the_defaults_keeps_only_its_own_fields() {
        let base =
            ModeProfile { agc: Some(AgcMode::Med), binaural: Some(false), ..Default::default() };
        let over = ModeProfile { agc: Some(AgcMode::Slow), ..Default::default() };
        let effective = over.over(base);
        assert_eq!(effective.agc, Some(AgcMode::Slow));
        assert_eq!(effective.binaural, Some(false));
    }

    #[test]
    fn putting_a_setting_back_to_the_default_forgets_it() {
        let default = Mode::Ft8.default_profile();
        // A departure is kept...
        let mut over = default;
        over.noise_reduction = Some(NrLevel::High);
        over.trim_against(&default);
        assert_eq!(over.noise_reduction, Some(NrLevel::High));
        assert!(!over.is_empty());
        // ...and put back to what the mode starts with, it is forgotten.
        let mut back = over;
        back.noise_reduction = default.noise_reduction;
        back.trim_against(&default);
        assert!(back.is_empty());
    }

    #[test]
    fn every_mode_speaks_to_every_setting() {
        for mode in Mode::ALL {
            assert!(!mode.default_profile().is_empty(), "{mode:?} has no defaults");
        }
    }

    #[test]
    fn stored_overrides_are_laid_over_the_mode_defaults() {
        let mut profiles = ModeProfiles::default();
        assert_eq!(profiles.effective(Mode::Usb), Mode::Usb.default_profile());
        profiles.set(
            Mode::Usb,
            ModeProfile { noise_reduction: Some(NrLevel::Off), ..Default::default() },
        );
        let effective = profiles.effective(Mode::Usb);
        assert_eq!(effective.noise_reduction, Some(NrLevel::Off));
        // The defaults still answer for everything the override is silent on.
        assert_eq!(effective.agc, Mode::Usb.default_profile().agc);
        profiles.set(Mode::Usb, ModeProfile::default());
        assert_eq!(profiles.overrides(Mode::Usb), None, "an empty override is forgotten");
    }

    #[test]
    fn the_file_holds_only_what_was_changed() {
        let mut profiles = ModeProfiles::default();
        profiles.set(Mode::Usb, ModeProfile { auto_notch: Some(true), ..Default::default() });
        let json = serde_json::to_string(&profiles).unwrap();
        assert_eq!(json, r#"{"modes":{"Usb":{"auto_notch":true}}}"#);
        let back: ModeProfiles = serde_json::from_str(&json).unwrap();
        assert_eq!(back, profiles);
    }

    #[test]
    fn a_file_from_a_build_that_knows_other_modes_round_trips() {
        let mut profiles = ModeProfiles::default();
        profiles.modes.insert("Holographic".into(), ModeProfile::default());
        profiles.set(Mode::Ft8, ModeProfile { auto_notch: Some(true), ..Default::default() });
        let json = serde_json::to_string(&profiles).unwrap();
        let back: ModeProfiles = serde_json::from_str(&json).unwrap();
        assert_eq!(back, profiles);
        assert_eq!(back.overrides(Mode::Ft8).unwrap().auto_notch, Some(true));
    }
}
