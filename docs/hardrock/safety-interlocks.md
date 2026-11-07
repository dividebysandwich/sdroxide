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

## Not interlocks, but related

- TUNE is allowed while transmitting: tuning needs a carrier, and the ATU switches its own relays under its own control at tuning power.
- Yellow and red are kept for warnings only (AJ3G): states such as OFF, BYP and tuning use the neutral blue.
