//! Received pictures to an FTP or FTPS server, the way QSSTV does it.
//!
//! The engine hands every SSTV picture (and, if asked, every weather chart)
//! to a [`PictureUploader`] as it is saved. The uploader's own thread makes a
//! copy fit for the web — QSSTV's caption bar, its 960 × 768 limit, PNG or
//! JPEG — and puts it on the server as `image1`, moving the earlier ones down
//! to `image2` … `imageN`, or under its own name. Beside each one goes QSSTV's
//! `checknew.jpg`, the heartbeat the web pages written for QSSTV poll.
//!
//! FTPS, explicit or implicit, runs on rustls; see [`tls`] for how the
//! server's certificate is judged.

mod ftp;
pub mod rotate;
pub mod stamp;
pub mod tls;
mod worker;

pub use stamp::Caption;
pub use worker::PictureUploader;
