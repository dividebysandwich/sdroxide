//! The sign-in gate every remote connection passes through.
//!
//! Two rules, and they are the whole design:
//!
//! * **One attempt at a time, server-wide.** Guessing a password is only
//!   practical if guesses can be made in parallel, so they cannot be: every
//!   attempt on every socket queues for the same turnstile. Whoever finds it
//!   occupied is told to come back rather than being parked on it, so the queue
//!   cannot be made to grow either.
//! * **A wrong answer shuts the door for [`LOCKOUT`].** The next attempt — from
//!   anyone — waits that long before it is judged. Combined with the first rule
//!   that caps the whole server at one guess every three seconds, which turns a
//!   dictionary run into a project measured in years and makes the wall-clock
//!   cost of a wrong guess enormously larger than any timing difference inside
//!   the comparison.
//!
//! The cost is deliberate and worth naming: while an attack is running, an
//! operator signing in legitimately waits their turn too. That is the trade a
//! shared lockout makes, and for a single-radio station it is the right way
//! round — a few seconds of delay against somebody else keying the transmitter.

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{info, warn};

use sdroxide_types::{AUTH_BUSY, AUTH_REFUSED, RemoteAccess};

use crate::AccessFn;

/// How long a wrong password shuts the door for.
const LOCKOUT: Duration = Duration::from_secs(3);

/// How long the operator has to get signed in, from the challenge to the last
/// attempt. Generous because a person is typing, quite possibly after going to
/// find a password manager; the turnstile is what limits guessing, not this.
const SIGN_IN_BUDGET: Duration = Duration::from_secs(300);

/// How many tries one socket gets before it is closed. Not the brute-force
/// defence — the turnstile is — but a socket that has sat here misspelling
/// things a dozen times is not going to succeed on the thirteenth, and it
/// stops a client that answers the challenge with garbage from doing so for
/// ever.
const MAX_ATTEMPTS: usize = 12;

/// What a check came back with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Ok,
    Wrong,
    /// Another attempt is being judged right now. Not a rejection — nothing was
    /// even compared — so it neither counts against the client nor extends the
    /// lockout.
    Busy,
}

/// The turnstile, plus how to find out what the server currently demands.
#[derive(Default)]
pub(crate) struct AuthGate {
    /// Consulted once per connection and again on each attempt, rather than
    /// read at startup: it is a file on this machine, and an operator who
    /// changes their password expects the change to hold without restarting
    /// the server and dropping whoever is on it.
    access: Option<AccessFn>,
    /// Both the "one at a time" lock and, inside it, the instant before which
    /// nothing may be judged. `None` means the door is open now.
    turnstile: Mutex<Option<Instant>>,
}

impl AuthGate {
    pub(crate) fn new(access: Option<AccessFn>) -> Self {
        AuthGate { access, turnstile: Mutex::new(None) }
    }

    /// What this server demands right now, or `None` if it is open to anyone
    /// who can reach the port.
    pub(crate) fn required(&self) -> Option<RemoteAccess> {
        let want = (self.access.as_ref()?)();
        want.is_enforced().then_some(want)
    }

    /// Judge one attempt.
    pub(crate) async fn check(&self, username: &str, password: &str) -> Verdict {
        // `try_lock`, not `lock`: an occupied turnstile means somebody else is
        // mid-attempt, and the answer to that is "come back", not a queue an
        // attacker can lengthen at will with one socket per entry.
        let Ok(mut door) = self.turnstile.try_lock() else { return Verdict::Busy };
        // Serve out whatever is left of the last wrong answer's lockout before
        // judging this one, still holding the turnstile so nothing slips past
        // while we wait.
        if let Some(until) = *door {
            tokio::time::sleep_until(until).await;
        }
        // Re-read rather than reuse an earlier copy: the wait above may have
        // been three seconds long, and the operator may have spent them
        // changing the password.
        let Some(want) = self.required() else { return Verdict::Ok };
        if want.accepts(username, password) {
            *door = None;
            Verdict::Ok
        } else {
            *door = Some(Instant::now() + LOCKOUT);
            Verdict::Wrong
        }
    }
}

/// The frames one protocol uses to run a challenge.
///
/// `/ws` and `/solar-ws` speak different message sets but the same exchange, so
/// the loop below is written once and told how to say each thing.
pub(crate) struct Frames<'a> {
    /// Which endpoint this is, for the log.
    pub(crate) what: &'static str,
    /// The encoded "I need a username and password".
    pub(crate) required: Vec<u8>,
    /// Builds the encoded "not those, and here is why".
    pub(crate) rejected: &'a (dyn Fn(&str) -> Vec<u8> + Send + Sync),
    /// Pulls credentials out of a client frame, if that is what it is.
    pub(crate) credentials: &'a (dyn Fn(&[u8]) -> Option<(String, String)> + Send + Sync),
}

