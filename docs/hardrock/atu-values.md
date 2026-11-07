# ATU-500 inductance and capacitance in the tuner window

The amplifier reports its tuner setting only as two step numbers: an inductor step (0 to 63) and a capacitor step (0 to 127), plus which side the capacitor is on. It doesn't say how many µH or pF those steps are. This note records how the values the AMP window shows were arrived at, so they can be checked or improved later.

## What the window shows

For the HR500 (code: `AmpModel::atu_scale` in `crates/sdroxide-types/src/amp.rs`):

- Inductance = 0.05 µH + step × 0.05 µH
- Capacitance = 10 pF + the sum of the capacitor bits that are on, worth 11, 23.5, 50, 110, 235, 500 and 1100 pF (bit 0 first)

Until 18 Oct 2026 the capacitance was 50 pF + step × 10 pF. It was changed when the tuner's firmware showed that the capacitor bits drive the relays directly (see "What the ATU-500 firmware shows" below).

The window shows each value with a "~" in front, to mark it as approximate, and the step number alongside, for example "~0.20 µH · step 3". A step of 0 is shown as "no inductance needed" or "no capacitance needed", because 0 is what the tuner chose, not a missing value.

These are estimates, judged good to about 10% (AJ3G, 17 Oct 2026). They come from the sources, the tuner's firmware and the bench tests below, not from measuring the board on its own. The floor (50 nH, 10 pF) was refitted after the firmware reading.

## Changes

| Date | Inductance | Capacitance | Why |
|---|---|---|---|
| 17 Oct 2026 | 0.05 µH + step × 0.05 µH | 50 pF + step × 10 pF | First bench fit (two 20 m tests). |
| 18 Oct 2026 | unchanged | 10 pF + the sum of the bits that are on (11, 23.5, 50, 110, 235, 500, 1100 pF) | The ATU-500 firmware showed that each step bit drives one relay. The capacitor bits are series pairs that aren't binary, so a straight 10 pF a step was wrong at high steps (step 64: about 690 pF shown, about 1110 pF real). The floor was refitted to 10 pF. The 20 m bench tune still reads 170 pF. |

## Sources

