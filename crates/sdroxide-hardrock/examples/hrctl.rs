//! `hrctl` — a command-line monitor and control for Hardrock amplifiers.
//!
//! ```text
//! hrctl [--port COM5] [--baud 19200] <command>
//!
//!   find                    scan every port and rate for an amplifier
//!   status                  one HRST report
//!   monitor [ms]            poll HRST continuously (default every 250 ms)
//!   log [ms]                monitor with timestamps and anything the amp sends unasked (default 100 ms)
//!   band <160|80|...|10>    select a band
//!   freq <hz>               tell the amp the radio's frequency
//!   mode <off|ptt>          bypass, or key with the radio
//!   ant <1|2>               select an antenna
//!   atu <on|off|tune|state> ATU in line, bypassed, start a tune, or report
//!   raw <text>              send anything, print whatever answers
//! ```
//!
//! Without `--port` the amplifier is found by scanning, which sends only the
//! harmless `HRAA;` to each port.

use std::process::ExitCode;
use std::thread::sleep;
use std::time::Duration;

use sdroxide_hardrock::{Amp, Band, Command, KeyMode, Response, Status, TempScale};

/// The HR500's own warning levels, from its operating manual, so the monitor
/// flags the same things the front panel would. The amplifier enforces its own
/// trips; these only colour the output.
const WARN_FWD_W: f32 = 500.0;
const WARN_RFL_W: f32 = 30.0;
const WARN_DRV_W: f32 = 8.0;
const WARN_IDD_A: f32 = 18.0;
const WARN_VDD_LOW_V: f32 = 45.0;
const WARN_TEMP_C: f32 = 50.0;

fn usage() -> ExitCode {
    eprintln!(
        "usage: hrctl [--port PORT] [--baud RATE] <find|status|monitor [ms]|log [ms]|band B|freq HZ|mode off|ptt|ant 1|2|temp c|f|atu on|off|tune|state|raw TEXT>"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut port = None;
    let mut baud = 19200;
    while args.first().is_some_and(|a| a.starts_with("--")) {
        let flag = args.remove(0);
        let Some(val) = (!args.is_empty()).then(|| args.remove(0)) else {
            return usage();
        };
        match flag.as_str() {
            "--port" => port = Some(val),
            "--baud" => match val.parse() {
                Ok(b) => baud = b,
                Err(_) => return usage(),
            },
            _ => return usage(),
        }
    }
    let Some(cmd) = args.first().cloned() else {
        return usage();
    };
    let rest = &args[1..];

    let opened = match &port {
        Some(p) => Amp::open(p, baud).map(|a| (p.clone(), baud, a)),
        None => Amp::find(),
    };
    let (name, rate, mut amp) = match opened {
        Ok(x) => x,
        Err(e) => {
            eprintln!("hrctl: {e}");
            return ExitCode::FAILURE;
        }
    };

    // HRST's temperature comes in whichever scale the amp displays; only HRTP
    // says which.
    let celsius = match amp.query(&Command::GetTemperature) {
        Ok(Response::Temperature(_, Some(scale))) => Some(scale == TempScale::C),
        _ => None,
    };

    let result = match cmd.as_str() {
        "find" => {
            println!("Hardrock found on {name} at {rate} baud");
            Ok(())
        }
        "status" => amp.status().map(|s| println!("{}", line(&s, celsius))),
        "monitor" => {
            let ms = rest.first().and_then(|s| s.parse().ok()).unwrap_or(250);
            eprintln!("monitoring {name} at {rate} baud every {ms} ms; Ctrl+C to stop");
            loop {
                match amp.status() {
                    Ok(s) => println!("{}", line(&s, celsius)),
                    Err(e) => eprintln!("{e}"),
                }
                sleep(Duration::from_millis(ms));
            }
        }
        "log" => {
            // Like monitor, but each line carries the seconds since start, and
            // anything else the amplifier says on its own is printed as it
            // arrives, so a trip can be lined up with the readings before it.
            let ms = rest.first().and_then(|s| s.parse().ok()).unwrap_or(100);
            let t0 = std::time::Instant::now();
            eprintln!("logging {name} at {rate} baud every {ms} ms; close the window to stop");
            loop {
                let t = t0.elapsed().as_secs_f32();
                match amp.status() {
                    Ok(s) => println!("{t:8.2}  {}", line(&s, celsius)),
                    Err(e) => println!("{t:8.2}  error: {e}"),
                }
                for r in amp.unsolicited.drain(..) {
                    println!("{t:8.2}  amp says: {r:?}");
                }
                sleep(Duration::from_millis(ms));
            }
        }
        "band" => match rest.first().and_then(|b| Band::from_label(b)) {
            Some(b) => amp.set_band(b),
            None => return usage(),
        },
        "freq" => match rest.first().and_then(|f| f.parse().ok()) {
            Some(hz) => amp.set_frequency(hz),
            None => return usage(),
        },
        "mode" => match rest.first().map(String::as_str) {
            Some("off") => amp.set_mode(KeyMode::Off),
            Some("ptt") => amp.set_mode(KeyMode::Ptt),
            _ => return usage(),
        },
        "ant" => match rest.first().and_then(|a| a.parse().ok()).filter(|n| *n == 1 || *n == 2) {
            Some(n) => amp.set_antenna(n),
            None => return usage(),
        },
        "temp" => match rest.first().map(|t| t.to_ascii_lowercase()).as_deref() {
            Some("c") => amp.send(&Command::SetTempScale(TempScale::C)),
            Some("f") => amp.send(&Command::SetTempScale(TempScale::F)),
            _ => return usage(),
        },
        "atu" => match rest.first().map(String::as_str) {
            Some("on") => amp.send(&Command::SetAtuActive(true)),
            Some("off") => amp.send(&Command::SetAtuActive(false)),
            Some("tune") => amp.send(&Command::Tune),
            Some("state") => atu_state(&mut amp),
            _ => return usage(),
        },
        "raw" if !rest.is_empty() => {
            let c = Command::Raw(rest.join(" "));
            amp.send(&c).and_then(|_| {
                // Whatever comes back in the next half second.
                for r in amp.listen(Duration::from_millis(500))? {
                    println!("{r:?}");
                }
                Ok(())
            })
        }
        _ => return usage(),
    };
    for r in amp.unsolicited.drain(..) {
        if let Response::Other(s) = &r {
            println!("{s};");
        }
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hrctl: {e}");
            ExitCode::FAILURE
        }
    }
}

