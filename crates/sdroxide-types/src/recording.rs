//! Scheduled recordings: "record 49 m at 18:00 UTC for half an hour".
//!
//! A job is a start time, a frequency, how long to record and what to capture.
//! The scheduler that acts on these lives in the interface (see the SWL fork's
//! `recording_jobs`), so the pure part — *when* a job starts and stops — is
//! here and can be tested without a radio or a clock.

use serde::{Deserialize, Serialize};

/// What a job captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingKind {
    /// Demodulated audio (a WAV beside the other recordings).
    #[default]
    Audio,
    /// The raw I/Q the radio delivers, replayable later.
    Iq,
    /// Both at once, which the engine already supports.
    Both,
}

impl RecordingKind {
    pub fn label(self) -> &'static str {
        match self {
            RecordingKind::Audio => "Audio",
            RecordingKind::Iq => "I/Q",
            RecordingKind::Both => "Both",
        }
    }
}

/// One scheduled recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordingJob {
    /// Stable id (0 = unassigned; the interface assigns on first store).
    pub id: u64,
    /// When to start, Unix seconds UTC.
    pub at_unix: u64,
    /// The frequency to tune to at the start.
    pub freq_hz: f64,
    pub mode: crate::Mode,
    /// How long to record, in seconds.
    pub duration_s: u32,
    pub kind: RecordingKind,
    /// The station, for the row and the report. Free text.
    pub name: String,
    /// Set once the job has run, so a one-shot does not fire twice.
    pub done: bool,
}

impl Default for RecordingJob {
    fn default() -> Self {
        RecordingJob {
            id: 0,
            at_unix: 0,
            freq_hz: 0.0,
            mode: crate::Mode::Am,
            duration_s: 30 * 60,
            kind: RecordingKind::Audio,
            name: String::new(),
            done: false,
        }
    }
}

/// What the scheduler should do with a job at a given instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAction {
    /// Start recording for this job.
    Start,
    /// Stop the recording this job started.
    Stop,
    /// Nothing to do.
    Idle,
}

impl RecordingJob {
    /// When the recording should end.
    pub fn end_unix(&self) -> u64 {
        self.at_unix.saturating_add(self.duration_s as u64)
    }

    /// What to do at `now`, given whether this job is the one currently
    /// recording.
    ///
    /// A finished job does nothing even if it is still "running", so a job that
    /// never got stopped (the program was closed mid-over) does not restart.
    pub fn action(&self, now: u64, running: bool) -> JobAction {
        if self.done && !running {
            return JobAction::Idle;
        }
        if now >= self.end_unix() {
            return if running { JobAction::Stop } else { JobAction::Idle };
        }
        if now >= self.at_unix && !running {
            return JobAction::Start;
        }
        JobAction::Idle
    }

    /// UTC as a listener reads it, e.g. `2026-09-16 18:00 UTC`.
    pub fn utc_text(&self) -> String {
        let (y, mo, d, h, mi, _s) = crate::utc_ymd_hms(self.at_unix as i64);
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(at: u64, dur: u32) -> RecordingJob {
        RecordingJob { at_unix: at, duration_s: dur, ..Default::default() }
    }

    #[test]
    fn a_job_starts_at_its_time_and_stops_at_its_end() {
        let j = job(1000, 60);
        assert_eq!(j.action(999, false), JobAction::Idle, "too early");
        assert_eq!(j.action(1000, false), JobAction::Start, "at the start");
        assert_eq!(j.action(1030, true), JobAction::Idle, "running");
        assert_eq!(j.action(1060, true), JobAction::Stop, "at the end");
        assert_eq!(j.action(2000, false), JobAction::Idle, "long after");
    }

    #[test]
    fn a_finished_job_never_restarts() {
        let mut j = job(1000, 60);
        j.done = true;
        assert_eq!(j.action(1030, false), JobAction::Idle, "done is done");
    }

    #[test]
    fn end_is_start_plus_duration() {
        assert_eq!(job(1000, 1800).end_unix(), 2800);
    }

    #[test]
    fn a_job_round_trips_through_json() {
        let j = RecordingJob {
            id: 3,
            at_unix: 1_789_587_720,
            freq_hz: 6_185_000.0,
            mode: crate::Mode::Am,
            duration_s: 1800,
            kind: RecordingKind::Both,
            name: "BBC World Service".into(),
            done: false,
        };
        let text = serde_json::to_string(&j).unwrap();
        assert_eq!(serde_json::from_str::<RecordingJob>(&text).unwrap(), j);
    }

    #[test]
    fn an_older_job_still_loads() {
        let j: RecordingJob =
            serde_json::from_str(r#"{"at_unix":1000,"freq_hz":6185000.0}"#).unwrap();
        assert_eq!(j.kind, RecordingKind::Audio, "a missing kind defaults to audio");
        assert_eq!(j.duration_s, 1800, "and a missing duration to half an hour");
    }
}
