# Hardrock amplifier: design decisions

Places where the panel deliberately does something other than copy the amplifier, and why. Before changing any of these, read the reason.

## 1. A yellow the amplifier shows before your first transmission is shown green

**Decided:** AJ3G, 18 Oct 2026.

**What the panel does.** Until SDRoxide has seen a transmission on the current connection to the amplifier, any indicator the amplifier reports as yellow (FWD, RFL, DRV, VDD or IDD) is shown green. A red is always shown. From the first key-up on, the panel shows the amplifier's lights exactly as it sends them.

**What the amplifier does.** The HR500 reports its five indicator lights in its status reply. A light it turns yellow stays yellow until the next key-up. It can also start up with DRV lit yellow when nothing is wrong. AJ3G saw this on 18 Oct 2026, after power-cycling the amplifier while sorting out the ELAD's menu 34: the amplifier's own screen and the panel both opened with DRV yellow, and both cleared at the first key-up.

**Why the panel doesn't copy it.**

- A yellow at start-up is a quirk of the amplifier's firmware, not a warning about anything happening now. There is no RF and no drive, so there is nothing for DRV to warn about.
- The panel is used from remote, where the operator can't see the amplifier or hear its fan. A yellow light there reads as "something is wrong with the amplifier". The operator might stop operating, or start fault-finding, when nothing is wrong.
- A warning that is often false trains people to ignore the colour. Yellow and red must only appear when they mean something.

**Why it is safe.**

- Only yellows are hidden, and only before the first transmission. A yellow is a caution; it never stops the amplifier.
- A red means the amplifier tripped and went off line. That is always shown, whenever it happened, so an operator connecting after a trip still sees it.
- From the first key-up the amplifier clears its own lights, and every warning during a transmission is shown as sent and held, as before.
- The live meters and readings are not affected; only the indicator colour is.

**Where.** `crates/sdroxide-hardrock/src/link.rs`, `lights_seen()`. Tested by `a_yellow_held_from_before_the_link_opened_shows_green`. Described in the user manual, section 2.23, "Readings".

**When it resets.** A new connection to the amplifier starts over: after SDRoxide starts, after the amplifier's USB lead is plugged back in, or after the amplifier is powered back on and reconnects.

## 2. In OFF (bypass), FWD, RFL and SWR come from the radio

**Decided:** AJ3G, 18 Oct 2026.

**What the panel does.** Only while the amplifier is in OFF, and only while the radio transmits:

- SWR is the radio's own reading over CAT, where it gives one.
- FWD is the watts the radio measures, where it reports watts. Otherwise it is the amplifier's DRV reading.
- RFL is the reflected watts the radio measures, where it reports them. Otherwise it is worked out as FWD × |Γ|², where |Γ| = (SWR − 1) / (SWR + 1).

The meter legend and the tiles' hover text say where each came from. Any reading the amplifier does give in OFF is kept; only a zero is filled. In PTT nothing changes: the amplifier's own bridge is used.

**What the amplifier does.** In OFF the HR500 reports FWD 0, RFL 0 and no SWR, whatever is going through it. AJ3G checked on 18 Oct 2026: 5 W from the ELAD in OFF showed nothing, and ANT 1 at 3:1 SWR showed nothing either, so it is not a low-power cut-off. The bridge on the LPF board (Rev D, sheet 2) is in the bypass path, so this is the firmware choosing not to report.

**Why.** Running barefoot through the bypassed amplifier, with or without the tuner, is a normal way to operate, and from remote the panel is the only meter the operator has. Showing nothing reads as no output. SDRoxide already shows the radio's own power and SWR for any rig that reports them, so the panel uses the same readings.

**Why DRV when the radio gives no watts.** DRV is the amplifier measuring the radio's power at its input. In bypass that power passes straight through to the tuner, so it is the forward power, less a small loss in the relays and filter. Some radios report only a needle position on their own power meter; that is not turned into watts, since it would invent a calibration the radio never gave.

**The ELAD FDM-DUO.** It reports forward power (`FP`), reflected power (`RP`) and SWR (`WR`). ELAD's documents give `FP` and `RP` no unit, so SDRoxide's original author left them out. AJ3G measured them on 18 Oct 2026 with the DUO's TUNE carrier: into a dummy load `FP 006091;RP 000001;WR0 01.02;` at about 6 W, and into an open `FP 008291;RP 007525;WR0 33.06;`. They are milliwatts, and the rig's own SWR is exactly what the two give. Each carries a flag: a space for a valid reading, `!` for one not to trust (receive, below 0.5 W, and while TUNE ramps up and down). Flagged readings are not used. So with a DUO, FWD, RFL and SWR all come from the radio.

