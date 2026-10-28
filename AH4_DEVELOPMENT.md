# SDRoxide AH-4 Tuner Support --- Development Note

**Last updated:** 22 October 2026\
**Repository:** `/opt/sdroxide`\
**Branch:** `ah4-tuner-support`\
**Last confirmed commit:** `418f4a32` ---
`Add initial AH-4 tuner support and I2C quarantine`

This note is intended to make the work resumable across sessions. Treat
the source code and Git history as authoritative; this note records the
current understanding and outstanding work, not a substitute for
inspecting the code.

## 1. Project objective

Add support in SDRoxide for an Icom AH-4 antenna tuner controlled by
custom firmware running on a Raspberry Pi Pico and connected through the
Hermes-Lite 2 I/O board.

The work must preserve SDRoxide's existing transmit safety mechanisms.
The tuner must never cause RF to be enabled unless the tuner explicitly
requests tuning RF and the interlock has confirmed that condition from
current, trustworthy status information.

## 2. Hardware and software context

-   Radio: Hermes-Lite 2.
-   I/O board: HL2 I/O board.
-   Tuner: Icom AH-4.
-   Custom AH-4 firmware: `icom_ah4.ino`, version 7.9.
-   I/O-board I²C address: `0x1D`.
-   Tuner command/status register: register 7 (`REG_ANTENNA_TUNER`) in
    `crates/sdroxide-hpsdr/src/ioboard.rs`.
-   Working development machine: Linux Mint laptop.
-   Repository path: `/opt/sdroxide`.
-   Working branch: `ah4-tuner-support`.
-   SDRoxide version at the start of this work: v1.6.10.

## 3. Protocol behaviour recorded so far

The following behaviour has been reported from the custom firmware
and/or observed during hardware testing. Reconfirm against the current
firmware source before relying on it for a safety-critical decision.

-   Write `0x01` to register 7 to start a tuning cycle.
-   Write `0x02` to request tuner bypass.
-   The bypass sequence has been observed with START high and status
    `0x03`, followed after about 70 ms by START low and status `0x00`.
-   `0xFD` is used as a reset/status-priming marker. The custom
    firmware's four-second timeout has been reported to emit `0xFD`.
-   Status `0xEE` means the tuner requests the host to transmit tuning
    RF.
-   RF must be enabled only while the current tuner status is exactly
    `0xEE`; it must be disabled immediately when the status ceases to be
    `0xEE`, or on timeout, error, disconnect, or uncertain/stale
    communication.
-   Status `0x00` represents successful completion/idle in the observed
    protocol.
-   `0xFC` and other status values in the `0xF0`--`0xFF` range indicate
    failure/reset/terminal error conditions, with exact interpretation
    dependent on the firmware.
-   The tuner status byte was empirically identified as `data[3]` in the
    EP6 ACK payload. Example observed status reads included
    `09 07 00 03` and `09 07 00 00`.

A separate Pico TX-inhibit pulse after a four-second timeout has been
proposed as an additional protection layer. This is not a replacement
for validating the software interlock.

## 4. Current committed implementation

Commit `418f4a32` includes 15 source files, with 614 insertions and 16
deletions. The change is broader than the latest quarantine work: it
contains the initial AH-4 implementation and supporting plumbing across
the HPSDR, radio, server, protocol, and UI code.

Important locations include:

-   `crates/sdroxide-hpsdr/src/ioboard.rs` --- I/O-board I²C scheduler,
    tuner state machine, timeout handling, and tests.
-   `crates/sdroxide-hpsdr/src/protocol1.rs` --- extracts ACK data from
    EP6.
-   `crates/sdroxide-hpsdr/src/protocol2.rs` --- contains a no-op
    handler for the tuner controller command at present.
-   `crates/sdroxide-radio/src/engine.rs` --- existing `SetTune(on)`
    behaviour and transmit-state synchronisation.
-   `crates/sdroxide-radio/src/source.rs` --- `IqSource` trait; any new
    trait method may need forwarding in `ConvertedSource`.
-   `crates/sdroxide-ui/src/app/top_bar.rs` and adjacent UI files --- UI
    plumbing and temporary tuner status display.
-   `src/hpsdr_source.rs` --- HPSDR source integration.

### Quarantine behaviour

`IoBoard` has an `i2c_quarantined: bool`, initially `false`.

-   `next_request()` returns `None` immediately when quarantined.
-   `on_ack()` ignores ACKs while quarantined.
-   `tuner_start()` and `tuner_recover()` reject requests while
    quarantined.
