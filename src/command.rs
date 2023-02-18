use std::{
  ffi::OsStr,
  fmt, io,
  process::{Command, CommandArgs, Stdio},
};

use crate::child::FfmpegChild;

/// A wrapper around [`std::process::Command`] with some convenient preset
/// argument sets and customization for `ffmpeg` specifically.
///
/// The `rustdoc` on each method includes relevant information from the FFmpeg
/// documentation: <https://ffmpeg.org/ffmpeg.html>. Refer there for the
/// exhaustive list of possible arguments.
pub struct FfmpegCommand {
  inner: Command,
}

impl FfmpegCommand {
  //// Generic option aliases
  //// https://ffmpeg.org/ffmpeg.html#Generic-options

  /// alias for `-hide_banner` argument.
  ///
  /// Suppress printing banner.
  ///
  /// All FFmpeg tools will normally show a copyright notice, build options and
  /// library versions. This option can be used to suppress printing this
  /// information.
  pub fn hide_banner(&mut self) -> &mut Self {
    self.arg("-hide_banner");
    self
  }

  //// Main option aliases
  //// https://ffmpeg.org/ffmpeg.html#Main-options

  /// Alias for `-i` argument, the input file path or URL.
  ///
  /// To take input from stdin, use the value `-` or `pipe`.
  pub fn input<S: AsRef<str>>(&mut self, path_or_url: S) -> &mut Self {
    self.arg("-i");
    self.arg(path_or_url.as_ref());
    self
  }

  /// Alias for `-y` argument: overwrite output files without asking.
  pub fn overwrite(&mut self) -> &mut Self {
    self.arg("-y");
    self
  }

  /// Alias for `-n` argument: do not overwrite output files, and exit immediately if a specified output file already exists.
  pub fn no_overwrite(&mut self) -> &mut Self {
    self.arg("-n");
    self
  }

  /// Alias for `-c:v` argument.
  ///
  /// Select an encoder (when used before an output file) or a decoder (when
  /// used before an input file) for one or more streams. `codec` is the name of a
  /// decoder/encoder or a special value copy (output only) to indicate that the
  /// stream is not to be re-encoded.
  pub fn codec_video<S: AsRef<str>>(&mut self, codec: S) -> &mut Self {
    self.arg("-c:v");
    self.arg(codec.as_ref());
    self
  }

  /// Alias for `-c:a` argument.
  ///
  /// Select an encoder (when used before an output file) or a decoder (when
  /// used before an input file) for one or more streams. `codec` is the name of a
  /// decoder/encoder or a special value `copy` (output only) to indicate that the
  /// stream is not to be re-encoded.
  pub fn codec_audio<S: AsRef<str>>(&mut self, codec: S) -> &mut Self {
    self.arg("-c:a");
    self.arg(codec.as_ref());
    self
  }

  /// Alias for `-t` argument.
  ///
  /// When used as an input option (before `-i`), limit the duration of data read from the input file.
  ///
  /// When used as an output option (before an output url), stop writing the output after its duration reaches duration.
  ///
  /// `duration` must be a time duration specification, see [(ffmpeg-utils)the Time duration section in the ffmpeg-utils(1) manual](https://ffmpeg.org/ffmpeg-utils.html#time-duration-syntax).
  ///
  /// `-to` and `-t` are mutually exclusive and -t has priority.
  pub fn duration<S: AsRef<str>>(&mut self, duration: S) -> &mut Self {
    self.arg("-t");
    self.arg(duration.as_ref());
    self
  }

  /// Alias for `-to` argument.
  ///
  /// Stop writing the output or reading the input at `position`. `position` must be a time duration specification, see [(ffmpeg-utils)the Time duration section in the ffmpeg-utils(1) manual](https://ffmpeg.org/ffmpeg-utils.html#time-duration-syntax).
  ///
  /// `-to` and `-t` (aka `duration()`) are mutually exclusive and `-t` has priority.
  pub fn to<S: AsRef<str>>(&mut self, position: S) -> &mut Self {
    self.arg("-to");
    self.arg(position.as_ref());
    self
  }

