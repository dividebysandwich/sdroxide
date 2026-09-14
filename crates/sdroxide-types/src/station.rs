//! The persisted configuration that belongs to the *station* rather than to
//! the screen in front of it.
//!
//! Seven files, all written next to each other in the config directory and all
//! owned by the engine: the network cockpit (`net.json`), the two built-in
//! control servers (`rigctld.json`, `tciserver.json`), the WSJT-X UDP
//! broadcast (`wsjtx.json`), the operator's satellite additions
//! (`satellites.json`), the antenna-rotator client (`rotator.json`) and the
//! external transmit/receive switch (`relay.json`).
//!
//! They are bundled into one announcement because they answer one question —
//! "what is this station set up to do?" — and because a remote client has to
//! be *told*: it has no access to the machine the engine runs on, so a settings
//! dialog there would otherwise open on defaults and, worse, could write those
//! defaults back over the operator's real configuration. The engine emits this
//! once at startup and again after every change, so the answer is always the
//! current one, whichever screen asked.
//!
//! What is *not* here: anything describing the operator's own desk. The
//! control-input bindings and the UI settings stay client-side, because a knob
//! on this table has nothing to do with the radio in the other room.

use serde::{Deserialize, Serialize};

use crate::{
    NetworkConfig, Region, RelayConfig, RigctldConfig, RotatorConfig, SatConfig, TciServerConfig,
    WsjtxConfig,
};

/// Everything the engine host persists on the station's behalf, as one
/// snapshot. See the module docs for why these travel together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StationConfig {
    /// Spot feeds, callsign lookup, uploads and the credentials they need.
    pub net: NetworkConfig,
    /// The built-in Hamlib rigctld listener.
    pub rigctld: RigctldConfig,
    /// The built-in TCI listener.
    pub tci_server: TciServerConfig,
    /// The WSJT-X UDP broadcast.
    pub wsjtx: WsjtxConfig,
    /// Element sets the operator added, the listings they subscribe to, and
    /// their satellite frequency overrides.
    pub sat: SatConfig,
    /// The rotctld client a satellite lock steers the antenna through.
    pub rotator: RotatorConfig,
    /// The ITU / IARU region the station is in, from `config.toml`.
    ///
    /// The odd one out in this bundle — it is a line in `config.toml` rather
    /// than a file of its own — but it belongs here for the same reason as the
    /// rest: it is a fact about the machine the antenna is attached to, and a
    /// remote client that guessed it from its own disk would draw a band plan
    /// for the wrong continent and warn about the wrong band edges.
    ///
    /// Every client adopts this into [`crate::set_region`] as it arrives, so
    /// the band buttons, the waterfall's band strip and the frequency displays
    /// all agree with the station they are operating.
    pub region: Region,
    /// Which country's CB channels the station works — the same reasoning as
    /// the region, and station-wide for the same reason: 11 m is not the same
    /// band everywhere, and the channel numbers an operator reads belong to
    /// where the antenna is. Every client adopts it into [`crate::set_cb_plan`].
    pub cb_plan: crate::CbPlan,
    /// The band plan itself — `bandplan.json` on the engine's machine, or the
    /// built-in IARU tables where there is no file.
    ///
    /// Travels for the same reason the region does, and more urgently: an
    /// operator who has narrowed 2 m to their licence's edges has a station
    /// that will refuse to key above them, and a client drawing the shipped
    /// plan would show a band button that does nothing and a waterfall strip
    /// that disagrees with the radio. The largest field in this bundle by some
    /// way, but the bundle is sent on connect and on change, not in a loop.
    pub band_plan: crate::BandPlan,
    /// The external T/R switch: the relay board or contact closure that gets
    /// the SDR out of the way while the station transmits.
    ///
    /// Here rather than in `radio.json` because it is one box for the whole
    /// station, in front of whatever this program happens to be receiving with
    /// — the same argument the LimeRFE makes for being its own crate. On a
    /// multi-radio station every engine reports whether it is on the air and
    /// the switch follows all of them.
    ///
    /// Appended last, for the usual reason.
    pub relay: RelayConfig,
    /// The frequencies the operator has added to the digital modes' own tables
    /// — `digi_presets.json` on the engine's machine (issue #268).
    ///
    /// Travels for the same reason the band plan does: a remote client draws
    /// the same **⇵** picker as the console, and a list held on the operator's
    /// laptop would be one the station never offers and one a second screen
    /// could silently write over.
    ///
    /// Appended last, for the usual reason.
    pub digi_presets: Vec<crate::DigiPreset>,
}