-   If the ACK for an AH-4 start write (`register 7`, data `0x01`) times
    out, the in-flight operation is abandoned, a fault is latched, the
    I²C bus is quarantined, and the scheduler returns without sending
    another request.
-   There is currently no automatic way to clear quarantine. This is
    intentional until a verified recovery procedure exists.

### Existing tests and validation

At the checkpoint, the following were reported:

-   `cargo test -p sdroxide-hpsdr` passed:
    -   77 unit tests
    -   1 DUC pacing integration test
    -   1 loopback integration test
    -   1 multi-DDC integration test
    -   no failures
-   `git diff --check` was clean before the commit.
-   Focused quarantine tests passed:
    -   `tuner_start_ack_timeout_quarantines_the_i2c_bus`
    -   `late_ack_is_ignored_while_i2c_bus_is_quarantined`
-   A warning reported that `tuner_recover` was unused. It has not yet
    been connected to a user-facing recovery control.

These are software-test results, not proof of hardware safety. Re-run
tests after any subsequent code changes.

## 5. Current Git state and backups

Last confirmed command output:

``` text
418f4a32 (HEAD -> ah4-tuner-support) Add initial AH-4 tuner support and I2C quarantine
```

`git status --short` showed no modified tracked files. It showed 12
untracked backups of `crates/sdroxide-hpsdr/src/ioboard.rs`, all named
`ioboard.rs.bak-*` with timestamp or task suffixes.

The backups were not committed. Keep them until the committed checkpoint
and subsequent changes are confirmed sufficient; do not delete them as
part of unrelated work.

To re-establish current state:

``` bash
cd /opt/sdroxide
git status --short
git log -1 --oneline
```

## 6. Outstanding technical concerns

### A. General ACK timeout semantics

The scheduler uses:

``` rust
const ACK_TIMEOUT: Duration = Duration::from_millis(150);
const MAX_TRIES: u8 = 3;
```

For ordinary operations, it retries the in-flight operation until the
retry limit, calls `give_up()`, clears queued work, invalidates
frequency bookkeeping, and can then schedule another request. The
existing test
`an_unanswered_write_is_retried_and_then_re_sent_with_the_next_update`
expects a new request to be sent after the operation is abandoned.

A missing ACK does not by itself prove the operation was not executed.
Because the ACK protocol does not appear to include a transaction
identifier, a late or duplicate ACK might be misattributed to a later
request. This is a risk hypothesis that needs verification against
Hermes-Lite 2 gateware/protocol semantics; do not claim the exact
behaviour is confirmed until verified.

Do not immediately generalise quarantine to every error without
distinguishing: - a request that receives an explicit `RADDR_ERROR`; - a
request that receives no ACK after retries; - a late or duplicate ACK,
if the gateware can produce one.

Inspect the relevant gateware/protocol documentation or source before
changing this policy.

### B. Stale tuner status

`IoBoard` currently stores `tuner_status: Option<u8>` as the most
recently read status. The reviewed code does not record the time of the
last successful read. After a read failure, an old `0x00` may remain
stored.

Before accepting a new tune, require a fresh and trustworthy idle
status; do not treat an old `0x00` as proof that the tuner is currently
idle.

### C. Tuning timeout versus ACK timeout

These are different failure paths:

-   **Start-command ACK timeout:** the command may or may not have
    reached the tuner. Current code quarantines the I²C bus.
-   **Four-second tuner-cycle timeout after command acknowledgement:**
    current code clears `tuner_start_pending`, marks the tuner
    absent/uncertain (`tuner_present = false`) and latches
    `tuner_fault`, but does not quarantine I²C.

Do not assume the second case means the tuner is idle or bypassed.
Decide whether continued status polling is safe only after reviewing the
firmware protocol and RF interlock requirements.

### D. Recovery is not yet defined

The existing `tuner_recover()` path is blocked during quarantine, by
design. A bypass write and subsequent `0x00` status are not enough on
their own to prove I²C transaction synchronisation if an earlier ACK may
still be outstanding.

Before clearing quarantine, establish a documented recovery mechanism
that: 1. restores or proves transaction synchronisation; 2. does not
accidentally start tuning or enable RF; 3. requests bypass only when it
is safe to do so; 4. verifies a fresh, valid tuner status; 5. leaves the
system faulted and tuning disabled if any step times out or is
ambiguous.

Do not invent a gateware reset or bus-flush guarantee. Verify whether
one exists.

### E. RF interlock not implemented/validated

