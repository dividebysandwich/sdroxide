//! Small blocking HTTP(S) helpers over `ureq` (rustls, pure-Rust TLS). Every
//! call is on a worker thread, so blocking is fine.

use std::time::Duration;

/// Shared agent with sane timeouts. Cheap to build per call.
///
/// A non-2xx response is deliberately *not* turned into an error here: the
/// callers report the server's own body on failure (many APIs explain the
/// rejection in it), and that body is only reachable while the status is still
/// carried by a response rather than by `Error::StatusCode`.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// GET `url`, returning the response body as a string.
pub fn get(url: &str) -> Result<String, String> {
    body_or_status(agent().get(url).call().map_err(|e| e.to_string())?)
}

/// POST `fields` as `application/x-www-form-urlencoded`.
pub fn post_form(url: &str, fields: &[(&str, &str)]) -> Result<String, String> {
    body_or_status(agent().post(url).send_form(fields.iter().copied()).map_err(|e| e.to_string())?)
}

/// POST `fields` as `application/x-www-form-urlencoded`, keeping the HTTP
/// status alongside the body instead of folding a non-2xx into an error.
///
/// For a service whose whole answer *is* the status code — HamQTH's real-time
/// logbook says 200/400/403/500 and puts only a short sentence in the body —
/// "wrong password" and "duplicate QSO" are the same string to
/// [`post_form`]'s caller. Here they are still two different numbers.
pub fn post_form_status(url: &str, fields: &[(&str, &str)]) -> Result<(u16, String), String> {
    let mut resp =
        agent().post(url).send_form(fields.iter().copied()).map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// POST `json` as `application/json` with a bearer token, keeping the HTTP
/// status alongside the body.
///
/// For a JSON API whose refusals are a status plus a machine-readable code in
/// the body — the World Radio League's is one — where "the key is wrong" and
/// "that contact is already logged" have to be told apart and reported
/// differently. See [`post_form_status`], which is this for the form-encoded
/// services.
pub fn post_json_status(
    url: &str,
    bearer: &str,
    json: &str,
) -> Result<(u16, String), String> {
    let mut resp = agent()
        .post(url)
        .header("Authorization", &format!("Bearer {bearer}"))
        .header("Content-Type", "application/json")
        .send(json)
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// GET `url` with a bearer token, keeping the HTTP status alongside the body.
pub fn get_bearer_status(url: &str, bearer: &str) -> Result<(u16, String), String> {
    let mut resp = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {bearer}"))
        .call()
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// The body as a string, or a compact message for an HTTP status error that
/// keeps the server's own body so the caller can report why it was rejected.
fn body_or_status(mut resp: ureq::http::Response<ureq::Body>) -> Result<String, String> {
    let status = resp.status();
    // Same 10 MB cap `read_to_string` applies by default.
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    if status.is_success() {
        return Ok(body);
    }
    let code = status.as_u16();
    let body = body.trim();
    if body.is_empty() {
        Err(format!("HTTP {code}"))
    } else {
        Err(format!("HTTP {code}: {}", body.chars().take(200).collect::<String>()))
    }
}
