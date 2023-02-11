use std::process::Child;

/// A wrapper around [`std::process::Child`] containing a spawned FFmpeg command.
/// Provides interfaces for reading parsed metadata, progress updates, warnings and errors, and
/// piped output frames if applicable.
pub struct FfmpegChild {
  inner: Child,
}

impl FfmpegChild {
  pub(crate) fn from_inner(inner: Child) -> Self {
    Self { inner }
  }

  /// Escape hatch to access the inner `Child`.
  pub fn as_inner(&mut self) -> &Child {
    &self.inner
  }

  /// Escape hatch to mutably access the inner `Child`.
  pub fn as_inner_mut(&mut self) -> &mut Child {
    &mut self.inner
  }
}
