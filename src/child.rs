use std::{
  io::{self, Write},
  process::{Child, ChildStdin},
};

/// A wrapper around [`std::process::Child`] containing a spawned FFmpeg command.
/// Provides interfaces for reading parsed metadata, progress updates, warnings and errors, and
/// piped output frames if applicable.
pub struct FfmpegChild {
  inner: Child,
  stdin: ChildStdin,
}

impl FfmpegChild {
  /// Send a command to ffmpeg over stdin, used during interactive mode.
  ///
  /// This method does not validate that the command is expected or handled
  /// correctly by ffmpeg. The returned `io::Result` indicates only whether the
  /// command was successfully sent or not.
  ///
  /// In a typical ffmpeg build, these are the supported commands:
  ///
  /// ```txt
  /// ?      show this help
  /// +      increase verbosity
  /// -      decrease verbosity
  /// c      Send command to first matching filter supporting it
  /// C      Send/Queue command to all matching filters
  /// D      cycle through available debug modes
  /// h      dump packets/hex press to cycle through the 3 states
  /// q      quit
  /// s      Show QP histogram
  /// ```
  pub fn send_stdin_command(&mut self, command: &[u8]) -> io::Result<()> {
    self.stdin.write_all(command)
  }

  /// Send a `q` command to ffmpeg over stdin,
  /// requesting a graceful shutdown as soon as possible.
  ///
  /// This method returns after the command has been sent; the actual shut down
  /// may take a few more frames as ffmpeg flushes its buffers and writes the
  /// trailer, if applicable.
  pub fn quit(&mut self) -> io::Result<()> {
    self.send_stdin_command(b"q")
  }

  /// Forcibly terminate the inner child process.
  ///
  /// Alternatively, you may choose to gracefully stop the child process by
  /// sending a command over stdin, using the `quit` method.
  ///
  /// Identical to `kill` in [`std::process::Child`].
  pub fn kill(&mut self) -> io::Result<()> {
    self.inner.kill()
  }

  /// Wrap a [`std::process::Child`] in a `FfmpegChild`.
  /// Should typically only be called by `FfmpegCommand::spawn`.
  ///
  /// ## Panics
  ///
  /// Panics if the child process's stdin was not piped. This could be because
  /// ffmpeg was spawned with `-nostdin`, or if the `Child` instance was not
  /// configured with `stdin(Stdio::piped())`.
  pub(crate) fn from_inner(mut inner: Child) -> Self {
    let stdin = inner.stdin.take().expect("Child stdin was not piped");
    Self { inner, stdin }
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
