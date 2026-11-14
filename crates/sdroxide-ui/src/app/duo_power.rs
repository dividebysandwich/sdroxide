//! The ELAD FDM-DUO's power buttons beside Drive: 1W, 3W, 5W and MAX (AJ3G,
//! 18 Oct 2026). Each puts the DUO's menu 34 `TX POWER` on that setting, the
//! radio's own ceiling; Drive then sets the power within it. MAX is the
//! default and what the HR500 needs: at 5 W the DUO cannot drive the amplifier
//! to full output on most bands.

use crate::i18n::tr;

/// The buttons: label, menu 34 setting as `TP`'s parameter, and what the
/// radio's step is called.
pub(in crate::app) const PRESETS: [(&str, u8); 4] = [("1W", 2), ("3W", 6), ("5W", 8), ("MAX", 9)];

/// The hover for the button that sets `code`.
pub(in crate::app) fn hover(code: u8) -> String {
    let base = tr!("duopower-sets-fdm-duo-s-own");
    match code {
        9 => tr!("duopower-max-fdm-duo-s-full", base = base),
        8 => tr!("duopower-t-5-w-hr500-line-will", base = base),
        _ => tr!(
            "duopower-w-running-barefoot-qrp",
            code = format!("{:.0}", step_w(code)),
            base = base
        ),
    }
}

/// Hearing your own signal, on the front panel (AJ3G, 13 Nov 2026). MIC A,
/// right of REC: hear an over keyed at the radio itself (its microphone PTT),
/// the inverse of `CatConfig::elad_mute_on_radio_ptt`.
pub(in crate::app) const MIC_CHIP: &str = "MIC A";

/// DIG A, under the 3W and 5W buttons: hear an over sdroxide keyed (the
/// digital modes, TUNE, PTT), `EladConfig::hear_own_tx`.
pub(in crate::app) const DIGI_CHIP: &str = "DIG A";

/// The hover for MIC A (0) or DIG A (1).
pub(in crate::app) fn monitor_hover(i: usize, lit: bool) -> String {
    let state = if lit { "On: you hear it." } else { "Off: the speakers are quiet for it." };
    match i {
        0 => format!(
            "Hear your own signal when you key the FDM-DUO at the radio: its microphone \
             PTT, a foot switch or another program. SDRoxide asks the radio over CAT \
             whether it is transmitting. Takes effect at once.\n\n{state}"
        ),
        _ => format!(
            "Hear your own signal when SDRoxide transmits: the digital modes, TUNE and PTT. \
             The receiver is read right through the over. Takes effect from the next \
             over.\n\n{state}"
        ),
    }
}

/// Menu 34's numbered steps, in watts, by `TP` parameter — the radio's own
/// labels (`sdroxide_cat::elad::TX_POWER_STEPS_W`, repeated here because the
/// UI does not depend on the CAT crate).
fn step_w(code: u8) -> f32 {
    const STEPS: [f32; 9] = [0.3, 0.5, 1.0, 1.2, 1.5, 2.0, 3.0, 4.0, 5.0];
    STEPS.get(code as usize).copied().unwrap_or(5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_are_menu_34s_own_steps() {
        let watts: Vec<f32> = PRESETS.iter().filter(|p| p.1 < 9).map(|p| step_w(p.1)).collect();
        assert_eq!(watts, vec![1.0, 3.0, 5.0]);
        assert_eq!(PRESETS[3], ("MAX", 9));
        assert!(hover(8).contains("HR500"));
    }

    #[test]
    fn the_monitor_chips_say_which_over_they_are_for() {
        assert!(monitor_hover(0, true).contains("microphone"));
        assert!(monitor_hover(1, false).contains("digital modes"));
        assert!(monitor_hover(1, false).contains("Off"));
    }
}
