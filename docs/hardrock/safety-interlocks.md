# Hardrock amplifier: safety interlocks

Every place SDRoxide refuses or delays something because doing it would be unsafe for the amplifier. Each is marked in the code with `SAFETY INTERLOCK` and its number here. AJ3G asked (17 Oct 2026) that every such block say what it blocks and why.

Before removing or loosening any of these, read why it is there.

## 1. No relay switching with RF present (amplifier link)

- **Where:** `crates/sdroxide-hardrock/src/link.rs`, `act()`.
- **Blocks:** antenna change, band set, band up/down, and tuner in/out (ON/BYP), whoever asks: this panel, a remote client, or anything else that sends the command.
- **When:** the amplifier says it is transmitting (status digit d1), or shows drive (0.1 W or more) or output (1 W or more), or does not answer the status query asked just before the command.
- **Why:** each of these switches a relay in the RF path. Switching one with RF on it opens or changes the load under power. On AJ3G's HR500 (17 Oct 2026) switching the tuner in or out while transmitting made the amp raise its RFL flag at once. The reflected power spike stresses the VRF2944 output devices, and arcing pits the relay contacts. The amp often skips its status reply mid-transmission, so no answer counts as "not safe": the command is dropped and a warning is logged.
- **What the operator sees:** nothing changes; unkey and click again.

## 2. Relay buttons greyed while transmitting (AMP window)

- **Where:** `crates/sdroxide-ui/src/app/amp.rs`, `rf_present()`, `NOT_WHILE_TX` and the ANT, DN, UP and ON/BYP buttons.
- **Blocks:** the same switches as #1, at the button.
- **When:** the latest reading shows drive of 0.1 W or more, or output of 1 W or more.
- **Why:** as #1. The button is greyed so the operator knows why nothing would happen, and hovering explains: "Not while transmitting: switching a relay with RF on it can damage it. Unkey first." #1 stays as the last check, because a reading can be a fraction of a second old.

## 3. Band follows the radio only between transmissions (engine)

- **Where:** `crates/sdroxide-radio/src/engine.rs`, `poll_amp()`.
- **Blocks:** sending the radio's new frequency to the amplifier (which sets its band) while on the air.
- **Why:** a band change switches the low-pass filter relays; under full power that arcs the contacts and can leave the output devices driving an open or wrong filter. The band is sent as soon as the transmission ends.

## 4. No OFF to PTT while the drive is over the amplifier's limit (AMP window and link)

- **Where:** `crates/sdroxide-ui/src/app/amp.rs` (the PTT/OFF button) and `crates/sdroxide-hardrock/src/link.rs`, `act()`.
- **Blocks:** putting the amplifier in line (OFF to PTT).
- **When:** the drive reads over the amplifier's drive limit: its own yellow DRV level, 8 W on the HR500 (from the operating manual's indicator table; it trips at 12 W). The HR50's 6 W is scaled from the HR500's and untested. The link checks the amplifier's status just before sending; no answer refuses the command, as in #1.
- **Why:** in OFF (bypass) the radio's own power goes straight through to the tuner, so an operator can run a 100 W radio barefoot through the bypassed amp (AJ3G, 18 Oct 2026). Switching to PTT with that still applied would drive the amplifier far past its trip level and can damage the VRF2944 output devices.
- **What the operator sees:** the PTT/OFF button greys while the drive is over the limit, and hovering says "Lower the drive first", with the drive and the limit. In bypass the DRV reading and tile stay green whatever they read, ATU in or not (AJ3G), because high drive there is not a fault.

## 5. Only reads are passed through to the tuner (amplifier link)

- **Where:** `crates/sdroxide-hardrock/src/lib.rs`, `Amp::send()`, through `protocol::passthrough_allowed()`. Every frame the amplifier link or the `hrctl` example sends goes through `Amp::send()`, `Command::Raw` included.
- **Blocks:** any tuner pass-through (`HRTM…`) that isn't a read. Allowed are `I`, `V`, `P`, `S`, `F` and `R` alone, and any letter followed by `?`. Refused, among others: `D1`/`D0` (the tuner's own directional coupler in or out of line), `A` (an autotune; TUNE goes as the amplifier's own `HRTU`), `E` (clears the tuner's memory), `Q` (writes its EEPROM), `U` (waits for a bootloader) and every set (`B`, `C`, `H`, `L`, `N`, `Y` with a value). The frame is not written; the caller gets an error naming interlock #5.
- **Why:** WA2EUJ explained (16 Nov 2026) that the tuner is designed to tune on the driver's power, so its coupler should see at most 20 to 40 W; switching it in with `D1` and then transmitting at full power can damage it. The pass-through also reaches commands that wipe or rewrite the tuner. SDRoxide itself only ever sends `HRTMP`; this makes sure nothing else, including a typed `hrctl raw` frame, can send more.
- **What the operator sees:** nothing in normal use. `hrctl raw` with a refused frame prints "not sent: only reads may be passed through to the tuner (interlock #5)".
- **Added:** (AJ3G to vet, 16 Nov 2026), design decision 7. Tested by `only_reads_pass_through_to_the_tuner` and `a_tuner_write_is_refused_before_it_is_sent`.

## Not interlocks, but related

- TUNE is allowed while transmitting: tuning needs a carrier, and the ATU switches its own relays under its own control at tuning power.
- TUNE is not checked against the amplifier's drive limit (AJ3G to vet, 16 Nov 2026). The AMP window's TUNE only sends `HRTU`; the carrier comes from the radio, keyed at the operator's Tune drive under SDRoxide's own SWR guard. The 8 W drive limit applies only to OFF to PTT (#4) and the DRV colours. Hardrock Panel judges its own TUNE against a separate 10 W limit (`tune_max_w`); SDRoxide has no such check to set it on.
- Yellow and red are kept for warnings only (AJ3G): states such as OFF, BYP and tuning use the neutral blue.
