use serde::{Deserialize, Serialize};

/// Built-in TCI server configuration (`tciserver.json`).
///
/// This is sdroxide *being* a TCI rig so third-party clients (WSJT-X's TCI rig
/// type, JTDX, MSHV, skimmers) can drive it. Not to be confused with
/// [`crate::TciConfig`], which is the client that connects sdroxide to somebody
/// else's rig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TciServerConfig {
    pub enabled: bool,
    /// Interface to bind. `127.0.0.1` keeps the server on this machine;
    /// `0.0.0.0` exposes it to the whole network — and TCI has no
    /// authentication, so anyone who can reach the port can transmit.
    pub bind: String,
    /// Listen port. 50001 is the TCI default that clients expect, and also the
    /// port a local ExpertSDR3/Thetis would already hold.
    pub port: u16,
    /// Name reported in the `device:` line of the connect burst.
    pub device_name: String,
    /// Let clients key the transmitter (`trx:<rx>,true,tci;`). Off leaves
    /// control and the receive streams working but refuses every key request.
    pub allow_tx: bool,
    pub max_clients: usize,
    /// Dial out as a TCI client and push this radio's VFO to another TCI
    /// server (590Ctrl) whenever the operator tunes. Control only: a `vfo:`
    /// line, no audio, no IQ, and it never keys. Independent of [`Self::enabled`],
    /// which is sdroxide being the server. Off so an existing `tciserver.json`
    /// keeps doing only what it did.
    pub client: bool,
    /// `host:port` of the TCI server to tune. `127.0.0.1:50001` is 590Ctrl's
    /// listen port (and ExpertSDR's). A bare host uses port 50001.
    pub client_address: String,
}

impl Default for TciServerConfig {
    fn default() -> Self {
        TciServerConfig {
            enabled: true,
            bind: "127.0.0.1".into(),
            port: 50001,
            device_name: "sdroxide".into(),
            allow_tx: true,
            max_clients: 8,
            client: false,
            client_address: "127.0.0.1:50001".into(),
        }
    }
}

impl TciServerConfig {
    /// Bind addresses offered in the UI.
    pub const BINDS: [&'static str; 2] = ["127.0.0.1", "0.0.0.0"];

    pub fn addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }

    /// True when this configuration hands the transmitter to the network at
    /// large, which deserves a warning in the UI.
    pub fn is_open_to_network(&self) -> bool {
        self.allow_tx && self.bind != "127.0.0.1" && self.bind != "localhost"
    }

    /// True when the outbound client would reach a server bound the way this
    /// config binds. The engine uses it only once this server is actually
    /// listening: a failed bind (590Ctrl already holds the port) must still
    /// let the client through to that other server.
    pub fn client_reaches_this_server(&self) -> bool {
        if !self.enabled || !self.client {
            return false;
        }
        let Some((host, port)) = parse_client_addr(&self.client_address) else {
            return false;
        };
        if port != self.port {
            return false;
        }
        let client_local = is_local_host(&host);
        let server_local = is_local_host(&self.bind);
        (client_local && server_local) || host.eq_ignore_ascii_case(self.bind.trim())
    }
}

/// `host[:port]`, `ws://host:port`, or a bare host (port 50001).
fn parse_client_addr(address: &str) -> Option<(String, u16)> {
    let a = address.trim();
    let a = a.strip_prefix("ws://").or_else(|| a.strip_prefix("wss://")).unwrap_or(a);
    let a = a.trim_end_matches('/');
    if a.is_empty() {
        return None;
    }
    match a.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port: u16 = port.parse().ok()?;
            Some((host.to_string(), port))
        }
        _ => Some((a.to_string(), 50001)),
    }
}

fn is_local_host(host: &str) -> bool {
    let h = host.trim().trim_matches(['[', ']']);
    matches!(h, "127.0.0.1" | "localhost" | "::1" | "0.0.0.0" | "::" | "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_server_json_leaves_the_client_off() {
        let cfg: TciServerConfig = serde_json::from_str(
            r#"{"enabled":true,"bind":"127.0.0.1","port":50001,"device_name":"sdroxide","allow_tx":true,"max_clients":8}"#,
        )
        .unwrap();
        assert!(!cfg.client);
        assert_eq!(cfg.client_address, "127.0.0.1:50001");
        assert!(cfg.enabled);
    }

    #[test]
    fn client_reaches_this_server_only_when_both_are_on_and_the_port_matches() {
        let mut cfg = TciServerConfig::default();
        cfg.client = true;
        assert!(cfg.client_reaches_this_server());
        cfg.client_address = "localhost:50001".into();
        assert!(cfg.client_reaches_this_server());
        cfg.bind = "0.0.0.0".into();
        assert!(cfg.client_reaches_this_server());
        cfg.client_address = "127.0.0.1:50002".into();
        assert!(!cfg.client_reaches_this_server());
        cfg.client_address = "127.0.0.1:50001".into();
        cfg.client = false;
        assert!(!cfg.client_reaches_this_server());
        cfg.client = true;
        cfg.enabled = false;
        assert!(!cfg.client_reaches_this_server());
        cfg.enabled = true;
        cfg.bind = "192.168.1.9".into();
        cfg.port = 50002;
        cfg.client_address = "192.168.1.9:50002".into();
        assert!(cfg.client_reaches_this_server());
        cfg.client_address = "192.168.1.10:50002".into();
        assert!(!cfg.client_reaches_this_server());
    }
}
