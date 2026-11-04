# Time signals: what to add next, and setting the clock per OS

Status of the `time-sync-signals` branch as of 2026-11-04, and a plan for
what comes after it.

> **Update, later the same day.** Done on this branch since: the AppImage
> `pkexec` fix (§2, Linux problem 1); Windows clock setting (§2, Windows plan
> steps 1–5, 7 and 8, with the hover text and manual corrected); DCF77's phase
> code (§1.2), read against a template learnt from the air rather than a
> built-in sequence, since PTB's description could not be fetched to check the
> sequence against; BPC (§1.1), from the publicly described layout, untested on
> air; and the seconds-only mode with BPM and RWM (§1.5, §1.6), tested against
> synthetic signals only. Still open: the Linux RTC, atomic and slewed steps
> (§2, Linux problems 2–4), the macOS headless-prompt edge case, and §1.3,
> §1.4 and Tier 4.

Two parts:

1. [New signals](#1-new-signals): which stations and codes are worth adding,
   in what order, and what each needs from the decoder.
2. [Setting the clock](#2-setting-the-clock-on-linux-macos-and-windows): what
   works today on Linux, macOS and Windows, what was verified, and what to fix.

## 0. What the branch has today

| Station | Band | Code | Read by |
|---|---|---|---|
| DCF77 | 77.5 kHz | ASK, 1 bit/s, parity | amplitude (coherent) |
| ALS162 | 162 kHz | PM, DCF77-like, parity | carrier phase |
| MSF | 60 kHz | ASK, 2 bits/s (A, B), parity | amplitude |
| WWVB | 60 kHz | ASK + PM (2012 phase code) | amplitude and phase |
| JJY 40 / JJY 60 | 40 / 60 kHz | ASK, inverted keying, parity | amplitude |
| WWV / WWVH | 2.5–25 MHz | ticks + 100 Hz BCD subcarrier | ticks, subcarrier |

The decoder fits stations that say one symbol per second as a pattern of
ten 100 ms slots (`format::Template`). Each station is a `Format`: templates,
edge polarity, frame length, encoder and decoder. HF stations are timed by
their ticks (`hf.rs`). Frames are tied to sample positions, and `Tracker`
needs no 60 s cadence, so frames that arrive more often than once a minute
are fine.

## 1. New signals

Station statuses below come from secondary sources: Wikipedia, sigidwiki,
Meinberg's glossary, and swling.com. The BIPM's annual time-signals report
(`webtai.bipm.org/.../timesignals_2025.pdf`) is the authoritative register, but
it could not be fetched from the build container. Check each station there
before starting work on it. Every status marked *verify* depends on that check.

### Tier 1: full time-of-day code that fits the existing framework

#### 1.1 BPC, Shangqiu, China, 68.5 kHz (top recommendation)

- **Why:** it is the only LF time service in China, and it reaches East Asia,
  where JJY is the only other choice. It is on air about 21 h a day (*verify*).
  It is a good fit for the decoder: ASK with carrier reductions of 100, 200,
  300 or 400 ms at the start of each second. That means 2 bits a second, all
  on the 100 ms slot grid.
- **Code (as publicly described):** a 20 s frame, so three per minute. Second 0
  of each frame has no reduction and marks the frame. Seconds 1–19 carry
  quaternary symbols: frame index (0/1/2), hour, minute, day of week, AM/PM,
  day, month, year, and parity bits. The time is China Standard Time
  (UTC+8, no DST).
- **Fit:**
  - Templates `low(0,1)`, `low(0,2)`, `low(0,3)` and `low(0,4)` map to
    `Sym::ab(a, b)`, the way MSF's A + 2·B already does. The marker template
    is all-high.
  - `frame_len = 20`. `encode` still returns `[Sym; 60]`, as three frames.
  - The status scope and symbol strip need no change.
- **Risk:** NTSC does not publish the bit map openly (sigidwiki says a licence
  is required). The public tables are reverse-engineered, and sources disagree
  on the parity and frame-index details. Mitigations:
  - Validate against real recordings before shipping. sdroxide already drives
    KiwiSDRs, and receivers in China, Japan and Korea hear BPC.
  - Keep `confirmations` at 2 or more.
  - Require that the three frames in a minute agree.
- **Extras:**
  - `TimeStation::Bpc`, appended at the end of the enum, because postcard
    numbers variants by position.
  - `zone_min = +480`.
  - refid `BPC\0`. RFC 5905 has no entry for BPC, but the refid is free text.
  - Site: Shangqiu, Henan, 34.457 N 115.837 E (*verify*).

#### 1.2 DCF77's phase (PRN) code, an upgrade to an existing station

- **Why:** DCF77 also sends each second's bit as a 512-chip pseudo-random
  phase sequence (±15.6°) after the amplitude drop. PTB has published it since
  1983 (Hetzel). Correlating that sequence brings two gains:
  - timing that is one to two orders of magnitude sharper than the
    amplitude edge;
  - several dB more sensitivity, which matters at the edge of coverage.

  This is the DCF77 counterpart of what the branch already does for WWVB.
- **Fit:** the coherent front end already follows DCF77's carrier phase.
  Steps:
  1. Add a PRN correlator over the 793 ms window.
  2. The bit is the sign of the correlation, so it fills the same `Sym`.
  3. Report the code as `TimeCode::Phase`/`Both`, as WWVB does.
  4. Feed the correlation peak to the timing in place of the edge when it is
     stronger.
- **Risk:** low. The specification is public and stable.

#### 1.3 HLA, Daejeon, Korea, 5 MHz (conditional)

- **Code:** per Meinberg, a BCD NRZ code at 1 bit/s carrying year, month, day,
  day of week, hour and minute in seconds 17–59, with DUT1 sent as double
  pulses. It is close to WWV, so it can reuse `hf.rs`.
- **Status:** **unclear.** Meinberg lists HLA as "suspended", but a 2026
  aggregator lists 5000 kHz around the clock. **Do not start until KRISS
  confirms it is on air.**
  CHU is the precedent: a decoder for a station that is off the air only
  decodes impostors.

### Tier 2: Russian LF with a time code

#### 1.4 RBU, Taldom, 66.66 kHz, and RTZ, Irkutsk, 50 kHz

- **Why:** they cover Russia and Central Asia, and RTZ covers parts of East
  Asia. Both carry a time code and DUT1.
- **Code:** a phase-modulated "DXXXW" format that is unlike anything in the
  branch. Public descriptions are thin. The radioscanner.ru signal base and
  VNIIFTRI notices are the starting points.
- **Status:** RTZ has a published hourly break, and the RBU schedule is
  unclear (*verify*).
- **Effort:** high. This needs a new phase-symbol reader along the lines of
  `als162.rs`, plus documentation work before any code. Plan it after Tier 1.

### Tier 3: tick-only short-wave stations (seconds and minutes, no date)

These stations send precise seconds and minute marks but no time of day. They
can only discipline a clock that is already within ±30 s. The date and hour
come from the computer's own clock, and the radio supplies the second and the
fraction. On HF, which is good to about 1 ms, that is useful: it corrects a
drifting clock where no coded station can be heard.

**Shared prerequisite: a "ticks only" mode.**

- Add a `Format` variant with no `decode`. The minute marker produces a
  `TimeFrame` whose minute is the computer clock's nearest minute.
- A hard guard: never step or serve more than ±29 s from this mode, because
  outside that the minute itself is ambiguous. Refuse with a new `Refusal`.
- The panel shows "seconds only — date from this computer".
- The NTP server may still serve it, but RFC 5905 needs a distinct refid per
  station.

#### 1.5 BPM, Pucheng, China, 2.5 / 5 / 10 / 15 MHz

- **Code:** 10 ms ticks of 1 kHz each second and a 300 ms minute tick, with
  voice and Morse ID at :29 and :59.
- **Hazards:**
  - **UT1 minutes.** Minutes 25–29 and 55–59 carry UT1 rather than UTC, and
    those differ by DUT1, up to 0.9 s. The decoder must ignore those minutes.
    Getting this wrong steps the clock by up to 0.9 s.
  - **Advance.** Some sources say BPM is sent 20 ms ahead of UTC. Confirm this
    against BIPM and make it a per-station constant.
  - **Shared frequencies and pitch.** BPM uses WWV/WWVH's frequencies, and its
    1 kHz tick has the same pitch as WWV's. Ways to tell them apart:
    - tick length: 10 ms against 5 ms;
    - minute pattern: BPM's 300 ms tick against WWV's 800 ms 1000 Hz tone;
    - WWV's 100 Hz subcarrier, which BPM does not have.

    Extend `sibling()` from one sibling to a set.

#### 1.6 RWM, Moscow, 4996 / 9996 / 14996 kHz

- **Code:** A1 carrier keying, with no audio tone. Seconds pulses are 100 ms
  (or 40 ms by some sources: *verify*), and the minute pulse is 500 ms. The
  schedule runs in 10-minute segments:
  - unmodulated carrier;
  - transmitter off at :08 and :38;
  - Morse ID;
  - 1 Hz and 10 Hz pulse segments.

  DUT1 is sent as doubled pulses.
- **Fit:** the keying is like long-wave ASK heard on HF. The amplitude
  front end works, but the timing needs HF's fade handling from `hf.rs`, and
  the decoder has to follow the segment schedule.

### Tier 4: opportunistic time from decoders the program already has

These are not time stations, but each one's transmitter is locked to UTC.
They are worth considering after Tiers 1–3, as a cross-check or a fallback.

- **AIS (`sdroxide-ais`):**
  - Base-station reports (messages 4 and 11) carry UTC to the second.
  - SOTDMA slots (26.67 ms) are aligned to the UTC minute by GNSS-locked
    transmitters.
  - The slot start of a base-station burst gives millisecond-level time on
    VHF, near ports and coasts. Propagation is line of sight and well defined.
- **HFDL (`sdroxide-hfdl`):** ground-station squitters are slotted on a
  UTC-aligned frame structure. This is useful but lower priority, because the
  timing depends on the ground station and is less documented.

### Not recommended

| Candidate | Why not |
|---|---|
| CHU | Closed 22 June 2026, already excluded on the branch. |
| YVTO, LOL, other dormant HF services | Long off air or irregular; same reasoning as CHU. |
| RDS CT, DRM / DAB / HD time, FT8 `DT` | Minute resolution or unknown latency, not authoritative. Fine for display, not for setting a clock. |
| Beta (VLF, military) | Irregular and not intended for public timekeeping. |
| GNSS (GPS L1 C/A) | Would be the most accurate source, but needs an acquisition and navigation solver, which is a project of its own, not a `Format`. |
| Loran-C / Chayka / eLoran | No time of day (except eLoran's data channel); hard to receive and decode. |

### Suggested order

1. **DCF77 phase code.** Low risk, public spec, and it improves the most-used
   station.
2. **BPC.** Large new coverage area, small fit into `Format`. Gate it on
   validation with real recordings.
3. **Ticks-only mode, then BPM, then RWM.**
4. **RBU / RTZ**, once their format is documented.
5. **HLA**, only if its status is confirmed.
6. **AIS time** as a cross-check source.

Each new station needs the same set of pieces as the ones on the branch:

- the enum variant (appended);
- `label`, `carriers_hz`, `site`, `lat_lon` and `ntp_refid`;
- `Format` with encoder, decoder and `fill` if it has parity;
- a `synth` model;
- round-trip, bit-flip and shifted-window tests in `format/mod.rs`;
- the noise and off-rate tests in `lib.rs`;
- the panel chip;
- a section in `USER_MANUAL.md` §3.26.

## 2. Setting the clock on Linux, macOS and Windows

How it works today (`crates/sdroxide-clock/src/os.rs`): the clock is always
*moved by an offset*, never set to an absolute time. Either the process steps
it itself (`ClockAuthority::Direct`), or it re-runs its own binary as root
with `--set-clock-offset-ns N` through the system's administrator prompt
(`Prompt`). `helper_main()` runs first in `main()`, so nothing else runs with
privilege.

### Summary

| | Direct (no prompt) | Through a prompt | AUTO | Status |
|---|---|---|---|---|
| **Linux** | root or `CAP_SYS_TIME`: works | `pkexec`: works for `.deb` and tarball, **fails for the AppImage** | works with Direct | **Yes**, with one packaging bug |
| **macOS** | root: works (by reading the code; not run on a Mac) | `osascript … with administrator privileges`: works for the `.app` (by reading the code) | works only as root | **Yes**, needs a test on real hardware |
| **Windows** | **not implemented** | **not implemented** | never | **No.** Always `Unavailable`; only the NTP server path works |

### Linux: verified in this container

A standalone copy of `os.rs` was built and run:

- As root: `authority() = Direct`, and `step_direct(0)` succeeded.
- A real `+2 s` step through the helper flag, then a `−2 s` step, moved the
  clock by +1.999995 s and −2.000006 s (net −10 µs). This was checked against
  `CLOCK_MONOTONIC`.
- As `nobody` with no capabilities and no `DISPLAY`: `Unavailable`, and
  `settimeofday` returned `EPERM`.
- As `nobody` with only an ambient `CAP_SYS_TIME`: `Direct`, and the step
  succeeded. The `CapEff` bit test is therefore correct.
- Bad helper arguments are refused with exit 2, and oversized steps with
  `InvalidInput`.

Not testable here: the `pkexec` dialog itself (no polkit agent).

**Problems found:**

1. **The AppImage cannot use the password prompt.** `step_elevated` passes
   `current_exe()` to `pkexec`. Inside an AppImage that path is
   `/tmp/.mount_XXXX/usr/bin/sdroxide`, on a FUSE mount made without
   `allow_other`, and root cannot reach it, so the helper never starts. The
   fix: when `$APPIMAGE` is set, run `pkexec "$APPIMAGE" --set-clock-offset-ns
   N`. AppRun already passes `"$@"` through.
2. **The RTC is not written.** `settimeofday` changes only the system clock.
   The kernel copies it to the RTC only in NTP "11-minute mode", so a reboot
   brings back the old time. Options:
   - in the helper, after a successful step, run `hwclock --systohc` or use
     `RTC_SET_TIME` on `/dev/rtc`;
   - or document that the change lasts only until a reboot.
3. **Precision and atomicity (minor).** The read-then-write pair of
   `gettimeofday`/`settimeofday` loses the microseconds between the two calls,
   which measured as about 4–7 µs. `clock_adjtime(CLOCK_REALTIME,
   {ADJ_SETOFFSET | ADJ_NANO})` makes the kernel apply the offset atomically,
   in nanoseconds. This is exactly the "move, not set" design, done by the
   kernel.
4. **AUTO steps rather than slews.** For the small corrections AUTO makes
   (≥ 100 ms), slewing with `adjtime(2)` or `ADJ_OFFSET` would avoid
   timestamps that jump backwards. `adjtime` also exists on macOS, so one
   change covers both systems.

### macOS: reviewed, not run (no Mac in the container)

- `geteuid() == 0` gives Direct. Otherwise `osascript` runs `do shell script
  (quoted form of <exe>) & " --set-clock-offset-ns N" with administrator
  privileges`. The path is quoted correctly for paths with spaces (the bundle
  lives at `/Applications/sdroxide.app/Contents/MacOS/sdroxide`). User cancel
  (`-128`) is recognised. `settimeofday` as root is allowed under SIP, since it
  is how `date(1)` sets the time.
- The panel already warns that `timed` puts the clock back while "Set time and
  date automatically" is on.
- Edge case: `--server` started by launchd with nobody logged in. There is no
  `SSH_CONNECTION`, so `prompt_available()` reports `Prompt`, but no dialog can
  be shown and `osascript` fails. To fix, also require a console user, for
  example by checking that `stat -f %Su /dev/console` is not `root` /
  `loginwindow`, or that `SecurityAgent` can be reached.
- **To do:** a manual test on Apple Silicon and Intel, covering the `.app`
  from the `.dmg`, a quarantined download, and a path with a space.

### Windows: not supported, and the UI says the wrong thing

- `authority()` only has a `#[cfg(unix)]` branch, so Windows always reports
  `Unavailable`.
- `step_direct` returns `Unsupported`, and `elevated()` returns "not supported
  on this system".
- The panel's hover text says *"run it as an administrator"*, but on Windows
  that does not help: even an elevated process reports `Unavailable`. The AUTO
  hover says the same. **This is misleading and should be fixed even if
  Windows support waits.**

**Plan for Windows.** The workspace already uses `windows-sys` 0.61, in
`sdroxide-relay`.

1. **Authority.**
   - Open the process token. If it holds `SeSystemtimePrivilege`
     (`LookupPrivilegeValueW` + `GetTokenInformation(TokenPrivileges)`), the
     authority is **Direct**. Under UAC that means an elevated administrator,
     or a service running as LocalService or LocalSystem.
   - Otherwise, if the user is in the Administrators group (split token) or UAC
     can ask for credentials, the authority is **Prompt**.
   - Otherwise it is Unavailable.
2. **Direct step.**
   1. `AdjustTokenPrivileges` to enable the privilege.
   2. `GetSystemTimePreciseAsFileTime`, then add the offset in 100 ns units.
   3. `FileTimeToSystemTime`, then `SetSystemTime`. Its resolution is 1 ms,
      which is enough here.
   4. Restore the privilege.
3. **Prompt step.**
   - `ShellExecuteExW` with `lpVerb = "runas"` on `current_exe()` with
     `--set-clock-offset-ns N` and `SEE_MASK_NOCLOSEPROCESS`, then
     `WaitForSingleObject` and `GetExitCodeProcess`.
   - `ERROR_CANCELLED` (1223) means the user declined.
   - Set `SW_HIDE` so no console window flashes if the binary is built for
     the console subsystem.
4. **Sync daemon.** Query the `W32Time` service with
   `OpenSCManager`/`QueryServiceStatus`. If it is running, say "Windows Time
   service (w32time)". Windows' "Set time automatically" puts the clock back
   on its schedule, just as `timed` does on macOS.
5. **NTP server on Windows.** Unprivileged binds to port 123 are allowed, but
   `w32time` may already hold UDP 123. Extend the existing "try 1123" hint to
   that case (`WSAEADDRINUSE`). Pointing Windows at sdroxide is then
   `w32tm /config /manualpeerlist:"127.0.0.1:1123,0x8" /syncfromflags:manual /update`.
   Document this in §3.26.
6. **Slewing (optional).** `SetSystemTimeAdjustmentPrecise` is Windows'
   counterpart to `adjtime`, for AUTO.
7. **Tests.**
   - A `cfg(windows)` unit test for authority detection.
   - In CI, `cargo check --target x86_64-pc-windows-gnu -p sdroxide-clock`.
     The release job already builds that target.
   - Manual: the MSI install (per machine), a declined UAC prompt, and a
     non-admin account.
8. **Manual.** Add a **Windows** bullet to §3.26 "Setting a clock needs
   administrator rights", and change the hover text in
   `panels/timesig.rs` (lines ~493–496 and ~533–535) to name each platform's
   real mechanism.

### Cross-platform follow-ups

- **Hover text per platform.** Name the actual mechanism: `pkexec`, the macOS
  dialog, or UAC.
- **One test seam.** Put `step_direct` behind a small trait so the
  `TimeHub::request_step` → `poll_step` path, including the "a fix moves with
  the clock" bookkeeping, can be tested without root. The arithmetic is already
  tested in `check_step`, but the outcome plumbing is not.
- **Helper on Windows.** `helper_main` already runs on every platform, and
  today it fails safely on Windows, because `step_direct` returns
  `Unsupported` and the helper exits 1. Once Windows is implemented, keep the
  same limits there: the 400-day cap and refusing extra arguments.
