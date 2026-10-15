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
pub fn post_json_status(url: &str, bearer: &str, json: &str) -> Result<(u16, String), String> {
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

/// PUT `json` as `application/json` (SondeHub listener metadata, etc.).
pub fn put_json(url: &str, json: &str) -> Result<String, String> {
    body_or_status(
        agent()
            .put(url)
            .header("Content-Type", "application/json")
            // SondeHub `/listeners` and `/sondes/telemetry` require RFC7231 Date
            // so they can correct client clock skew.
            .header("Date", &http_date_now())
            .send(json)
            .map_err(|e| e.to_string())?,
    )
}

/// PUT gzip-compressed JSON with `Content-Encoding: gzip` (SondeHub telemetry).
///
/// Returns `(http_status, body)`. Non-2xx is still returned so callers can
/// retry 5xx the way OpenWebRX's SondeHub reporter does. A 202 with an
/// `errors` array in the body still counts as HTTP success here — callers
/// must inspect the body (SondeHub rejects unknown `software_name` that way).
pub fn put_gzip_json(url: &str, json: &str) -> Result<(u16, String), String> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    let body = enc.finish().map_err(|e| e.to_string())?;
    let mut resp = agent()
        .put(url)
        .header("Content-Type", "application/json")
        .header("Content-Encoding", "gzip")
        .header("Date", &http_date_now())
        .send(body)
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((status, text))
}

/// `Date: Thu, 01 Jan 1970 00:00:00 GMT` for SondeHub's required header.
fn http_date_now() -> String {
    const DOW: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MON: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date from Unix days (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let tod = secs % 86_400;
    let h = tod / 3600;
    let mi = (tod % 3600) / 60;
    let s = tod % 60;
    // 1970-01-01 was Thursday; day-of-week from Unix days.
    let dow = DOW[((secs / 86_400 + 4) % 7) as usize];
    format!(
        "{dow}, {d:02} {} {y:04} {h:02}:{mi:02}:{s:02} GMT",
        MON[(m as usize).saturating_sub(1).min(11)]
    )
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