/// Run the challenge to a verdict.
///
/// Returns whether the socket may go on. The caller has already read `Hello`
/// and accepted its protocol version, so a client on the wrong protocol is told
/// *that* rather than being asked to sign in to a server it could not have
/// talked to anyway.
pub(crate) async fn challenge(
    socket: &mut WebSocket,
    gate: &AuthGate,
    frames: Frames<'_>,
    already: bool,
) -> bool {
    // A cookie the station itself signed, still inside its dwell and still
    // matching the password this server is configured with today. The sign-in
    // card on this connection is then not needed, and asking for it anyway
    // would be the bug the cookie exists to remove: a station of three radios
    // gives an answer here and then asks for the same answer on the next two.
    if already || gate.required().is_none() {
        return true;
    }
    if socket.send(Message::Binary(frames.required.into())).await.is_err() {
        return false;
    }

    // Bounded two ways: `deadline` caps how long the socket may sit here at all,
    // and `attempts` caps how many guesses it gets within that. Keepalives count
    // against the first but not the second.
    let deadline = Instant::now() + SIGN_IN_BUDGET;
    let mut attempts = 0;
    while attempts < MAX_ATTEMPTS {
        let bytes = match tokio::time::timeout_at(deadline, socket.recv()).await {
            Ok(Some(Ok(Message::Binary(bytes)))) => bytes,
            // Keepalives. Worth handling here and not in the `Hello` read
            // above, because this socket may be open for minutes on end — a
            // person is typing, quite possibly after going to find a password
            // manager — which is exactly long enough for a browser, a proxy or
            // a load balancer to ping it. The transport answers them; all that
            // is needed here is not to mistake one for a failed sign-in.
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
            // Timed out, closed, errored, or a text frame, which this protocol
            // never sends. Nothing here is worth answering.
            _ => return false,
        };
        attempts += 1;
        // Anything that is not an answer to the challenge is either a client
        // that does not know about it or one trying to walk around it, and
        // both get the same treatment: ignored, and charged an attempt.
        let Some((username, password)) = (frames.credentials)(&bytes) else {
            warn!("{}: expected sign-in credentials", frames.what);
            continue;
        };
        let why = match gate.check(&username, &password).await {
            Verdict::Ok => {
                info!("{}: signed in as {username:?}", frames.what);
                return true;
            }
            Verdict::Wrong => {
                // Deliberately the same message whether the username or the
                // password was wrong — see [`AUTH_REFUSED`].
                warn!("{}: sign-in refused for {username:?}", frames.what);
                AUTH_REFUSED
            }
            // Not a refusal, and the wording is a shared constant because the
            // client has to be able to tell it apart from one: an answer that
            // was never compared must not cost a remembered password, and a
            // tab nobody is looking at has to be free to offer it again.
            Verdict::Busy => AUTH_BUSY,
        };
        if socket.send(Message::Binary((frames.rejected)(why).into())).await.is_err() {
            return false;
        }
    }
    warn!("{}: too many sign-in attempts; closing", frames.what);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(user: &str, pass: &str) -> AuthGate {
        let want = RemoteAccess { username: user.into(), password: pass.into() };
        AuthGate::new(Some(Box::new(move || want.clone())))
    }

    /// A server told nothing, and a server told empty strings, are both open —
    /// which is what every installation that predates this feature is.
    #[tokio::test]
    async fn a_server_with_no_credentials_challenges_nobody() {
        assert!(AuthGate::default().required().is_none());
        assert!(gate("", "").required().is_none());
    }

    #[tokio::test]
    async fn the_right_password_gets_in_and_the_wrong_one_does_not() {
        let g = gate("oe1test", "hunter2");
        assert!(g.required().is_some());
        assert_eq!(g.check("oe1test", "hunter2").await, Verdict::Ok);
        assert_eq!(g.check("oe1test", "wrong").await, Verdict::Wrong);
    }

    /// The lockout: after a wrong answer the *next* attempt — right or wrong,
    /// and from whoever — waits out the full three seconds before it is judged.
    #[tokio::test(start_paused = true)]
    async fn a_wrong_answer_shuts_the_door_for_three_seconds() {
        let g = gate("oe1test", "hunter2");
        assert_eq!(g.check("oe1test", "wrong").await, Verdict::Wrong);

        let started = Instant::now();
        assert_eq!(g.check("oe1test", "hunter2").await, Verdict::Ok);
        assert!(
            started.elapsed() >= LOCKOUT,
            "the next attempt was judged after {:?}, not the full lockout",
            started.elapsed()
        );

        // ...and a correct answer opens it again, so an operator who mistypes
        // once is not made to wait a second time.
        let started = Instant::now();
        assert_eq!(g.check("oe1test", "hunter2").await, Verdict::Ok);
        assert!(started.elapsed() < LOCKOUT);
    }

    /// The parallelism rule: while one attempt is being judged — which, after a
    /// wrong answer, takes three seconds — every other attempt is turned away
    /// rather than run alongside it or queued behind it.
    #[tokio::test(start_paused = true)]
    async fn attempts_cannot_be_run_side_by_side() {
        let g = std::sync::Arc::new(gate("oe1test", "hunter2"));
        assert_eq!(g.check("oe1test", "wrong").await, Verdict::Wrong);

        // One guesser sits in the lockout holding the turnstile...
        let held = std::sync::Arc::clone(&g);
        let slow = tokio::spawn(async move { held.check("oe1test", "guess-1").await });
        tokio::task::yield_now().await;

        // ...and everybody else is refused outright, having been compared
        // against nothing.
        for _ in 0..50 {
            assert_eq!(g.check("oe1test", "guess-n").await, Verdict::Busy);
        }
        assert_eq!(slow.await.unwrap(), Verdict::Wrong);
    }

    /// A password changed while the server is running applies to the next
    /// sign-in; nobody has to restart it and drop whoever is connected.
    #[tokio::test]
    async fn the_credentials_are_re_read_for_every_attempt() {
        let live = std::sync::Arc::new(std::sync::Mutex::new(RemoteAccess {
            username: "oe1test".into(),
            password: "hunter2".into(),
        }));
        let seen = std::sync::Arc::clone(&live);
        let g = AuthGate::new(Some(Box::new(move || seen.lock().unwrap().clone())));

        assert_eq!(g.check("oe1test", "hunter2").await, Verdict::Ok);
        live.lock().unwrap().password = "correct-horse".into();
        assert_eq!(g.check("oe1test", "correct-horse").await, Verdict::Ok);
        // And clearing it out reopens the server, without a restart.
        *live.lock().unwrap() = RemoteAccess::default();
        assert!(g.required().is_none());
    }
}
