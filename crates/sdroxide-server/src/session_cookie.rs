//! The sign-in cookie: a signed statement that a station's *server* remembers
//! somebody, so the browser's other connections to that station do not each
//! have to be given the password again.
//!
//! # Why this exists and what it does not fix
//!
//! A station serves each of its radios on a connection of its own, and asks
//! each one for a password, so a browser holding a whole station meets one
//! sign-in per radio. The client already collapses that — an answer the station
//! accepted is kept for the run and offered to the station's other connections
//! — but that is memory in the page, it does not survive a reload, and it
//! leaves the password sitting in `localStorage` where any script on the origin
//! can read it.
//!
//! This is the other half, and it is the half that is actually the server's
//! business: the *station* says "yes, this is that person, for this long", and
//! the browser replays it automatically on every connection, reload and radio
//! included. Nothing about it is readable by script — [`COOKIE_NAME`] is
//! `HttpOnly` — so the password itself never has to be kept in the page at
//! all on the browser path.
//!
//! # No secret file
//!
//! The signing key is derived from the credentials this server was *already*
//! configured with. That is deliberate and it is the reason there is no new
//! secret to store, back up, or lose:
//!
//! * nothing is written to disk, so there is no third secret to protect;
//! * **changing the password invalidates every cookie at once**, because the
//!   key changed — which is what an operator who changes their password means
//!   and what a separate stored secret would have to be taught to do anyway;
//! * a station with no credentials configured cannot sign anything at all, so
//!   this can never become a way in that the password was not.
//!
//! # What a cookie is
//!
//! A bearer token: whoever holds it is the operator, for its lifetime. That is
//! the trade the 12 and 24 hour dwells make explicit, and why both are offered
//! rather than a silent default. Revocation is expiry or a password change —
//! there is no per-device list and no "sign out everywhere", because that
//! needs a server-side session store and this is a single-operator station.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use sdroxide_types::RemoteAccess;

use crate::auth::AuthGate;

/// The cookie's name. Long and specific enough that it cannot collide with
/// anything the reverse proxy or another app on the same host is using.
pub(crate) const COOKIE_NAME: &str = "sdroxide_session";

/// The two dwells the sign-in card offers, in seconds: twelve hours and a day.
pub(crate) const DWELL_12H: u64 = 12 * 3600;
pub(crate) const DWELL_24H: u64 = 24 * 3600;

/// Anything else is refused rather than rounded, so a hand-edited cookie
/// cannot buy itself a longer life than the card offers.
fn clamp_dwell(hours: u32) -> u64 {
    match hours {
        0 | 1..=12 => DWELL_12H,
        _ => DWELL_24H,
    }
}

/// The dwell as whole hours, which is what the card and the reply both talk
/// in. Same clamp: the two dwells offered are the two dwells possible.
pub(crate) fn dwell_hours(hours: u32) -> u32 {
    (clamp_dwell(hours) / 3600) as u32
}

/// Now, in seconds since the epoch. Its own function so the tests can talk
/// about expiry without a clock they cannot move.
fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The signing key for a station: its own configured credentials, and nothing
/// else. `None` when the station asks for no password, which is also the only
/// case in which a cookie can never be minted.
pub(crate) fn key(access: Option<&RemoteAccess>) -> Option<String> {
    let want = access?;
    want.is_enforced().then(|| format!("{}\u{1}{}", want.username, want.password))
}

/// A cookie value for `username`, good for `hours`, signed with `key`.
pub(crate) fn mint(key: &str, username: &str, hours: u32) -> String {
    let expires = now_unix() + clamp_dwell(hours);
    // The payload is hex rather than joined with a separator: a username is
    // the operator's own text and may contain anything at all, and a token
    // whose own framing the caller controls is a token that can be
    // re-framed into somebody else's.
    let payload = hex(username.as_bytes());
    let body = format!("{payload}:{expires}");
    let mac = hex(&hmac(key.as_bytes(), body.as_bytes()));
    format!("{body}:{mac}")
}