1. **HobbyPCB's HR500 Control Panel.** It multiplies the steps by 0.05 µH and 10 pF, with nothing added. This is the "linear" reading.
2. **The ATU-500 user manual's specification table.** It gives 63 inductor steps from 50 nH to 3.2 µH, and 127 capacitor steps from 22 pF to 2800 pF.
3. **The ATU-500 schematic (sheet 1, Rev B).** The part values and the T130 core mixes and turns (counted on AJ3G's board) give the per-relay values below. Summing these for each step's relay bits is the "per-relay" reading. The firmware later confirmed that the bits drive these relays directly. For the capacitors, this table is now what the window uses. For the inductors, it isn't: the counted turns don't add up to the manual's 3.2 µH (see below).

| Inductor bit | Relay | Core, turns | Nominal (AL × N²) |
|---|---|---|---|
| L1 | Y9 | T130-0, 2 T | 6 nH |
| L2 | Y10 | T130-6, 2 T | 38 nH |
| L3 | Y11 | T130-6, 5 T | 240 nH |
| L4 | Y12 | T130-2, 7 T | 539 nH |
| L5 | Y13 | T130-2, 11 T | 1331 nH |
| L6 | Y14 | T130-2, 16 T | 2816 nH |

| Capacitor bit | Parts (two in series) | Effective |
|---|---|---|
| C1 | 22 pF | 11 pF |
| C2 | 47 pF | 23.5 pF |
| C3 | 100 pF | 50 pF |
| C4 | 220 pF | 110 pF |
| C5 | 470 pF | 235 pF |
| C6 | 1000 pF | 500 pF |
| C7 | 2200 pF | 1100 pF |

4. **The ATU-500 firmware (V1.1F), added 18 Oct 2026.** It shows how a step number reaches the relays: one relay per bit, with no lookup table. See "What the ATU-500 firmware shows" below.

The first three sources disagree, so a bench test was used to choose between them. The firmware later settled how the steps map to relays.

## Bench test (17 Oct 2026, AJ3G)

### Setup

- Band and frequency: 20 m, 14.074 MHz.
- Amp: in OFF (bypass).
- Drive: about 5 W from the ELAD, through the ATU.
- Load: two 50 Ω loads in parallel on a T, connected by ordinary (not phase-stable) cable.

### Results

- **Tune:** the ATU tuned to L step 3, C step 12, capacitor on the input side, and reported 1.23:1.
- **Load measured with a VNA at the amp's antenna jack:** 30.5 − j0.88 Ω at 14.080 MHz. This includes the T and the cable, which is why it isn't the nominal 25 Ω.
- **Ideal L network for that load:** 286 nH in series and 181 pF on the input side.

### Predicted SWR into the measured load at step 3 / step 12

| Reading | L, C in circuit | Predicted SWR |
|---|---|---|
| Linear (Control Panel) | 150 nH, 120 pF | 1.41:1 |
| Per-relay (schematic) | 44 nH, 160 pF | 1.90:1 |
| Manual's range as 22 pF a step | 200 nH, 264 pF | 1.61:1 |
| **Linear plus a 50 nH / 22 pF floor** | **200 nH, 142 pF** | **1.27:1** |
| Linear plus an 80 nH / 22 pF floor | 230 nH, 142 pF | 1.20:1 |
| Per-relay plus a 50 nH / 22 pF floor | 94 nH, 182 pF | 1.74:1 |
| **Measured by the ATU** | | **1.23:1** |

To reach 1.23:1, the per-relay reading would need more than 170 nH of stray inductance, which isn't believable for this board. The linear reading needs between 60 and 180 nH of series stray and 15 to 30 pF of shunt stray, depending on the mix. Using the manual's own minimums (50 nH, 22 pF) as that floor lands within 0.04 of the measured SWR.

## Second test: same tune into 50 Ω (17 Oct 2026)

With the tuner still on L step 3 / C step 12, C IN, the load was changed to a single 50 Ω termination. The VNA at the amp's RF input (amp OFF) read 43.6 − j20.4 Ω at 14.090 MHz. This reading includes the amp's own path to the tuner, which couldn't be measured separately (short and open on the antenna jack weren't possible).

Predicted for step 3 / step 12 into 50 Ω, before any path:

| Reading | L, C in circuit | Predicted |
|---|---|---|
| Linear | 150 nH, 120 pF | 49.0 − j14.9 Ω |
| Linear plus 50 nH / 22 pF | 200 nH, 142 pF | 50.0 − j17.7 Ω |
| **Linear plus 50 nH / 50 pF** | **200 nH, 170 pF** | **within 2.5 Ω of the reading** |
| Per-relay | 44 nH, 160 pF | 35.9 − j22.8 Ω |
| **Measured** | | **43.6 − j20.4 Ω** |

