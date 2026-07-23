//! FSQ contacts list (address book) for directed messaging. Persisted natively
//! in `contacts.json`; edited from the FSQ panel.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FsqContact {
    /// Stable id (0 = unassigned; the UI assigns on add).
    pub id: u64,
    pub call: String,
    pub name: String,
    pub note: String,
}