/// The username a cookie says, if it is one this station minted, has not
/// expired, and still matches the credentials now configured.
pub(crate) fn verify(key: &str, token: &str, now: u64) -> Option<String> {
    let mut parts = token.split(':');
    let (payload, expires, mac) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let expires: u64 = expires.parse().ok()?;
    if expires <= now {
        return None;
    }
    let want = hex(&hmac(key.as_bytes(), format!("{payload}:{expires}").as_bytes()));
    // Constant time: this is a MAC comparison, and a byte-at-a-time early exit
    // is a timing oracle on the thing that authenticates every connection.
    if !bool::from(want.as_bytes().ct_eq(mac.as_bytes())) {
        return None;
    }
    from_hex(payload)
}

/// Whether these request headers carry a cookie this station accepts.
///
/// `None` — "no cookie worth the name" — is the common case and must not cost
/// anything: it is a header map that usually has no `Cookie` in it at all.
pub(crate) fn from_headers(headers: &HeaderMap, gate: &AuthGate) -> Option<String> {
    let cookies = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    let raw = cookies.split(';').filter_map(|c| c.trim().split_once('=')).find(|(k, _)| *k == COOKIE_NAME)?.1;
    let key = key(gate.required().as_ref())?;
    verify(&key, raw.trim(), now_unix())
}

/// The `Set-Cookie` value that hands the browser this token.
///
/// `secure` is the caller's read of whether the request arrived over TLS,
/// because a `Secure` cookie sent over plain HTTP is not sent at all — and a
/// station on a LAN with no reverse proxy is a real configuration, not a
/// mistake.
pub(crate) fn set_cookie(token: &str, hours: u32, secure: bool) -> String {
    let mut out = format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}", clamp_dwell(hours));
    if secure {
        out.push_str("; Secure");
    }
    out
}

/// The `Set-Cookie` value that takes it away again.
///
/// `HttpOnly` means the page cannot clear a cookie it is not allowed to read,
/// so signing out has to be something the station answers — otherwise a shared
/// browser would keep the operator signed in until the dwell ran out, with no
/// button on it that works.
///
/// Written out rather than routed through [`set_cookie`] on purpose: that one
/// clamps `hours` to a dwell, and `0` clamps to the *twelve-hour* one, so
/// signing out this way would have handed back a cookie that lives for half a
/// day. A sign-out that signs you in for twelve hours is the kind of bug that
/// is only ever caught by a test asking what the string actually says.
pub(crate) fn clear_cookie(secure: bool) -> String {
    let mut out = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if secure {
        out.push_str("; Secure");
    }
    out
}

/// Whether this request came in over TLS, as far as the station can tell.
///
/// A reverse proxy terminates TLS itself, so `X-Forwarded-Proto` is the honest
/// answer in front of one; the scheme is the fallback when there is not one.
pub(crate) fn arrived_secure(headers: &HeaderMap) -> bool {
    if let Some(proto) = headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()) {
        return proto.split(',').next().unwrap_or(proto).trim() == "https";
    }
    false
}

// --- the MAC ------------------------------------------------------------

/// HMAC-SHA256 over the bytes, on a 64-byte block.
///
/// Written out rather than pulled in as a dependency because it is eight lines
/// against `sha2`, which the tree already ships through rustls, and because the
/// construction is fixed by RFC 2104: two nested hashes, the inner under an
/// ipad and the outer under an opad. There is nothing here to get wrong and
/// nothing to configure.
fn hmac(key: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let (mut ipad, mut opad) = ([0x36u8; BLOCK], [0x5cu8; BLOCK]);
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).expect("nibble"));
        s.push(char::from_digit((b & 0xf) as u32, 16).expect("nibble"));
    }
    s
}