fn atu_state<P: std::io::Read + std::io::Write>(amp: &mut Amp<P>) -> sdroxide_hardrock::Result<()> {
    let present = amp.query(&Command::AtuPresent)?;
    if present != Response::AtuPresent(true) {
        println!("ATU: not fitted");
        return Ok(());
    }
    let active = amp.query(&Command::GetAtuActive)?;
    let tuning = amp.query(&Command::IsTuning)?;
    let last = amp.query(&Command::LastTuneResult)?;
    println!("ATU: {active:?}, {tuning:?}, last tune {last:?}");
    Ok(())
}

/// One monitor line, with `!` after any reading past the amplifier's own
/// warning level.
fn line(s: &Status, celsius: Option<bool>) -> String {
    let flag = |bad: bool| if bad { "!" } else { "" };
    // Without HRTP's letter, a reading over 100 can only be Fahrenheit.
    let fahrenheit = celsius.map_or(s.temperature > 100.0, |c| !c);
    let temp_c = if fahrenheit { (s.temperature - 32.0) / 1.8 } else { s.temperature };
    let mut out = format!(
        "FWD {:4.0} W{}  RFL {:3.0} W{}  DRV {:4.1} W{}  SWR {:3.1}  VDD {:4.1} V{}  IDD {:4.1} A{}  TEMP {:3.0}{}{}  {}  {}  ANT{}",
        s.forward_w,
        flag(s.forward_w > WARN_FWD_W),
        s.reflected_w,
        flag(s.reflected_w > WARN_RFL_W),
        s.drive_w,
        flag(s.drive_w > WARN_DRV_W),
        s.swr,
        s.vdd_v,
        flag(s.vdd_v > 0.0 && s.vdd_v < WARN_VDD_LOW_V),
        s.idd_a,
        flag(s.idd_a > WARN_IDD_A),
        s.temperature,
        match celsius {
            Some(true) => "C",
            Some(false) => "F",
            None => "",
        },
        flag(temp_c > WARN_TEMP_C),
        match s.mode {
            KeyMode::Off => "OFF",
            KeyMode::Ptt => "PTT",
        },
        s.band.map_or("--", |b| b.label()),
        s.antenna,
    );
    if !s.extra.is_empty() {
        out.push_str(&format!("  +{}", s.extra.join("-")));
    }
    out
}
