//! `POST /signin` and `POST /signout`: where a browser trade its password for
//! a signed cookie, and where they hand it back.
//!
//! # Why these are HTTP and not a socket message
//!
//! A cookie is set with a response header, and the WebSocket handshake's
//! response headers are gone before the sign-in conversation on the socket has
//! even started — axum completes the upgrade to hand back a socket, so by the
//! time [`crate::auth::challenge`] has read a password there is no response
//! left to attach a `Set-Cookie` to. So the answer is given once over plain
//! HTTP, and every connection after it — every radio, every reload, the solar
//! feed — carries the cookie on its own upgrade and is let straight in.
//!
//! That is also why this costs no protocol version: nothing on the postcard
//! socket changes, and [`crate::session_cookie`] is entirely an HTTP-header
//! affair.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tracing::info;

use sdroxide_types::{AUTH_BUSY, AUTH_REFUSED};

use crate::{Station, auth, session_cookie};

/// What a browser posts: the credentials, and how long it would like them
/// remembered for.
#[derive(Deserialize)]
pub(crate) struct SignIn {
    pub username: String,
    pub password: String,
    /// Hours, so the card can say "12" or "24" rather than a duration in
    /// seconds an operator has to translate. Anything else is clamped to the
    /// nearer of the two the card offers.
    #[serde(default)]
    pub hours: u32,
}

#[derive(Serialize)]
struct SignedIn {
    username: String,
    hours: u32,
}

/// Hand back a cookie for these credentials, if they are the right ones.
///
/// Judged by the same gate and the same turnstile as the socket sign-in, so
/// this cannot become a second, easier way to guess: one answer at a time
/// server-wide, three seconds of lockout after a wrong one, and a station that
/// is merely busy is told to come back rather than being told it was wrong.
pub(crate) async fn signin(
    State(station): State<Arc<Station>>,
    headers: HeaderMap,
    Json(want): Json<SignIn>,
) -> Response {
    let Some(key) = session_cookie::key(station.auth.required().as_ref()) else {
        // No password is configured, so there is nothing to remember and
        // nothing to sign. Saying so is better than handing out a cookie for
        // a station that never asked.
        return (
            axum::http::StatusCode::CONFLICT,
            "this station asks no password, so it has no sign-in to remember",
        )
            .into_response();
    };

    let verdict = station.auth.check(&want.username, &want.password).await;
    match verdict {
        auth::Verdict::Ok => {}
        auth::Verdict::Wrong => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                [("content-type", "text/plain")],
                AUTH_REFUSED,
            )
                .into_response();
        }
        auth::Verdict::Busy => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                [("content-type", "text/plain")],
                format!("{AUTH_BUSY}; try again in a moment"),
            )
                .into_response();
        }
    }

    let hours = session_cookie::dwell_hours(want.hours);
    let token = session_cookie::mint(&key, &want.username, hours);
    let cookie = session_cookie::set_cookie(&token, hours, session_cookie::arrived_secure(&headers));
    info!("signed in as {:?} for {hours}h", want.username);
    (
        [(axum::http::header::SET_COOKIE, cookie)],
        Json(SignedIn { username: want.username, hours }),
    )
        .into_response()
}

/// Take the cookie away.
///
/// `HttpOnly` means the page cannot clear a cookie it is not allowed to read,
/// so without this a shared browser keeps the operator signed in until the
/// dwell runs out and the button on it that should end it does nothing — the
/// fork's own rule, a control that cannot do the thing must not be offered.
pub(crate) async fn signout(headers: HeaderMap) -> Response {
    (
        [(
            axum::http::header::SET_COOKIE,
            session_cookie::clear_cookie(session_cookie::arrived_secure(&headers)),
        )],
        Json(SignedIn { username: String::new(), hours: 0 }),
    )
        .into_response()
}