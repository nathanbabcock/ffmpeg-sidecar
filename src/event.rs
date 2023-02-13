#[derive(Debug, Clone)]
pub enum FfmpegEvent {
  ParsedVersion(FfmpegVersion),
  ParsedConfiguration(FfmpegConfiguration),
  ParsedInputStream(AVStream),
  ParsedOutputStream(AVStream),
  LogInfo(String),
  LogWarning(String),
  LogError(String),
  LogUnknown(String),
  Progress(FfmpegProgress),
  OutputFrame(OutputVideoFrame),
}

#[derive(Debug, Clone)]
pub struct FfmpegInputs {
  pub duration: String,
  pub start_sec: f32,
  pub bitrate_kbps: f32,
  pub streams: Vec<AVStream>,
}

#[derive(Debug, Clone)]
pub struct FfmpegOutputs {
  pub streams: Vec<AVStream>,
}

#[derive(Debug, Clone)]
pub struct AVStream {
  /// Corresponds to stream `-pix_fmt` parameter, e.g. `rgb24`
  pub pix_fmt: String,
  /// Width in pixels
  pub width: u32,
  /// Height in pixels
  pub height: u32,
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
  pub frame: u32,
  pub fps: f32,
  pub q: f32,
  pub size_kb: u32,
  pub time: String,
  pub bitrate_kbps: f32,
  pub speed: f32,
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
