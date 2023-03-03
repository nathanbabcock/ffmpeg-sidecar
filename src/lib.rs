//! Wrap a standalone FFmpeg binary in an intuitive Iterator interface.
//!
//! ## Example
//!
//! ```rust
//! use ffmpeg_sidecar::{
//!   child::FfmpegChild, command::FfmpegCommand, event::FfmpegEvent, iter::FfmpegIterator,
//! };
//!
//! // similar to `std::process::Command`
//! let mut command = FfmpegCommand::new();
//! command
//!   .testsrc() // generate a test pattern video
//!   .rawvideo(); // pipe raw video output
//!
//! // similar to `std::process::Child`
//! let mut child: FfmpegChild = command.spawn().unwrap();
//!
//! // Iterator over all messages and output
//! let iter: FfmpegIterator = child.iter().unwrap();
//! iter.for_each(|event: FfmpegEvent| {
//!   match event {
//!     FfmpegEvent::OutputFrame(frame) => {
//!       let _pixels = frame.data; // <- raw RGB pixels! 🎨
//!     }
//!     FfmpegEvent::Error(e) => eprintln!("Error: {}", e),
//!     _ => {}
//!   }
//! });
//! ```
//!

#[cfg(test)]
mod test;

pub mod auto_download;
pub mod child;
pub mod comma_iter;
pub mod command;
pub mod error;
pub mod event;
pub mod iter;
pub mod log_parser;
pub mod pix_fmt;
pub mod read_until_any;
