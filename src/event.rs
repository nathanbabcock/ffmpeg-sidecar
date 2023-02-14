#[derive(Debug, Clone)]
pub enum FfmpegEvent {
  ParsedVersion(FfmpegVersion),
  ParsedConfiguration(FfmpegConfiguration),
  ParsedStreamMapping(String),
  ParsedOutput(FfmpegOutput),
  ParsedInputStream(AVStream),
  ParsedOutputStream(AVStream),
  LogInfo(String),
  LogWarning(String),
  LogError(String),
  LogUnknown(String),
  LogEOF,
  /// An error that didn't originate from the ffmpeg logs
  Error(String),
  Progress(FfmpegProgress),
  OutputFrame(OutputVideoFrame),
  Done,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegOutput {
  pub to: String,
  pub index: u32,
  pub raw_log_message: String,
}

impl FfmpegOutput {
  /// Detects one of several identifiers which indicate output to stdout
  pub fn is_stdout(&self) -> bool {
    ["pipe", "pipe:", "pipe:1"].contains(&self.to.as_str())
  }
}

#[derive(Debug, Clone)]
pub struct AVStream {
  /// Corresponds to stream `-pix_fmt` parameter, e.g. `rgb24`
  pub pix_fmt: String,
  /// Width in pixels
  pub width: u32,
  /// Height in pixels
  pub height: u32,
  /// The index of the input or output that this stream belongs to
  pub parent_index: usize,
  /// The stderr line that this stream was parsed from
  pub raw_log_message: String,
  // /// e.g. `Video`, `Audio`, `data`, `subtitle`, etc.
  // pub stream_type: String,
  // /// Corresponds to stream `-f` parameter, e.g. `rawvideo`
  // pub format: String,
  // /// Frames per second
  // pub fps: f32,
  // /// tbr is guessed from the video stream and is the value users want to see when they look for the video frame rate
  // tbr: f32,
  // /// the time base in AVStream that has come from the container
  // tbn: f32,
}

#[derive(Debug, Clone)]
pub struct FfmpegVersion {
  pub version: String,
  pub raw_log_message: String,
}

#[derive(Debug, Clone)]
pub struct FfmpegConfiguration {
  pub configuration: Vec<String>,
  pub raw_log_message: String,
}

#[derive(Debug, Clone)]
pub struct FfmpegProgress {
  /// index of the current output frame
  pub frame: u32,

  /// frames per second
  pub fps: f32,

  /// Quality factor (if applicable)
  pub q: f32,

  /// Current total size of the output in kilobytes
  pub size_kb: u32,

  /// The raw time string in a format like `00:03:29.04`
  pub time: String,

  /// Bitrate in kilo**bits** per second
  pub bitrate_kbps: f32,

  /// Processing speed as a ratio of the input duration
  ///
  /// - 1x is realtime
  /// - 2x means 2 seconds of input are processed in 1 second of wall clock time
  pub speed: f32,

  /// The line that this progress was parsed from
  pub raw_log_message: String,
}

#[derive(Debug, Clone)]
pub struct OutputVideoFrame {
  pub width: u32,
  pub height: u32,
  pub pix_fmt: String,
  pub output_index: u32,
  pub data: Vec<u8>,
}
