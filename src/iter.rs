use std::sync::mpsc::Receiver;

use crate::event::FfmpegEvent;

/// An iterator over events from an ffmpeg process, including parsed metadata, progress, and raw video frames.
pub struct FfmpegIterator {
  rx: Receiver<FfmpegEvent>,
}

impl FfmpegIterator {
  pub fn new(rx: Receiver<FfmpegEvent>) -> Self {
    Self { rx }
  }
}

impl Iterator for FfmpegIterator {
  type Item = FfmpegEvent;

  fn next(&mut self) -> Option<Self::Item> {
    self.rx.recv().ok()
  }
}