Fitting the floor to both tests together (this reading, and the tuner's own 1.23:1 into the 30.5 Ω load) gives 50 to 60 nH and about 50 pF:

| Floor | Error against 43.6 − j20.4 Ω | SWR into 30.5 Ω |
|---|---|---|
| 50 nH, 50 pF | 2.5 Ω | 1.26:1 |
| 55 nH, 50 pF | 2.8 Ω | 1.25:1 |
| 60 nH, 52 pF | 3.1 Ω | 1.23:1 |
| 50 nH, 22 pF (first estimate) | 7.0 Ω | 1.27:1 |

At this stage the window used 50 nH and 50 pF (since replaced; see the refit below). The inductance floor agrees with the manual's minimum. The capacitance floor is higher than the manual's 22 pF, which fits the large stray capacitance expected from the relays and wiring. Some of it may be in the amp's own path and the test cable rather than in the tuner, so the capacitance is the less certain of the two. The few ohms left over are put down to that path. The per-relay reading stays the worst fit.

## Third point: 10 m (18 Oct 2026)

Same T load, 28.074 MHz, amp OFF: the ATU tuned to L step 3, C step 3, C IN, 1.13:1. The window showed ~0.20 µH and ~80 pF. The load wasn't measured at 28 MHz, so this is only a rough check. If it were still 30.5 Ω, the ideal would be 143 nH and 91 pF, and the predicted SWR at step 3 / step 3 would be:

| Reading | L, C | Predicted SWR |
|---|---|---|
| Linear plus the 50 nH / 50 pF floor | 200 nH, 80 pF | 1.42:1 |
| Per-relay | 44 nH, 34.5 pF | 1.56:1 |
| Linear, no floor | 150 nH, 30 pF | 1.73:1 |
| **Measured** | | **1.23:1 at 14 MHz; 1.13:1 here** |

The floor model is still the best of the three. The gap to 1.13:1 is most likely the T and cable looking different at 28 MHz. A VNA reading of the load at 28.074 MHz would turn this into a proper check.

## Fourth point: 80 m (18 Oct 2026)

Same T load, 3.573 MHz: the ATU reported Tuned at 1.74:1 with L step 2, C step 9, C IN (shown as ~0.15 µH and ~140 pF). A real match into 30.5 Ω here needs about 1.13 µH and 712 pF, so the tuner stopped almost at its start: the load alone is 1.64:1, and all three readings predict the same (1.63 to 1.67:1). It says nothing about which reading is right; it shows the tuner accepting a result under its own threshold rather than finding the match.

## Why a floor

The ATU-500 is built on FR4, without controlled-impedance traces. Even with every relay bit off, it keeps some inductance and capacitance in circuit:

- **Series inductance:** traces, relay contacts and the shorting paths across the unused inductors.
- **Shunt capacitance:** open relay contacts, pads and the capacitor bus.
- **The relays themselves:** these are general-purpose relays, not RF parts. Each adds contact and lead inductance when closed and contact-to-contact and coil-to-contact capacitance when open, and the network has 13 of them plus the bypass and C IN/C OUT relays (AJ3G, 17 Oct).

The manual's minimums are taken to be that floor, which HobbyPCB would have found by design and experiment. The schematic alone can't give it, because strays aren't on a schematic. AJ3G's point (17 Oct): a schematic won't account for the strays; those come from the design and from experiment with this kind of board.

## Why the VNA tests through the amplifier were left out

The tuner's input was also measured at the amp's RF input (amp OFF, ATU in), and the bypass path was measured with the ATU on BYP:

- **ATU in:** 19.0 − j15.7 Ω.
- **ATU on BYP:** 47.4 − j28.1 Ω.

The path from the input jack to the antenna jack in bypass (relays, low-pass filter, wiring) changes the impedance a lot. It was modelled two ways, as a line (53.5 Ω, 135°) and as a C-L-C pi (505 pF, 430 nH), and both models agree. Neither the linear nor the per-relay reading of step 3 / step 12 reproduces 19 − j15.7 Ω through that path; the setting that does fit is about L step 9, C step 25 with C OUT. That suggests the tuner held a different setting during the VNA test, or that one load isn't enough to pin down the path. These measurements were therefore not used. The 19 − j15.7 Ω reading also disagrees with the second test (43.6 − j20.4 Ω into 50 Ω at the same tune), which supports the tuner having been in a different state at the time.

## Limits

- The values come from one tune (step 3 / step 12), on one band, into two loads. The floor can't be separated from the per-step size with one point, and the real per-step values aren't exactly linear: the cores' AL varies with frequency and the parts have tolerances.
- The test is end to end, so the strays (and the cable and T, which sit inside the measured load) are lumped into the result. But the tuner reports only an SWR magnitude, not a phase, so the first test alone fitted a range of floors (about 60 to 180 nH series with 15 to 30 pF shunt). The second test's complex reading narrowed it to about 50 to 60 nH and 50 pF, but it includes the amp's own path, which couldn't be taken out.
- The strays are not truly fixed: the capacitance across each open relay depends on which bits are off, so the floor shifts a little with the step.
- The values matter only for display. Nothing in SDRoxide controls the tuner from them.

## How to improve them

1. **Settle the setting during the VNA test.** Tune with the ELAD, then move the cable straight to the VNA without touching anything, and note the tuner box values with the reading.
2. **Collect more points.** Repeat with two or three other mismatched loads, and on 80 m and 10 m. Fit the step size and the floor together, and per bit if the data allows.
3. **Measure the board.** AJ3G's view (18 Oct 2026): even with the schematic, getting much closer would take destructive testing, with the boards taken out, parts unsoldered and a full OSLT calibration. That isn't planned, so these values are treated as final unless more bench data turns up.

## What the ATU-500 firmware shows (18 Oct 2026)

AJ3G supplied the ATU-500's own firmware, V1.1F ("HR500 ATU version 1.1"), as a hex file for a PIC18F26K22. It was disassembled (gputils `gpdasm`) and read at two places. The file is HobbyPCB's and isn't kept in this repo.

**The status reply.** The tuner answers with `L%03u-C%03u-H%01u-B%01u-S%03u-F%03u-`. L and C are the inductor and capacitor step variables, H is set when the capacitor is on the output side, B when the tuner is bypassed. S and F are two more stored values, not yet identified.

**The relay drive** (the routine at 0x04E8). The firmware shifts two bytes into the chained 74HC595s (U3, then U2; sheet 3):

- First byte, to U3 (QA to QF = L1 to L6, QH = C OUT): `(63 − L step)`, plus bit 7 when C is on the output side.
- Second byte, to U2 (QA to QG = C1 to C7, QH = C IN): `C step`, plus bit 7 when C is on the input side.

So there is no lookup table: each step bit drives one relay directly, bit 0 the smallest part. The inductor byte is inverted, which fits relays that short out the inductors not wanted (inferred from the inversion; the schematic doesn't show the contact sense clearly).

**What this means for the values.**

- **Inductance.** With direct binary relays, the linear 0.05 µH a step is what the design would give if the six coils are 50, 100, 200, 400, 800 and 1600 nH (3.15 µH in all, the manual's 3.2 µH). The per-relay table above, from counted turns and catalogue AL, doesn't fit that and fitted the bench worst, so the counted turns or the AL values are likely off. The display stays at 0.05 µH a step plus the 50 nH floor.
- **Capacitance.** The capacitors are series pairs of 22, 47, 100, 220, 470, 1000 and 2200 pF, so the bits are worth 11, 23.5, 50, 110, 235, 500 and 1100 pF. That isn't linear: it tracks 10 pF a step at low steps but is well above it at high steps (step 64 is 1100 pF, not 640 pF; step 127 is about 2030 pF, not 1270 pF). At the bench tune (step 12 = 50 + 110 = 160 pF) a 10 pF floor gives the 170 pF the second test fitted. At step 3 it gives about 45 pF rather than 80 pF.

### Refit (18 Oct 2026, agreed by AJ3G)

With C as the sum of the bits, the floor was refitted to the two 20 m readings (L step 3 = 200 nH with its floor; C step 12 = bits 2 and 3 = 160 pF):

| C floor | C in circuit | Error against 43.6 − j20.4 Ω | SWR into 30.5 Ω (tuner: 1.23:1) |
|---|---|---|---|
| 0 pF | 160 pF | 3.5 Ω | 1.26:1 |
| 5 pF | 165 pF | 2.8 Ω | 1.26:1 |
| **10 pF** | **170 pF** | **2.5 Ω** | **1.27:1** |
| 20 pF | 180 pF | 3.2 Ω | 1.28:1 |

10 pF fits best. It gives the same 170 pF at step 12 as the earlier 50 pF + 10 pF a step, so the 20 m bench tune reads the same as before. The floor is now much smaller, which is more believable for relay and trace strays alone.

The other two points can't separate the readings. On 80 m (step 9) every reading predicts about 1.62:1, as before. On 10 m (step 3) the sum of the bits gives 45 pF and predicts 1.83:1 into 30.5 Ω, against the tuner's 1.13:1; the old linear reading predicted about 1.45:1. But the load wasn't measured at 28 MHz, and the T and cable change a lot there, so this isn't taken as evidence against the firmware, which shows how the relays are actually driven.