**SDRoxide's main TX meter.** It used to show the radio's forward power where the radio had one, and the amplifier's otherwise. With the DUO's `FP` read, that would have shown the DUO's 6 W while the amplifier was putting 500 W on the air. So while the amplifier is in PTT the main meter now shows the amplifier's power and SWR; the radio's are shown otherwise. SDRoxide's SWR protection is unchanged: it still reads the radio's SWR.

**A radio that measures its own watts never borrows DRV.** With the FDM-DUO and its Rig control port, when the DUO flags its forward reading (`!`), FWD and RFL are left empty and the legend says "the rig flagged its reading", rather than showing DRV. AJ3G keyed into an open on 18 Oct 2026: a known-good LP-100A digital vector wattmeter showed the DUO folding back to 2.94 W, while DRV read almost 10 W and the DUO's own screen 8.7 W. DRV over-reads into a mismatch, which is exactly when the DUO stops vouching for its own reading. Into a dead open no bridge here is to be trusted; the 3:1 test is the meaningful one. A radio that never reports watts still gets DRV, as above.

**Limits.** The radio's readings are measured at the radio, so they include the cable to the amplifier and the bypass path as well as the tuner and antenna.

**Where.** `crates/sdroxide-ui/src/app/amp.rs`, `from_rig()`; the DUO's readings in `crates/sdroxide-cat/src/elad.rs`, `parse_milliwatts()`; the main TX meter in `crates/sdroxide-radio/src/engine.rs`, `amp_tx_meter()`. Tests: `in_bypass_fwd_rfl_and_swr_come_from_the_rig`, `forward_and_reflected_power_are_milliwatts`. Tested by `in_bypass_fwd_rfl_and_swr_come_from_the_rig`. User manual section 2.23, "Running barefoot in bypass".

## 3. The FDM-DUO's 1W / 3W / 5W / MAX buttons set menu 34

**Decided:** AJ3G, 18 Oct 2026.

**What it does.** With an ELAD FDM-DUO that can transmit and has its Rig control (CAT) port set, the TX box shows 1W, 3W, 5W and MAX beside the Drive slider, captioned "ELAD FDM-DUO TX PWR". Each sets the DUO's menu 34 `TX POWER` (`TP02`, `TP06`, `TP08`, `TP09`) at once, and saves it as `elad_tx_power` in the radio settings, so it is sent again at every connect in place of the MAX that was always sent before. Until one is picked it is MAX. Drive and Tune set the power within it, through the audio (decision 4); they never write menu 34.

**Why menu 34 and not Drive.** AJ3G wanted the buttons to set the radio's own power setting. It is a ceiling the radio holds whatever the audio does: at 5 W the DUO cannot exceed about 6 W, which matters for an amplifier with an 8 W drive limit.

**Why MAX is the default.** With the HR500 in line the DUO needs MAX: at 5 W it cannot drive the amplifier to full output on most bands (AJ3G). The power is brought down with Drive. 1W, 3W and 5W are for running barefoot or QRP.

**What the steps give.** Measured with a known-good LP-100A digital vector wattmeter, 20 m, dummy load: 1 W gave 1.44 W on TUNE and 1.41 W on FT8; 3 W gave 3.77 W on TUNE and 3.67 W on FT8 at Drive 100%; 5 W gave 5.9 to 6.1 W; MAX about 8 W. Each step lets a little more out than it says.

**Why beside Drive, not in Settings or the AMP panel.** The FDM-DUO and the FDM-S2 share one settings page, and a power setting there would confuse S2 users (AJ3G). The AMP panel is only there with an amplifier configured, and DUO users without one need it too. Beside Drive it is where power is set anyway, and it is shown only for a DUO with a Rig control port (AJ3G: "only when ELAD TX is enabled in CAT/Rig Control"). Every other radio's TX box is unchanged.

**Where.** `crates/sdroxide-cat/src/elad.rs`, `tx_power_frame()`, `Elad::new` and `set_power_step()` (test `menu_34_is_the_operators_choice_at_connect_and_from_the_buttons`); `Command::SetRigPowerStep`; `CatConfig::elad_tx_power`; the buttons in `crates/sdroxide-ui/src/app/top_bar.rs`, `tx_duo_presets()` and `duo_presets_apply()`, and `crates/sdroxide-ui/src/app/duo_power.rs`. User manual section 2.23, "FDM-DUO power buttons".

## 4. On the FDM-DUO, Drive 100% sends a quarter of full audio

**Decided:** AJ3G, 18 Oct 2026, from his measurements.

