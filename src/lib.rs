#![cfg_attr(docsrs, feature(doc_cfg))]

//! Wrap a standalone FFmpeg binary in an intuitive Iterator interface.
//!
//! ## Example
//!
//! ```rust
//! use ffmpeg_sidecar::command::FfmpegCommand;
//! fn main() -> anyhow::Result<()> {
//!   // Run an FFmpeg command that generates a test video
//!   let iter = FfmpegCommand::new() // <- Builder API like `std::process::Command`
//!     .testsrc()  // <- Discoverable aliases for FFmpeg args
//!     .rawvideo() // <- Convenient argument presets
//!     .spawn()?   // <- Ordinary `std::process::Child`
//!     .iter()?;   // <- Blocking iterator over logs and output
//!
//!   // Use a regular "for" loop to read decoded video data
//!   for frame in iter.filter_frames() {
//!     println!("frame: {}x{}", frame.width, frame.height);
//!     let _pixels: Vec<u8> = frame.data; // <- raw RGB pixels! 🎨
//!   }
//!
//!   Ok(())
//! }
//! ```

#[cfg(test)]
mod test;

pub mod child;
pub mod comma_iter;
pub mod command;
pub mod download;
pub mod event;
pub mod ffprobe;
pub mod iter;
pub mod log_parser;
pub mod metadata;
pub mod paths;
pub mod pix_fmt;
pub mod read_until_any;
pub mod version;

#[cfg(feature = "named_pipes")]
#[cfg_attr(docsrs, doc(cfg(feature = "named_pipes")))]
pub mod named_pipes;

pub use anyhow::Result;
