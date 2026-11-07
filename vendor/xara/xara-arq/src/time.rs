pub type Time = i64;

pub const SECOND: Time = 48_000;
pub(crate) const IDLE_POLL: Time = 10 * SECOND;
pub(crate) const IDLE_POLL_LIMIT: u32 = 7;
pub(crate) const DISC_TIMEOUT_LIMIT: u32 = 3;
