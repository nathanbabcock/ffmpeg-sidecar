pub enum FfmpegEvent {
  ParsedVersion(FfmpegVersion),
  ParsedConfiguration(FfmpegConfiguration),
  ParsedInputs(FfmpegInputs),
  ParsedOutputs(FfmpegOutputs),
  LogInfo(String),
  LogWarning(String),
  LogError(String),
  LogUnknown(String),
  Progress(FfmpegProgress),
  OutputFrame(OutputVideoFrame),
}

pub struct FfmpegInputs {
  pub duration: String,
  pub start_sec: f32,
  pub bitrate_kbps: f32,
  pub streams: Vec<AVStream>,
}

pub struct FfmpegOutputs {
  pub streams: Vec<AVStream>,
}

pub struct AVStream {
  /// e.g. `Video`, `Audio`, `data`, `subtitle`, etc.
  pub stream_type: String,
  /// Corresponds to stream `-f` parameter, e.g. `rawvideo`
  pub format: String,
  /// Corresponds to stream `-pix_fmt` parameter, e.g. `rgb24`
  pub pix_fmt: String,
  /// Width in pixels
  pub width: u32,
  /// Height in pixels
  pub height: u32,
  /// Frames per second
  pub fps: f32,

  pub raw_log_message: String,
  // /// tbr is guessed from the video stream and is the value users want to see when they look for the video frame rate
  // tbr: f32,
  // /// the time base in AVStream that has come from the container
  // tbn: f32,
}

pub struct FfmpegVersion {
  version: String,
  raw_log_message: String,
}

pub struct FfmpegConfiguration {
  configuration: Vec<String>,
  raw_log_message: String,
}

pub struct FfmpegProgress {
  frame: u32,
  fps: f32,
  q: f32,
  size_kb: u32,
  time: String,
  bitrate_kbps: f32,
  speed: f32,
  raw_log_message: String,
}

pub struct OutputVideoFrame {
  pub width: u32,
  pub height: u32,
  pub pix_fmt: String,
  pub output_index: u32,
  pub data: Vec<u8>,
}
