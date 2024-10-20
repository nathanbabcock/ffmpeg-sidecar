use crate::{child::FfmpegChild, paths::ffmpeg_path};
use std::{
  ffi::OsStr,
  fmt, io,
  process::{Command, CommandArgs, Stdio},
};

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
  //// Generic option aliases ////
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

  /// Alias for `-f` argument, the format name.
  ///
  /// Force input or output file format. The format is normally auto detected
  /// for input files and guessed from the file extension for output files, so
  /// this option is not needed in most cases.
  pub fn format<S: AsRef<str>>(&mut self, format: S) -> &mut Self {
    self.arg("-f");
    self.arg(format.as_ref());
    self
  }

  /// Alias for `-i` argument, the input file path or URL.
  ///
  /// To take input from stdin, use the value `-` or `pipe:0`.
  pub fn input<S: AsRef<str>>(&mut self, path_or_url: S) -> &mut Self {
    self.arg("-i");
    self.arg(path_or_url.as_ref());
    self
  }

  /// Alias for the output file path or URL.
  ///
  /// To send output to stdout, use the value `-` or `pipe:1`.
  ///
  /// Since this is the last argument in the command and has no `-` flag
  /// preceding it, it is equivalent to calling `.arg()` directly. However,
  /// using this command helps label the purpose of the argument, and makes the
  /// code more readable at a glance.
  pub fn output<S: AsRef<str>>(&mut self, path_or_url: S) -> &mut Self {
    self.arg(path_or_url.as_ref());
    self
  }

  /// Alias for `-y` argument: overwrite output files without asking.
  pub fn overwrite(&mut self) -> &mut Self {
    self.arg("-y");
    self
  }

  /// Alias for `-n` argument: do not overwrite output files, and exit
  /// immediately if a specified output file already exists.
  pub fn no_overwrite(&mut self) -> &mut Self {
    self.arg("-n");
    self
  }

  /// Alias for `-c:v` argument.
  ///
  /// Select an encoder (when used before an output file) or a decoder (when
  /// used before an input file) for one or more video streams. `codec` is the
  /// name of a decoder/encoder or a special value `copy`` (output only) to
  /// indicate that the stream is not to be re-encoded.
  pub fn codec_video<S: AsRef<str>>(&mut self, codec: S) -> &mut Self {
    self.arg("-c:v");
    self.arg(codec.as_ref());
    self
  }

  /// Alias for `-c:a` argument.
  ///
  /// Select an encoder (when used before an output file) or a decoder (when
  /// used before an input file) for one or more audio streams. `codec` is the
  /// name of a decoder/encoder or a special value `copy` (output only) to
  /// indicate that the stream is not to be re-encoded.
  pub fn codec_audio<S: AsRef<str>>(&mut self, codec: S) -> &mut Self {
    self.arg("-c:a");
    self.arg(codec.as_ref());
    self
  }

  /// Alias for `-c:s` argument.
  ///
  /// Select an encoder (when used before an output file) or a decoder (when
  /// used before an input file) for one or more subtitle streams. `codec` is
  /// the name of a decoder/encoder or a special value `copy` (output only) to
  /// indicate that the stream is not to be re-encoded.
  pub fn codec_subtitle<S: AsRef<str>>(&mut self, codec: S) -> &mut Self {
    self.arg("-c:s");
    self.arg(codec.as_ref());
    self
  }

  /// Alias for `-t` argument.
  ///
  /// When used as an input option (before `-i`), limit the duration of data
  /// read from the input file.
  ///
  /// When used as an output option (before an output url), stop writing the
  /// output after its duration reaches duration.
  ///
  /// `duration` must be a time duration specification, see [(ffmpeg-utils)the
  /// Time duration section in the ffmpeg-utils(1)
  /// manual](https://ffmpeg.org/ffmpeg-utils.html#time-duration-syntax).
  ///
  /// `-to` and `-t` are mutually exclusive and -t has priority.
  pub fn duration<S: AsRef<str>>(&mut self, duration: S) -> &mut Self {
    self.arg("-t");
    self.arg(duration.as_ref());
    self
  }

  /// Alias for `-to` argument.
  ///
  /// Stop writing the output or reading the input at `position`. `position`
  /// must be a time duration specification, see [(ffmpeg-utils)the Time
  /// duration section in the ffmpeg-utils(1)
  /// manual](https://ffmpeg.org/ffmpeg-utils.html#time-duration-syntax).
  ///
  /// `-to` and `-t` (aka `duration()`) are mutually exclusive and `-t` has
  /// priority.
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

  //// Video option aliases
  //// https://ffmpeg.org/ffmpeg.html#Video-Options

  /// Alias for '-crf:v' argument.
  ///
  /// Set CRF (Constant Rate Factor) for quality-based VBR (Variable BitRate)
  ///
  /// Use this rate control mode if you want to keep the best quality and care
  /// less about the file size. Lower values means better quality with
  /// bigger average bitrate (0 usually means lossless).
  ///
  /// Possible values depend on codec:
  ///   * 0-51 for h264 (default is 23), see [ffmpeg encoding guide for h264
  ///     for more details](https://trac.ffmpeg.org/wiki/Encode/H.264#crf)
  ///   * 0-51 for h265 (default is 28), see [ffmpeg encoding guide for h265
  ///     for more details](https://trac.ffmpeg.org/wiki/Encode/H.265#ConstantRateFactorCRF)
  ///   * 0-63 for vp9  (no default, 31 is recommended for 1080p HD video),
  ///     see [ffmpeg encoding guide for vp9 for more details](https://trac.ffmpeg.org/wiki/Encode/VP9#constrainedq)
  ///   * 0-63 for av1(libaom-av1) (no default), see [ffmpeg encoding guide
  ///     for libaom for more details](https://trac.ffmpeg.org/wiki/Encode/AV1#ConstantQuality)
  ///   * 0-63 for av1(libsvtav1) (default is 30), see [ffmpeg encoding guide
  ///     for svt-av1 for mode details](https://trac.ffmpeg.org/wiki/Encode/AV1#CRF)
  pub fn crf(&mut self, crf: u32) -> &mut Self {
    self.arg("-crf:v");
    self.arg(crf.to_string());
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

  /// Alias for `-preset:v` argument.
  ///
  /// Set preset which is basically trade-off between encoding speed and
  /// compression ratio.
  ///
  /// For h264 and h265 allowed values are:
  ///   * ultrafast
  ///   * superfast
  ///   * veryfast
  ///   * faster
  ///   * medium (default preset)
  ///   * slow
  ///   * slower
  ///   * veryslow
  ///   * placebo
  ///
  /// For svt-av1 supported values 0-13 (higher number providing a higher
  /// encoding speed). Prior to version 0.9.0 valid values was 0-8.
  ///
  /// For libaom supported values 0-11 (higher number providing a higher
  /// encoding speed)
  ///
  /// VP9 has no presets
  pub fn preset<S: AsRef<str>>(&mut self, preset: S) -> &mut Self {
    self.arg("-preset:v");
    self.arg(preset.as_ref());
    self
  }

  /// Alias for `-r` argument.
  ///
  /// Set frame rate (Hz value, fraction or abbreviation).
  ///
  /// As an input option, ignore any timestamps stored in the file and instead
  /// generate timestamps assuming constant frame rate `fps`. This is not the
  /// same as the `-framerate` option used for some input formats like image2 or
  /// v4l2 (it used to be the same in older versions of FFmpeg). If in doubt use
  /// `-framerate` instead of the input option `-r`.
  pub fn rate(&mut self, fps: f32) -> &mut Self {
    self.arg("-r");
    self.arg(fps.to_string());
    self
  }

  /// Alias for `-s` argument.
  ///
  /// Set frame size.
  ///
  /// As an input option, this is a shortcut for the `video_size` private
  /// option, recognized by some demuxers for which the frame size is either not
  /// stored in the file or is configurable – e.g. raw video or video grabbers.
  ///
  /// As an output option, this inserts the `scale` video filter to the end of
  /// the corresponding filtergraph. Please use the `scale` filter directly to
  /// insert it at the beginning or some other place.
  ///
  /// The format is `'wxh'` (default - same as source).
  pub fn size(&mut self, width: u32, height: u32) -> &mut Self {
    self.arg("-s");
    self.arg(format!("{}x{}", width, height));
    self
  }

  /// Alias for `-vn` argument.
  ///
  /// As an input option, blocks all video streams of a file from being filtered
  /// or being automatically selected or mapped for any output. See `-discard`
  /// option to disable streams individually.
  ///
  /// As an output option, disables video recording i.e. automatic selection or
  /// mapping of any video stream. For full manual control see the `-map`
  /// option.
  pub fn no_video(&mut self) -> &mut Self {
    self.arg("-vn");
    self
  }

  //// Advanced video option aliases
  //// https://ffmpeg.org/ffmpeg.html#Advanced-Video-options

  /// Alias for `-pix_fmt` argument.
  ///
  /// Set pixel format. Use `-pix_fmts` to show all the supported pixel formats.
  /// If the selected pixel format can not be selected, ffmpeg will print a
  /// warning and select the best pixel format supported by the encoder. If
  /// pix_fmt is prefixed by a `+`, ffmpeg will exit with an error if the
  /// requested pixel format can not be selected, and automatic conversions
  /// inside filtergraphs are disabled. If pix_fmt is a single `+`, ffmpeg
  /// selects the same pixel format as the input (or graph output) and automatic
  /// conversions are disabled.
  pub fn pix_fmt<S: AsRef<str>>(&mut self, format: S) -> &mut Self {
    self.arg("-pix_fmt");
    self.arg(format.as_ref());
    self
  }

  /// Alias for `-hwaccel` argument.
  ///
  /// Use hardware acceleration to decode the matching stream(s). The allowed
  /// values of hwaccel are:
  ///
  /// - `none`: Do not use any hardware acceleration (the default).
  /// - `auto`: Automatically select the hardware acceleration method.
  /// - `vdpau`: Use VDPAU (Video Decode and Presentation API for Unix) hardware
  ///   acceleration.
  /// - `dxva2`: Use DXVA2 (DirectX Video Acceleration) hardware acceleration.
  /// - `d3d11va`: Use D3D11VA (DirectX Video Acceleration) hardware
  ///   acceleration.
  /// - `vaapi`: Use VAAPI (Video Acceleration API) hardware acceleration.
  /// - `qsv`: Use the Intel QuickSync Video acceleration for video transcoding.
  ///   - Unlike most other values, this option does not enable accelerated
  ///     decoding (that is used automatically whenever a qsv decoder is selected),
  ///     but accelerated transcoding, without copying the frames into the system
  ///     memory.
  ///   - For it to work, both the decoder and the encoder must support QSV
  ///     acceleration and no filters must be used.
  ///
  /// This option has no effect if the selected hwaccel is not available or not
  /// supported by the chosen decoder.
  ///
  /// Note that most acceleration methods are intended for playback and will not
  /// be faster than software decoding on modern CPUs. Additionally, `ffmpeg`
  /// will usually need to copy the decoded frames from the GPU memory into the
  /// system memory, resulting in further performance loss. This option is thus
  /// mainly useful for testing.
  pub fn hwaccel<S: AsRef<str>>(&mut self, hwaccel: S) -> &mut Self {
    self.arg("-hwaccel");
    self.arg(hwaccel.as_ref());
    self
  }

  //// Audio option aliases
  //// https://ffmpeg.org/ffmpeg.html#Audio-Options

  /// Alias for `-an` argument.
  ///
  /// As an input option, blocks all audio streams of a file from being filtered
  /// or being automatically selected or mapped for any output. See `-discard`
  /// option to disable streams individually.
  ///
  /// As an output option, disables audio recording i.e. automatic selection or
  /// mapping of any audio stream. For full manual control see the `-map`
  /// option.
  pub fn no_audio(&mut self) -> &mut Self {
    self.arg("-an");
    self
  }

  //// Advanced option aliases
  //// https://ffmpeg.org/ffmpeg.html#Advanced-options

  /// Alias for `-map` argument.
  ///
  /// Create one or more streams in the output file. This option has two forms
  /// for specifying the data source(s): the first selects one or more streams
  /// from some input file (specified with `-i`), the second takes an output
  /// from some complex filtergraph (specified with `-filter_complex` or
  /// `-filter_complex_script`).
  ///
  /// In the first form, an output stream is created for every stream from the
  /// input file with the index input_file_id. If stream_specifier is given,
  /// only those streams that match the specifier are used (see the [Stream
  /// specifiers](https://ffmpeg.org/ffmpeg.html#Stream-specifiers) section for
  /// the stream_specifier syntax).
  ///
  /// A `-` character before the stream identifier creates a "negative" mapping.
  /// It disables matching streams from already created mappings.
  ///
  /// A trailing `?` after the stream index will allow the map to be optional:
  /// if the map matches no streams the map will be ignored instead of failing.
  /// Note the map will still fail if an invalid input file index is used; such
  /// as if the map refers to a non-existent input.
  ///
  /// An alternative `[linklabel]` form will map outputs from complex filter
  /// graphs (see the `-filter_complex` option) to the output file. `linklabel`
  /// must correspond to a defined output link label in the graph.
  ///
  /// This option may be specified multiple times, each adding more streams to
  /// the output file. Any given input stream may also be mapped any number of
  /// times as a source for different output streams, e.g. in order to use
  /// different encoding options and/or filters. The streams are created in the
  /// output in the same order in which the `-map` options are given on the
  /// commandline.
  ///
  /// Using this option disables the default mappings for this output file.
  pub fn map<S: AsRef<str>>(&mut self, map_string: S) -> &mut Self {
    self.arg("-map");
    self.arg(map_string.as_ref());
    self
  }

  /// Alias for `-readrate` argument.
  ///
  /// Limit input read speed.
  ///
  /// Its value is a floating-point positive number which represents the maximum
  /// duration of media, in seconds, that should be ingested in one second of
  /// wallclock time. Default value is zero and represents no imposed limitation
  /// on speed of ingestion. Value `1` represents real-time speed and is
  /// equivalent to `-re`.
  ///
  /// Mainly used to simulate a capture device or live input stream (e.g. when
  /// reading from a file). Should not be used with a low value when input is an
  /// actual capture device or live stream as it may cause packet loss.
  ///
  /// It is useful for when flow speed of output packets is important, such as
  /// live streaming.
  pub fn readrate(&mut self, speed: f32) -> &mut Self {
    self.arg("-readrate");
    self.arg(speed.to_string());
    self
  }

  /// Alias for `-re`.
  ///
  /// Read input at native frame rate. This is equivalent to setting `-readrate
  /// 1`.
  pub fn realtime(&mut self) -> &mut Self {
    self.arg("-re");
    self
  }

  /// Alias for `-fps_mode` argument.
  ///
  /// Set video sync method / framerate mode. vsync is applied to all output
  /// video streams but can be overridden for a stream by setting fps_mode.
  /// vsync is deprecated and will be removed in the future.
  ///
  /// For compatibility reasons some of the values for vsync can be specified as
  /// numbers (shown in parentheses in the following table).
  ///
  /// - `passthrough` (`0`): Each frame is passed with its timestamp from the
  ///   demuxer to the muxer.
  /// - `cfr` (`1`): Frames will be duplicated and dropped to achieve exactly
  ///   the requested constant frame rate.
  /// - `vfr` (`2`): Frames are passed through with their timestamp or dropped
  ///   so as to prevent 2 frames from having the same timestamp.
  /// - `drop`: As passthrough but destroys all timestamps, making the muxer
  ///   generate fresh timestamps based on frame-rate.
  /// - `auto` (`-1`): Chooses between cfr and vfr depending on muxer
  ///   capabilities. This is the default method.
  pub fn fps_mode<S: AsRef<str>>(&mut self, parameter: S) -> &mut Self {
    self.arg("-fps_mode");
    self.arg(parameter.as_ref());
    self
  }

  /// Alias for `-bsf:v` argument.
  ///
  /// Set bitstream filters for matching streams. `bitstream_filters` is a
  /// comma-separated list of bitstream filters. Use the `-bsfs` option to get
  /// the list of bitstream filters.
  ///
  /// See also: `-bsf:s` (subtitles), `-bsf:a` (audio), `-bsf:d` (data)
  pub fn bitstream_filter_video<S: AsRef<str>>(&mut self, bitstream_filters: S) -> &mut Self {
    self.arg("-bsf:v");
    self.arg(bitstream_filters.as_ref());
    self
  }

  /// Alias for `-filter_complex` argument.
  ///
  /// Define a complex filtergraph, i.e. one with arbitrary number of inputs
  /// and/or outputs. For simple graphs – those with one input and one output of
  /// the same type – see the `-filter` options. `filtergraph` is a description
  /// of the filtergraph, as described in the "Filtergraph syntax" section of
  /// the ffmpeg-filters manual.
  ///
  /// Input link labels must refer to input streams using the
  /// `[file_index:stream_specifier]` syntax (i.e. the same as `-map` uses). If
  /// `stream_specifier` matches multiple streams, the first one will be used.
  /// An unlabeled input will be connected to the first unused input stream of
  /// the matching type.
  ///
  /// Output link labels are referred to with `-map`. Unlabeled outputs are
  /// added to the first output file.
  ///
  /// Note that with this option it is possible to use only lavfi sources
  /// without normal input files.
  pub fn filter_complex<S: AsRef<str>>(&mut self, filtergraph: S) -> &mut Self {
    self.arg("-filter_complex");
    self.arg(filtergraph.as_ref());
    self
  }

  //// Preset argument sets for common use cases.

  /// Generate a procedural test video. Equivalent to `ffmpeg -f lavfi -i
  /// testsrc=duration=10`.
  ///
  /// [FFmpeg `testsrc` filter
  /// documentation](https://ffmpeg.org/ffmpeg-filters.html#allrgb_002c-allyuv_002c-color_002c-colorchart_002c-colorspectrum_002c-haldclutsrc_002c-nullsrc_002c-pal75bars_002c-pal100bars_002c-rgbtestsrc_002c-smptebars_002c-smptehdbars_002c-testsrc_002c-testsrc2_002c-yuvtestsrc)
  pub fn testsrc(&mut self) -> &mut Self {
    self.args(["-f", "lavfi", "-i", "testsrc=duration=10"]);
    self
  }

  /// Preset for emitting raw decoded video frames on stdout. Equivalent to `-f
  /// rawvideo -pix_fmt rgb24 -`.
  pub fn rawvideo(&mut self) -> &mut Self {
    self.args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"]);
    self
  }

  /// Configure the ffmpeg command to produce output on stdout.
  ///
  /// Synchronizes two changes:
  /// 1. Pass `pipe:1` to the ffmpeg command ("output on stdout")
  /// 2. Set the `stdout` field of the inner `Command` to `Stdio::piped()`
  pub fn pipe_stdout(&mut self) -> &mut Self {
    self.arg("-");
    self.inner.stdout(Stdio::piped());
    self
  }

  /// Redirect the command's `stdout` to a named pipe.
  #[cfg(all(unix, feature = "named_pipes"))]
  pub fn create_named_pipe(&mut self, path: &std::path::Path) -> Result<&mut Self, Box<dyn std::error::Error>> {
    use nix::sys::stat;
    use nix::unistd;
    unistd::mkfifo(path, stat::Mode::S_IRWXU)?;

    self.arg(format!("pipe:{}", path.display()));
    self.inner.stdout(Stdio::piped());

    Ok(self)
  }

  /// Redirect the command's `stdout` to a named pipe.
  #[cfg(all(windows, feature = "named_pipes"))]
  pub fn create_named_pipe(&mut self, path: &std::path::Path) -> Result<&mut Self, Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::namedpipeapi::CreateNamedPipeW; // Corrected import
    use winapi::um::winbase::{PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT};

    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
      CreateNamedPipeW(
        path_wide.as_ptr(),
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
        1,
        4096,
        4096,
        0,
        null_mut(),
      )
    };

    if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
      return Err("Failed to create named pipe".into());
    }

    self.arg(format!("pipe:{}", path.display()));
    self.inner.stdout(Stdio::piped());

    // todo: `handle` needs to be persisted in order to call `CloseHandle` later

    Ok(self)
  }

  /// Automatically applied in the constructor of `FfmpegCommand`. Configures
  /// logging with a level and format expected by the log parser.
  ///
  /// Equivalent to `ffmpeg -loglevel level+info`.
  ///
  /// The `level` flag adds a prefix to all log messages with the log level in
  /// square brackets, allowing the parser to distinguish between ambiguous
  /// messages like warnings vs errors.
  ///
  /// The `+info` flag enables the `info` log level, which is the default level.
  ///
  /// If this settings is manually overridden, the log parser should still work,
  /// but lose some semantic distinction between log levels.
  fn set_expected_loglevel(&mut self) -> &mut Self {
    self.args(["-loglevel", "level+info"]);
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

  /// Appends `-n` (no overwrite) to the args list if needed.
  /// The interactive "Would you like to overwrite?" prompt is problematic,
  /// since it won't be parsed by the log parser and the process will appear
  /// to hang indefinitely without any indication of what's happening.
  fn prevent_overwrite_prompt(&mut self) -> &mut Self {
    let is_overwrite_arg = |arg| arg == "-y" || arg == "-n" || arg == "-nostdin";
    if !self.get_args().any(is_overwrite_arg) {
      self.no_overwrite();
    }
    self
  }

  /// Spawn the ffmpeg command as a child process, wrapping it in a
  /// `FfmpegChild` interface.
  ///
  /// Please note that if the result is not used with [`wait()`](FfmpegChild::wait)
  /// the process is not cleaned up correctly resulting in a zombie process
  /// until your main thread exits.
  ///
  /// Identical to `spawn` in [`std::process::Command`].
  pub fn spawn(&mut self) -> io::Result<FfmpegChild> {
    self.prevent_overwrite_prompt();
    self.inner.spawn().map(FfmpegChild::from_inner)
  }

  /// Print a command that can be copy-pasted to run in the terminal. Requires
  /// `&mut self` so that it chains seamlessly with other methods in the
  /// interface.
  pub fn print_command(&mut self) -> &mut Self {
    let program = self.inner.get_program().to_str();
    let args = self
      .inner
      .get_args()
      .map(|s| s.to_str())
      .collect::<Option<Vec<_>>>();
    if let (Some(program), Some(args)) = (program, args) {
      println!("Command: {} {}", program, args.join(" "));
    }

    self
  }

  /// Disable creating a new console window for the spawned process on Windows.
  /// Has no effect on other platforms. This can be useful when spawning a command
  /// from a GUI program.
  ///
  /// This is called automatically in the constructor. To override, use
  /// `CommandExt::creation_flags()` directly on the inner `Command`.
  pub fn create_no_window(&mut self) -> &mut Self {
    self.as_inner_mut().create_no_window();
    self
  }

  //// Constructors
  pub fn new() -> Self {
    Self::new_with_path(ffmpeg_path())
  }

  pub fn new_with_path<S: AsRef<OsStr>>(path_to_ffmpeg_binary: S) -> Self {
    // Configure `Command`
    let mut inner = Command::new(&path_to_ffmpeg_binary);
    inner.stdin(Stdio::piped());
    inner.stderr(Stdio::piped());
    inner.stdout(Stdio::piped());

    // Configure `FfmpegCommand`
    let mut ffmpeg_command = Self { inner };
    ffmpeg_command.set_expected_loglevel();
    ffmpeg_command.create_no_window();
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
  /// Format the program and arguments of a Command for display. Any non-utf8
  /// data is lossily converted using the utf8 replacement character.
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

impl From<FfmpegCommand> for Command {
  fn from(val: FfmpegCommand) -> Self {
    val.inner
  }
}

/// Verify whether ffmpeg is installed on the system. This will return true if
/// there is an ffmpeg binary in the PATH, or in the same directory as the Rust
/// executable.
pub fn ffmpeg_is_installed() -> bool {
  Command::new(ffmpeg_path())
    .arg("-version")
    .create_no_window()
    .stderr(Stdio::null())
    .stdout(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or_else(|_| false)
}

pub(crate) trait BackgroundCommand {
  fn create_no_window(&mut self) -> &mut Self;
}

impl BackgroundCommand for Command {
  /// Disable creating a new console window for the spawned process on Windows.
  /// Has no effect on other platforms. This can be useful when spawning a command
  /// from a GUI program.
  fn create_no_window(&mut self) -> &mut Self {
    #[cfg(target_os = "windows")]
    std::os::windows::process::CommandExt::creation_flags(self, 0x08000000);
    self
  }
}
