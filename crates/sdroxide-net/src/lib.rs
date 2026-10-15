//! Native network-cockpit clients for sdroxide: DX-cluster telnet, POTA/SOTA
//! and PSK Reporter spot feeds, QRZ/HamQTH callsign lookup, and QSO upload /
//! QSL-confirmation download (eQSL, QRZ Logbook, HamQTH, Club Log, LoTW).
//!
//! NATIVE ONLY — this crate does blocking network I/O on its own threads and
//! must never be a dependency of any wasm-targeted crate. The GUI keeps its
//! threadless model: the [`SpotManager`] owns every feed/worker thread and the
//! engine only drains it (non-blocking) on its existing poll loop.

mod cluster;
mod event;
mod freedvreporter;
mod http;
mod lookup;
mod manager;
mod poll;
mod pota;
mod pskreporter;
mod pskupload;
mod rbn;
mod socketio;
mod sondehub;
mod sota;
mod upload;
mod wsprnet;

pub use event::NetEvent;
pub use manager::SpotManager;
pub use pskupload::MAX_REPORT_HZ as MAX_PSK_REPORT_HZ;
pub use pskupload::Report as PskReport;
pub use sondehub::{Item as SondeHubItem, Reporter as SondeHubReporter};
pub use wsprnet::{Item as WsprItem, Query as WsprQuery, Reporter as WsprReporter};

// On-demand blocking helpers, exposed for tests / direct use.
pub use freedvreporter::{ProbeStation, ProbeSummary, probe as freedv_reporter_probe};
pub use lookup::lookup as lookup_callsign;
pub use upload::{sync_confirmations, upload as upload_qso};