fn from_hex(s: &str) -> Option<String> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = s
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect();
    String::from_utf8(bytes?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "f6kim\u{1}hunter2";

    /// The round trip the whole feature rests on: a token this station minted
    /// says who, and only to that station.
    #[test]
    fn a_minted_cookie_names_its_owner() {
        let token = mint(KEY, "f6kim", 12);
        assert_eq!(verify(KEY, &token, now_unix()).as_deref(), Some("f6kim"));
    }

    /// The trade this feature makes is bounded time, so expiry is the property
    /// that has to hold — a cookie that outlives its dwell is not a session
    /// cookie, it is a password.
    ///
    /// The edge is inclusive on the dead side: at the expiry second it is
    /// already gone, which is what `Max-Age` means and the only reading that
    /// gives the operator the twelve hours they were promised rather than
    /// twelve hours and one second.
    #[test]
    fn an_expired_cookie_is_refused() {
        let token = mint(KEY, "f6kim", 12);
        let born = now_unix();
        let lifetime = DWELL_12H;
        assert!(verify(KEY, &token, born + lifetime - 1).is_some(), "good the second before");
        assert!(verify(KEY, &token, born + lifetime).is_none(), "good on the second");
        assert!(verify(KEY, &token, born + lifetime + 1).is_none(), "good past it");
    }

    /// Changing the password must invalidate every cookie at once, which is
    /// what deriving the key from the credentials buys. Nothing else in the
    /// tree can do this without a secret to keep in step.
    #[test]
    fn another_password_refuses_the_cookie() {
        let token = mint(KEY, "f6kim", 24);
        assert!(verify("f6kim\u{1}hunter3", &token, now_unix()).is_none());
    }

    /// Tampering has to fail the MAC rather than parse: the username is hex
    /// inside the signed payload, so a hand-edited token cannot re-frame
    /// itself into somebody else, and a mangled one is not a panic.
    #[test]
    fn a_tampered_cookie_is_refused() {
        let token = mint(KEY, "f6kim", 12);
        let body = token.split_once(':').unwrap();
        let payload = body.0;
        let (expires, mac) = body.1.split_once(':').unwrap();
        for bad in [
            format!("{payload}:{expires}"),
            format!("{payload}:{}", u64::MAX),
            format!("{}:{expires}:{mac}", hex(b"someone")),
            format!("{payload}:{expires}:{}", "0".repeat(64)),
            format!("{payload}:{expires}:{mac}:extra"),
            String::new(),
            "::".into(),
            "zz:12:ff".into(),
        ] {
            assert!(verify(KEY, &bad, now_unix()).is_none(), "accepted {bad}");
        }
    }

    /// The dwell is the operator's choice and the ceiling is the choice, not a
    /// hint: an hour and twelve hours are the same cookie, and anything past
    /// twelve is the day.
    #[test]
    fn the_dwell_is_the_card_and_nothing_longer() {
        let twelve = mint(KEY, "f6kim", 12);
        assert_eq!(twelve.split(':').nth(1).unwrap().parse::<u64>().unwrap() - now_unix(), DWELL_12H);
        for h in [0, 1, 6] {
            let short = mint(KEY, "f6kim", h);
            assert_eq!(
                short.split(':').nth(1).unwrap().parse::<u64>().unwrap() - now_unix(),
                DWELL_12H,
                "an hour asked for the short dwell"
            );
        }
        let day = mint(KEY, "f6kim", 24);
        assert_eq!(day.split(':').nth(1).unwrap().parse::<u64>().unwrap() - now_unix(), DWELL_24H);
        // The one that would actually matter: longer than the card offers.
        let greedy = mint(KEY, "f6kim", 720);
        assert_eq!(greedy.split(':').nth(1).unwrap().parse::<u64>().unwrap() - now_unix(), DWELL_24H);
    }

    /// A station that asks for no password must not be able to mint one — that
    /// is what stops this from ever being a way in the password was not.
    #[test]
    fn an_open_station_has_no_key() {
        assert!(key(None).is_none());
        assert!(key(Some(&RemoteAccess { username: String::new(), password: String::new() })).is_none());
    }

    /// HttpOnly is the point of the thing: without it the page's own script can
    /// read the operator's session, which is strictly worse than the
    /// localStorage password it replaces.
    #[test]
    fn the_cookie_is_http_only_and_takes_the_path() {
        let set = set_cookie("tok", 12, false);
        assert!(set.contains("HttpOnly"), "{set}");
        assert!(set.contains("Path=/"), "{set}");
        assert!(set.contains("SameSite=Strict"), "{set}");
        assert!(!set.contains("Secure"), "plain HTTP must still work: {set}");
        assert!(set_cookie("tok", 12, true).contains("Secure"));
        // And signing out is the only way to take it away.
        assert!(clear_cookie(false).contains("Max-Age=0"));
    }
}