The ordinary UI TUNE button currently uses the existing
`Command::SetTune(!tx.tune)` path. `SetTune(on)` in
`crates/sdroxide-radio/src/engine.rs` updates transmit state and, when
TX is active, calls `source.set_tx_drive(...)`. `sync_tx_state()`
derives transmit intent from PTT or TUNE and calls
`source.tx_begin(...)`, while enforcing existing safety rails.

Do not bypass `sync_tx_state()` or otherwise circumvent the existing
radio safety logic.

The required future sequence is: 1. Ensure RF is off before issuing the
AH-4 start command. 2. Wait for current, trustworthy status `0xEE`. 3.
Enable the approved low-power tuning carrier only while status remains
exactly `0xEE`. 4. Disable RF immediately when status is no longer
`0xEE`, or on timeout, error, disconnect, stale status, or uncertain
communication. 5. Treat status `0x00` as successful completion only when
the sequence has actually observed the active tuning phase and a valid
return to idle. 6. Keep failure latched until an explicit, verified
recovery sequence completes.

This sequence still needs design, implementation, and hardware
validation. Software alone should not be treated as the sole RF
protection.

## 7. Recommended work plan

Work incrementally. Before each change, inspect the current source and
relevant tests. Make a narrowly scoped patch, run focused tests, run the
package test suite, inspect the diff, and commit only when the result is
understood.

### Stage 1 --- Stabilise existing failure handling

1.  Inspect Hermes-Lite 2 gateware/protocol documentation/source to
    establish the ACK semantics, including whether late or duplicate
    ACKs are possible and how explicit `RADDR_ERROR` differs from a
    missing ACK.
2.  Decide and document the correct policy for each failure class.
3.  Add status freshness tracking and ensure failed status reads cannot
    leave a stale idle status usable for a new tuning request.
4.  Add focused tests for timeout, stale status, late ACK handling, and
    fault latching.
5.  Re-run `cargo test -p sdroxide-hpsdr`, inspect `git diff --check`,
    and commit the stabilisation patch.

Do not change timeout policy based solely on an unverified assumption
about gateware behaviour.

### Stage 2 --- Design recovery

1.  Establish a protocol-backed method to restore transaction
    synchronisation.
2.  Define explicit recovery states and timeout behaviour.
3.  Ensure every uncertain recovery outcome leaves tuning disabled and
    the fault latched.
4.  Test the state machine before wiring a UI recovery control.

### Stage 3 --- Implement the RF interlock

1.  Trace the full `Command::SetTune`, `sync_tx_state()`,
    `source.tx_begin()`, and `set_tx_drive()` paths.
2.  Choose an implementation that uses the existing transmit safety
    mechanisms.
3.  Add unit/state-machine tests for all status transitions and failure
    conditions.
4.  Verify that a stale `0xEE` cannot keep RF enabled.
5.  Keep normal TUNE behaviour unchanged until the new sequence is
    complete and tested.

### Stage 4 --- Controlled hardware tests

Plan and document tests for normal tune, successful completion, bypass,
tuner-reported failure, no ACK, status-read failure, disconnect,
timeout, and recovery failure. Confirm the independent Pico TX-inhibit
behaviour as an additional layer. Do not test uncertain failure paths
with RF enabled unless an independently verified protection arrangement
is in place.

## 8. Working practices

-   The user runs commands locally and pastes output; the assistant
    cannot directly edit `/opt/sdroxide`.
-   Prefer short commands and small, guarded edits. Large terminal
    pastes have previously been truncated or garbled.
-   Before editing, inspect the exact current code. Use exact-match
    guards and make a backup when performing scripted replacements.
-   After editing, inspect the diff before testing.
-   Run focused tests first, then `cargo test -p sdroxide-hpsdr`.
-   Check `git diff --check` and `git status --short`.
-   Commit each well-defined checkpoint so the work can be resumed
    independently.
-   Do not delete the existing untracked backups until the user is
    satisfied they are no longer needed.
-   Be explicit about what is confirmed, what is inferred, and what
    remains unverified. Hardware observations and software tests must
    not be presented as interchangeable evidence.

## 9. Immediate next action

Do not make another code change until the ACK semantics are better
established. Inspect the relevant Hermes-Lite 2 protocol documentation
or gateware source and answer:

-   Can one I²C request produce a late or duplicate ACK after the host
    has timed out and moved on?
-   Does the ACK identify the requested I²C register/operation, or only
    indicate an I²C result?
-   What exactly does `RADDR_ERROR` guarantee about the transaction?
-   Is there a documented operation that safely resets or drains the I²C
    request/ACK state without causing an unintended tuner action?

Once these questions are resolved, proceed with the smallest justified
change.
