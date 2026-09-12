//! `atchat-client` — a headless AtCHAT NET station on a plain terminal, for
//! RF-less testing.
//!
//! The same [`AtChatSession`] the sdroxide ATCHAT mode drives, here on a
//! virtual TCP channel (`atchat-channel`, or `channel_server.py`) instead of
//! the radio. Type to chat; the roster, master election and ARQ file transfer
//! all run underneath exactly as they do on the air.
//!
//!   atchat-client TA1ABC --connect 127.0.0.1:6000
//!
//! Commands: `/msg <call> <text>`, `/sendfile <path> [call]`, `/status`,
//! `/drop`, `/reconnect`, `/quit`. Anything else is a line to the common
//! channel.

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use sdroxide_atchat::AtChatSession;

#[derive(Parser, Debug)]
#[command(about = "Headless AtCHAT NET station over a virtual channel — RF-less test tool")]
struct Args {
    /// This station's callsign.
    callsign: String,

    /// The channel address to connect to.
    #[arg(long, default_value = "127.0.0.1:6000")]
    connect: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,sdroxide_atchat=info".into()),
        )
        .init();

    let args = Args::parse();
    let session = AtChatSession::new(&args.callsign, Some(args.connect.clone()));
    println!(
        "[{}] joining {} …  (type /quit to leave)",
        args.callsign.to_uppercase(),
        args.connect
    );

    let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut buf = String::new();
        loop {
            buf.clear();
            match stdin.read_line(&mut buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if line_tx.send(buf.trim_end().to_string()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut tick = tokio::time::interval(Duration::from_millis(300));
    let mut seen_chat = 0usize;
    let mut seen_log = 0usize;
    let mut seen_files = 0usize;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let snap = session.snapshot();
                for c in snap.chat.iter().skip(seen_chat) {
                    if c.own { continue; }
                    if c.private {
                        println!("  [{} → me] {}", c.from, c.text);
                    } else {
                        println!("  <{}> {}", c.from, c.text);
                    }
                }
                seen_chat = snap.chat.len();
                for l in snap.log.iter().skip(seen_log) {
                    println!("  · {l}");
                }
                seen_log = snap.log.len();
                for f in snap.files.iter().skip(seen_files) {
                    println!("  ⇩ file from {}: {} → {}", f.from, f.filename, f.path);
                }
                seen_files = snap.files.len();
            }
            maybe = line_rx.recv() => {
                let Some(line) = maybe else { break };
                let line = line.trim();
                if line.is_empty() { continue; }
                if let Some(rest) = line.strip_prefix('/') {
                    let mut it = rest.splitn(2, ' ');
                    let cmd = it.next().unwrap_or("");
                    let arg = it.next().unwrap_or("").trim();
                    match cmd {
                        "quit" | "q" => break,
                        "drop" => { session.drop_link(); println!("  · dropped"); }
                        "reconnect" | "rc" => { session.reconnect(); println!("  · reconnecting"); }
                        "status" | "s" => print_status(&session),
                        "msg" | "m" => {
                            let mut p = arg.splitn(2, ' ');
                            match (p.next(), p.next()) {
                                (Some(call), Some(text)) if !text.is_empty() => {
                                    session.send_chat(&call.to_uppercase(), text);
                                }
                                _ => println!("  usage: /msg <call> <text>"),
                            }
                        }
                        "sendfile" | "f" => {
                            let mut p = arg.splitn(2, ' ');
                            match p.next() {
                                Some(path) if !path.is_empty() => {
                                    let dst = p.next().unwrap_or("").trim().to_uppercase();
                                    session.send_file(PathBuf::from(path), &dst);
                                    println!("  · sending {path}");
                                }
                                _ => println!("  usage: /sendfile <path> [call]"),
                            }
                        }
                        other => println!("  ? unknown command /{other}"),
                    }
                } else {
                    session.send_chat("", line);
                }
            }
        }
    }

    println!("bye");
    Ok(())
}

fn print_status(session: &AtChatSession) {
    let s = session.snapshot();
    println!(
        "  {} | {} | role={} | master={}",
        s.my_call,
        if s.connected { "connected" } else { "offline" },
        s.role.unwrap_or("-"),
        s.master.as_deref().unwrap_or("-"),
    );
    for (call, status, age) in &s.roster {
        println!("    {call:8} {status:6} {age:.0}s");
    }
    for t in &s.transfers {
        let dir = if t.incoming { "in " } else { "out" };
        println!(
            "    {dir} {} {} {}/{}{}",
            t.filename,
            t.peer,
            t.have,
            t.total,
            if t.complete { " ✓" } else { "" }
        );
    }
}