**What was wrong.** AJ3G measured the DUO on 20 m with a known-good LP-100A digital vector wattmeter into a dummy load, menu 34 at MAX, using SDRoxide's TUNE: 0.94 W at Drive 1%, 2.08 W at 5%, 3.72 W at 10%, 5.39 W at 15%, 6.85 W at 20%, 7.36 W at 22%, 7.73 W at 23%, then 8.0 to 8.3 W everywhere from 24% to 100%. FT8 gave the same (5.26 W at 15%, 7.9 to 8.1 W from 25% up). So three quarters of the Drive slider did nothing, and a preset worked out on a straight line would have put 8 W into the HR500 when 5 W was asked for. The HR500's own DRV reading agreed with the LP-100A.

**What it does now.** For the DUO, Drive and Tune are scaled into the bottom 24% of the old range (`DUO_AUDIO_POWER_CEILING`, applied by `CatHandle::audio_gain_for`). Drive 100% is about 8 W and the output is a straight line from about 0.5 W at the bottom, so every point of the slider does something.

**Why voice is no different.** A voice signal's average is far below its peaks, but its peaks are as loud as the tune tone, and peaks are what the DUO's output stage and the HR500's drive limit see. Digital modes are a steady tone like TUNE, which FT8 confirmed.

**Why 24% holds at any power setting.** With menu 34 at 5 W the knee was in the same place (3.72 W at 15%, 5.9 W at 25%, 6.0 W at 75%), so it is set by the audio into the DUO, not by its power step. Drive therefore spans whatever ceiling menu 34 sets.

**Limits.** Measured on one DUO, on 20 m, at its audio input setting as it is. A DUO set up with a different audio input level would have its knee elsewhere. Not applied on a DUO connected over USB alone with no serial cable: there SDRoxide scales the tune tone itself, and that path has not been measured.

**Where.** `crates/sdroxide-cat/src/elad.rs`, `DUO_AUDIO_POWER_CEILING`; `crates/sdroxide-cat/src/lib.rs`, `audio_gain_for`; User manual section 2.23, "Drive on the FDM-DUO".

## 5. At or past the tune limit, the SWR tile is red and the radio's watts are marked unreliable

**Decided:** AJ3G, 18 Oct 2026, after bench testing.

**Deviation.** The HR500 has no SWR indicator, so the panel's SWR tile had no colour, to match. It now turns red when the SWR is at or past SDRoxide's tune limit (twice the SWR limit in Settings → General, 5.0:1 by default).

**Why.** Into an open circuit on 20 m, with the amp in OFF, the DUO reported 7.75 W forward and 6 to 7 W reflected, so the panel showed FWD 8 W and RFL 6–7 W "from the rig". AJ3G's known-good LP-100A digital vector wattmeter read 2.62 W forward and 2.93 W reflected: the DUO was folding back and its own watts read about three times high. The SWR it reported, 22.5:1, was the honest figure. So at a very high SWR the panel points the operator at the SWR, in red, and warns across the top of each meter showing the radio's FWD or RFL, centred in large red letters ("FWD UNRELIABLE AT THIS SWR"), with the reason on hover. AJ3G first saw it as a legend note and asked for it at the top, red and larger, because a legend is too easy to miss. Red is a warning, in line with the rule that yellow and red mean warnings only.

**Limits.** It does not stop anything; SDRoxide's own SWR guard already stops TUNE and transmit at its limits. The threshold is the tune limit so there is one figure for the operator to find in the settings. (On an HR500 with its tuner it is now 6.1:1 at the latest: see decision 6.)

**Where.** `crates/sdroxide-ui/src/app/amp.rs`, `swr_past` and `readings_row`; User manual section 2.23, "Readings".

## 6. The tuner panel follows how the ATU-500 really searches

**Decided:** AJ3G, 14 Nov 2026, from Jim WA2EUJ's (HobbyPCB) written explanation of the tuner's algorithm.

**What changed it.** The panel's tuner handling and its notes had been worked out from watching the tuner, and assumed things it doesn't do: for example, that it stops at the first match that is "good enough", and that a successful tune leaves the tuner's network in line. WA2EUJ explained by email how the search actually works (see "How the ATU-500 tunes" in [atu-values.md](atu-values.md)): it aims for 1.3:1 and always keeps the best setting it has seen; it leaves a load already better than 1.3:1 bypassed; and it doesn't try at all above about 6:1. His explanation corrected those assumptions, and the panel now says what the tuner did.

**What the panel does now.**

