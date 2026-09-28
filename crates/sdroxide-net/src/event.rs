//! Internal channel vocabulary shared by the feed threads, the worker threads,
//! and the [`crate::SpotManager`].

use crossbeam_channel::Sender;
use sdroxide_types::{CallsignInfo, QsoRecord, Spot, SpotKind, UploadResult};

/// A non-spot result from a network worker, drained by the manager and turned
/// into a `RadioEvent` by the engine.
#[derive(Debug, Clone)]
pub enum NetEvent {
    /// The merged, de-duplicated, age-pruned spot set (manager → engine).
    Spots(Vec<Spot>),
    /// Live band-opening detections from the same feeds that produce
    /// [`NetEvent::Spots`]: band × continent paths whose recent activity has
    /// surged past its own 3-hour baseline. Replaced wholesale whenever the
    /// analysis result changes.
    BandOpenings(Vec<sdroxide_types::BandOpening>),
    /// Human-readable feed/connection status (`None` clears it).
    Status(Option<String>),
    /// A callsign-lookup result.
    Callsign(CallsignInfo),
    /// One QSO/target upload result.
    Upload(UploadResult),
    /// The outcome of checking one service's stored credentials. Distinct
    /// from [`NetEvent::Upload`] because nothing was logged: this is an
    /// answer about the account, not about a QSO.
    LoginTest(sdroxide_types::LoginTestResult),
    /// Parsed confirmation records downloaded from LoTW/eQSL.
    Confirmations(Vec<QsoRecord>),
    /// WSPR reception reports fetched from WSPRnet — normally reports of *our*
    /// own transmissions, which is the only feedback a beacon ever gets.
    ///
    /// Not folded into [`NetEvent::Spots`]: a WSPR report carries a power level,
    /// a drift and a reporter, and flattening it into a cluster-style
    /// [`Spot`] would discard exactly the fields that make it a measurement of a
    /// path rather than an invitation to work someone.
    WsprSpots(Vec<sdroxide_types::WsprSpot>),
    /// Paths a spotting network demonstrated between two *other* stations —
    /// today the Reverse Beacon Network's skimmers.
    ///
    /// Not [`NetEvent::Spots`] for the same reason `WsprSpots` is not: these
    /// are measurements rather than invitations, and there are thousands of
    /// them a minute. They go to the propagation field, which is the one
    /// consumer that wants every last one of them, and never to the spot list,
    /// which would be buried.
    PropPaths(Vec<sdroxide_types::PropPath>),
}

/// A feed thread's full current view of its own spots (replaces the prior set
/// for that kind in the manager's store).
pub type FeedBatch = (SpotKind, Vec<Spot>);

pub type FeedTx = Sender<FeedBatch>;
pub type EventTx = Sender<NetEvent>;
