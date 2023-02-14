use std::{
  ffi::OsStr,
  fmt, io,
  process::{Command, CommandArgs, Stdio},
};

use crate::child::FfmpegChild;

/// A wrapper around [`std::process::Command`] with some convenient preset argument
/// sets and customization for ffmpeg specifically.
pub struct FfmpegCommand {
  inner: Command,
}

impl FfmpegCommand {
  //// Argument presets

  /// Generate a procedural test video.
  /// Equivalent to `ffmpeg -i lavfi -f testsrc`
  ///
  /// [FFmpeg `testsrc` filter documentation](https://ffmpeg.org/ffmpeg-filters.html#allrgb_002c-allyuv_002c-color_002c-colorchart_002c-colorspectrum_002c-haldclutsrc_002c-nullsrc_002c-pal75bars_002c-pal100bars_002c-rgbtestsrc_002c-smptebars_002c-smptehdbars_002c-testsrc_002c-testsrc2_002c-yuvtestsrc)
  pub fn testsrc(&mut self) -> &mut Self {
    self.args(&["-f", "lavfi", "-i", "testsrc"]);
    self
  }

  /// Configure the ffmpeg command to produce output on stdout.
  ///
  /// Synchronizes two changes:
  /// 1. Pass `pipe:1` to the ffmpeg command ("output on stdout")
  /// 2. Set the `stdout` field of the inner `Command` to `Stdio::piped()`
  pub fn pipe_stdout(&mut self) -> &mut Self {
    self.args(&["-"]);
    self.inner.stdout(Stdio::piped());
    self
  }

  /// Automatically applied in the constructor of `FfmpegCommand`.
  /// Configures logging with a level and format expected by the log parser.
  ///
  /// Equivalent to `ffmpeg -loglevel level+info`.
  ///
  /// The `level` flag adds a prefix to all log messages with the log level in square brackets,
  /// allowing the parser to distinguish between ambiguous messages like
  /// warnings vs errors.
  ///
  /// The `+info` flag enables the `info` log level, which is the default level.
  ///
  /// If this settings is manually overridden, the log parser should still work,
  /// but lose some semantic distinction between log levels.
  fn set_expected_loglevel(&mut self) -> &mut Self {
    self.args(&["-loglevel", "level+info"]);
    self
  }

  //// `std::process::Command` passthrough methods

  /// Adds an argument to pass to the program.
  ///
  /// Identical to `arg` in [`std::process::Command`].
  pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
    self.inner.arg(arg.as_ref());
    self
  }

  /// Adds multiple arguments to pass to the program.
  ///
  /// Identical to `args` in [`std::process::Command`].
  pub fn args<I, S>(&mut self, args: I) -> &mut Self
  where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
  {
    for arg in args {
      self.arg(arg.as_ref());
    }
    self
  }

  /// Returns an iterator of the arguments that will be passed to the program.
  ///
  /// Identical to `get_args` in [`std::process::Command`].
  pub fn get_args(&self) -> CommandArgs<'_> {
    self.inner.get_args()
  }

  /// Spawn the ffmpeg command as a child process, wrapping it in a
  /// `FfmpegChild` interface.
  ///
  /// Identical to `spawn` in [`std::process::Command`].
  pub fn spawn(&mut self) -> io::Result<FfmpegChild> {
    self.inner.spawn().map(FfmpegChild::from_inner)
  }

  pub fn new() -> Self {
    Self::new_with_exe("ffmpeg")
  }

  pub fn new_with_exe<S: AsRef<OsStr>>(exe: S) -> Self {
    // Configure `Command`
    let mut inner = Command::new(&exe);
    inner.stdin(Stdio::piped());
    inner.stderr(Stdio::piped());
    inner.stdout(Stdio::piped());

    // Configure `FfmpegCommand`
    let mut ffmpeg_command = Self { inner };
    ffmpeg_command.set_expected_loglevel();
    ffmpeg_command
  }

  /// Escape hatch to access the inner `Command`.
  pub fn as_inner(&mut self) -> &Command {
    &self.inner
  }

  /// Escape hatch to mutably access the inner `Command`.
  pub fn as_inner_mut(&mut self) -> &mut Command {
    &mut self.inner
  }
}

impl Default for FfmpegCommand {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Debug for FfmpegCommand {
  /// Format the program and arguments of a Command for display. Any
  /// non-utf8 data is lossily converted using the utf8 replacement
  /// character.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.inner.fmt(f)
  }
}

impl From<Command> for FfmpegCommand {
  /// Convert a `Command` into a `FfmpegCommand`, making no guarantees about the
  /// validity of its configured arguments and stdio. For example,
  /// `set_expected_loglevel()` is not automatically applied, which can have
  /// unexpected effects on log parsing.
  fn from(inner: Command) -> Self {
    Self { inner }
  }
}

impl Into<Command> for FfmpegCommand {
  fn into(self) -> Command {
    self.inner
  }
}