  /// Alias for `-fs` argument.
  ///
  /// Set the file size limit, expressed in bytes. No further chunk of bytes is
  /// written after the limit is exceeded. The size of the output file is
  /// slightly more than the requested file size.
  pub fn limit_file_size(&mut self, size_in_bytes: u32) -> &mut Self {
    self.arg("-fs");
    self.arg(size_in_bytes.to_string());
    self
  }

  /// Alias for `-ss` argument.
  ///
  /// When used as an input option (before `-i`), seeks in this input file to
  /// position. Note that in most formats it is not possible to seek exactly, so
  /// `ffmpeg` will seek to the closest seek point before `position`. When
  /// transcoding and `-accurate_seek` is enabled (the default), this extra
  /// segment between the seek point and `position` will be decoded and
  /// discarded. When doing stream copy or when `-noaccurate_seek` is used, it
  /// will be preserved.
  ///
  /// When used as an output option (before an output url), decodes but discards
  /// input until the timestamps reach `position`.
  ///
  /// `position` must be a time duration specification, see [(ffmpeg-utils)the
  /// Time duration section in the ffmpeg-utils(1)
  /// manual](https://ffmpeg.org/ffmpeg-utils.html#time-duration-syntax).
  pub fn seek<S: AsRef<str>>(&mut self, position: S) -> &mut Self {
    self.arg("-ss");
    self.arg(position.as_ref());
    self
  }

  /// Alias for `-sseof` argument.
  ///
  /// Like the `-ss` option but relative to the "end of file". That is negative
  /// values are earlier in the file, 0 is at EOF.
  pub fn seek_eof<S: AsRef<str>>(&mut self, position: S) -> &mut Self {
    self.arg("-sseof");
    self.arg(position.as_ref());
    self
  }

  /// Alias for `-frames:v` argument.
  ///
  /// Stop writing to the stream after `framecount` frames.
  ///
  /// See also: `-frames:a` (audio), `-frames:d` (data).
  pub fn frames(&mut self, framecount: u32) -> &mut Self {
    self.arg("-frames:v");
    self.arg(framecount.to_string());
    self
  }

  /// Alias for `-filter` argument.
  ///
  /// Create the filtergraph specified by `filtergraph` and use it to filter the
  /// stream.
  ///
  /// `filtergraph` is a description of the filtergraph to apply to the stream,
  /// and must have a single input and a single output of the same type of the
  /// stream. In the filtergraph, the input is associated to the label `in`, and
  /// the output to the label `out`. See the ffmpeg-filters manual for more
  /// information about the filtergraph syntax.
  ///
  /// See the [`-filter_complex`
  /// option](https://ffmpeg.org/ffmpeg.html#filter_005fcomplex_005foption) if
  /// you want to create filtergraphs with multiple inputs and/or outputs.
  pub fn filter<S: AsRef<str>>(&mut self, filtergraph: S) -> &mut Self {
    self.arg("-filter");
    self.arg(filtergraph.as_ref());
    self
  }

  //// Preset argument sets for common use cases.

  /// Generate a procedural test video.
  /// Equivalent to `ffmpeg -i lavfi -f testsrc`
  ///
  /// [FFmpeg `testsrc` filter documentation](https://ffmpeg.org/ffmpeg-filters.html#allrgb_002c-allyuv_002c-color_002c-colorchart_002c-colorspectrum_002c-haldclutsrc_002c-nullsrc_002c-pal75bars_002c-pal100bars_002c-rgbtestsrc_002c-smptebars_002c-smptehdbars_002c-testsrc_002c-testsrc2_002c-yuvtestsrc)
  pub fn testsrc(&mut self) -> &mut Self {
    self.args(&["-f", "lavfi", "-i", "testsrc"]);
    self
  }

  /// Preset for emitting raw decoded video frames on stdout.
  /// Equivalent to `-f rawvideo -pix_fmt rgb24 -`.
  pub fn rawvideo(&mut self) -> &mut Self {
    self.args(&["-f", "rawvideo", "-pix_fmt", "rgb24", "-"]);
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
  ///
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

  /// Print a command that can be copy-pasted to run in the terminal.
  /// Requires `&mut self` so that it chains seamlessly with other methods in the interface.
  pub fn print_command(&mut self) -> &mut Self {
    println!("Command: {:?}", self.inner);
    self
  }

  //// Constructors
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

  //// Escape hatches
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