- A tune that ends "Tuned" with the tuner ON but its network bypassed reads, in green, "Tuned: under 1.3:1 without the tuner", with a hover saying this is expected. Before, it showed only the bypass drawing, which looked as if nothing had happened.
- A failed tune (TUNE red) shows a short red line under TUNE saying what to do, with the reason on hover and on TUNE: above about 6:1 the tuner stays bypassed, so check the antenna and feed line; Lo power means under 0.5 W reached it, so keep a steady carrier of a few watts; Failed (above 2:1) means try again or check the antenna; Power too high means lower the drive. Before, a failure with the tuner bypassed showed nothing but the red TUNE.
- The tuner's SWR, beside its state, says on hover that it comes from the tuner's own coupler, while the SWR tile comes from the low-pass filter board's coupler, so they can differ by up to about 0.2.
- The TUNE hover says the tuner aims for 1.3:1 and keeps the best match it has seen.
- On an HR500 with its tuner, the SWR tile (decision 5) is red from 6.1:1 at the latest, rather than at the tune limit (up to 10:1). 6.1:1 is just above the tuner's 6:1, so the tuner's own refusal at 6:1 comes first and the tile isn't red over a load the tuner would still try (AJ3G). It is never below the operator's SWR limit itself.

**Limits.** Display only: nothing changes what or when the amplifier transmits or switches. SDRoxide's own SWR guard (`swr_tune_limit`, used by the engine) is unchanged, because it serves every radio and external tuner, some of which start their search at 10:1.

**Where.** `crates/sdroxide-ui/src/app/amp.rs`, `swr_warn_at`, `ATU_500_SWR_RED`, `TuneFailure`, `tuned_without_tuner` and the tuner section of the window; `docs/hardrock/atu-values.md`, "How the ATU-500 tunes"; User manual section 2.23, "Readings" and "Tuner".

## 7. The tuner's starting SWR, its T result, Last tunes and a read-only pass-through

**Decided:** AJ3G, 16 Nov 2026, from WA2EUJ's answer about the ATU-500's status reply and result letters (AJ3G to vet, 16 Nov 2026).

**What changed it.** In the tuner's `P` reply, `S` is the starting (untuned) return loss and `F` the final one, both in tenths of a dB, and `B` is 1 when the network is bypassed because the match was already good enough (see "The tuner's status reply and results" in [atu-values.md](atu-values.md)). In the result, `T` means the untuned load was better than 1.3:1 and was left bypassed, and `S` a completed tune. SDRoxide had dropped `S`, read `B` as the band and taken `T` and `S` alike as "Tuned".

**What the panel does now.**

- `AtuParams` reads `B` as bypassed and `S` as the starting return loss (`start_swr`); `TuneResult::Bypassed` comes from `T`. Reading `B` makes decision 6's *Tuned: under 1.3:1 without the tuner* work on a real tuner, which never sends the `Y` field the panel had looked for.
- After a tune the tuner's line shows where it started from, as in *1.46:1 (from 3.44:1)*; after a bypass, the load's own SWR. Both come from the tuner's own coupler.
- When a tune ends the link keeps a `TuneRecord` (time, band, result, network with both SWRs), the last four, newest first, and logs a line for it. The AMP window lists them under **LAST TUNES**. With the capacitor at step 0 no side is named for it (AJ3G).
- The tuner's state line has a hover for every state, saying how the tuner tunes and whose SWR it shows.
- Whether the last tune succeeded is judged on the link's own words, not on the translated "Tuned", which never matched in other languages.
- Every tuner pass-through that isn't a read is refused before it is sent: safety interlock #5. He warned that switching the tuner's coupler in (`D1`) at full power can damage it, and other commands wipe or rewrite the tuner.
- The wire protocol goes to v212: `AtuNetwork` and `AtuReadout` carry the new fields.

**Not ported.** Hardrock Panel also raised TUNE's drive limit to 10 W on the HR500 (`tune_max_w`). SDRoxide has nothing to raise: it never judges TUNE against the amplifier's drive limit. The 8 W limit there is OFF to PTT (#4) and the DRV colours only, and both stay.

**Where.** `crates/sdroxide-hardrock/src/protocol.rs` (`AtuParams`, `TuneResult`, `passthrough_allowed`), `lib.rs` (`Amp::send`), `link.rs` (`tune_log_line`, the tune's end); `crates/sdroxide-types/src/amp.rs` (`TuneRecord`, `TUNE_HISTORY`, `TUNE_RESULT_*`); `crates/sdroxide-ui/src/app/amp.rs` (`tuner_swr_words`, `tune_row`, `tune_time`, `tune_succeeded`, the tuner section); User manual section 2.23, "Tuner".
