//! Any event that occurs during the execution of an FFmpeg command.

/// Any event that occurs during the execution of an FFmpeg command,
/// including log messages, parsed metadata, progress updates, and output.
#[derive(Debug, Clone, PartialEq)]
pub enum FfmpegEvent {
  ParsedVersion(FfmpegVersion),
  ParsedConfiguration(FfmpegConfiguration),
  ParsedStreamMapping(String),
  ParsedInput(FfmpegInput),
  ParsedOutput(FfmpegOutput),
  ParsedInputStream(Stream),
  ParsedOutputStream(Stream),
  ParsedDuration(FfmpegDuration),
  Log(LogLevel, String),
  LogEOF,
  /// An error that didn't originate from the ffmpeg logs
  Error(String),
  Progress(FfmpegProgress),
  OutputFrame(OutputVideoFrame),
  /// A chunk of data that may not correspond to a complete frame.
  /// For example, it may contain encoded h264.
  /// These chunks will need to be handled manually, or piped directly to
  /// another FFmpeg instance.
  OutputChunk(Vec<u8>),
  Done,
}

/// The internal log level designated by FFmpeg on each message.
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
  Info,
  Warning,
  Error,
  Fatal,
  Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegInput {
  pub index: u32,
  pub duration: Option<f64>,
  pub raw_log_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegDuration {
  pub input_index: u32,
  pub duration: f64,
  pub raw_log_message: String,
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

/// Represents metadata about a stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
  /// Corresponds to stream `-f` parameter, e.g. `rawvideo`, `h264`, `opus` or `srt`.
  pub format: String,
  // The language of the stream as a three letter code such as `eng`, `ger` or `jpn`.
  pub language: String,
  /// The index of the input or output that this stream belongs to.
  pub parent_index: u32,
  /// The index of the stream inside the input.
  pub stream_index: u32,
  /// The stderr line that this stream was parsed from.
  pub raw_log_message: String,
  // Data that is specific to a certain stream type.
  pub type_specific_data: StreamTypeSpecificData,
}

impl Stream {
  pub fn is_audio(&self) -> bool {
    matches!(self.type_specific_data, StreamTypeSpecificData::Audio(_))
  }
  pub fn is_subtitle(&self) -> bool {
    matches!(self.type_specific_data, StreamTypeSpecificData::Subtitle())
  }
  pub fn is_video(&self) -> bool {
    matches!(self.type_specific_data, StreamTypeSpecificData::Video(_))
  }
  pub fn is_other(&self) -> bool {
    matches!(self.type_specific_data, StreamTypeSpecificData::Other())
  }

  pub fn audio_data(&self) -> Option<&AudioStream> {
    match &self.type_specific_data {
      StreamTypeSpecificData::Audio(audio_stream) => Some(audio_stream),
      _ => None,
    }
  }
  pub fn video_data(&self) -> Option<&VideoStream> {
    match &self.type_specific_data {
      StreamTypeSpecificData::Video(video_stream) => Some(video_stream),
      _ => None,
    }
  }
}

/// Represents metadata that is specific to a stream, e.g. fields that are only found in audio
/// streams or that are only found in video streams, etc. Storing this in an enum allows function to
/// accept the generic `Stream` type regardless of its actual type (audio, video, ...).
#[derive(Debug, Clone, PartialEq)]
pub enum StreamTypeSpecificData {
  Audio(AudioStream),
  Video(VideoStream),
  Subtitle(),
  Other(),
}

/// Represents metadata that is specific to audio streams.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioStream {
  /// The sample rate of the audio stream, e.g. 48000 (Hz)
  pub sample_rate: u32,
  /// The number of channels of the audio stream, e.g. `stereo`, `5.1` or `7.1`
  pub channels: String,
}

/// Represents metadata that is specific to video streams.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoStream {
  /// Corresponds to stream `-pix_fmt` parameter, e.g. `rgb24`
  pub pix_fmt: String,
  /// Width in pixels
  pub width: u32,
  /// Height in pixels
  pub height: u32,
  /// Framerate in frames per second
  pub fps: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegVersion {
  pub version: String,
  pub raw_log_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegConfiguration {
  pub configuration: Vec<String>,
  pub raw_log_message: String,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
pub struct OutputVideoFrame {
  /// The width of this video frame in pixels
  pub width: u32,
  /// The height of this video frame in pixels
  pub height: u32,
  /// The pixel format of the video frame, corresponding to the chosen
  /// `-pix_fmt` FFmpeg parameter.
  pub pix_fmt: String,
  /// The index of the FFmpeg output stream that emitted this frame.
  /// In a typical case, there is only one output stream and this will be 0.
  pub output_index: u32,
  /// Raw image frame data. The layout of the pixels in memory depends on
  /// `width`, `height`, and `pix_fmt`.
  pub data: Vec<u8>,
  /// Index of current frame, starting at 0 and monotonically increasing by 1
  pub frame_num: u32,
  /// Output frame timestamp in seconds
  pub timestamp: f32,
}

impl std::fmt::Debug for OutputVideoFrame {
  /// Omit the `data` field from the debug output
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OutputVideoFrame")
      .field("width", &self.width)
      .field("height", &self.height)
      .field("pix_fmt", &self.pix_fmt)
      .field("output_index", &self.output_index)
      .finish()
  }
}

// TODO fix the output for OutputChunk also
