use std::{
  io::{self, BufRead, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
  sync::mpsc::{sync_channel, Receiver},
};

/// Check if the ffmpeg command exists. Uses system-wide scope by default (e.g.
/// PATH var on windows)
pub fn check_ffmpeg() -> bool {
  check_ffmpeg_with_path("ffmpeg")
}

/// Check if ffmpeg exists at the given path
pub fn check_ffmpeg_with_path(ffmpeg_exe: &str) -> bool {
  Command::new(ffmpeg_exe)
    .arg("-version")
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

pub struct OutputVideoFrame {
  pub width: u32,
  pub height: u32,
  pub pix_fmt: String,
  pub data: Vec<u8>,
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

/// Represents any raw or parsed log message, or outputted video frame
pub enum FfmpegEvent {
  LogInfo(String),
  LogWarning(String),
  LogError(String),
  LogUnknown(String),
  Progress(FfmpegProgress),
  OutputFrame(OutputVideoFrame),
}

pub struct FfmpegSidecar {
  /// The path to the ffmpeg executable
  ffmpeg_exe: String,
  child: Option<Child>,
  args: Vec<String>,
  stdout: Option<ChildStdout>,
  stderr: Option<ChildStderr>,
  stdin: Option<ChildStdin>,
}

impl FfmpegSidecar {
  /// Runs `ffmpeg -version`
  ///
  /// Spawn a command to print the version and configuration of ffmpeg,
  /// consuming the instance.
  pub fn run_version(mut self) -> io::Result<Receiver<FfmpegEvent>> {
    self.args(&["-version"]);
    self.start()
  }

  /// Generate a procedural test video.
  /// Equivalent to `ffmpeg -i lavfi -f testsrc`
  pub fn testsrc(mut self) -> Self {
    self.args(&["-f", "lavfi", "-i", "testsrc"]);
    self
  }

  /// Configure the ffmpeg command to produce output on stdout.
  /// Equivalent to `ffmpeg ... -` or `ffmpeg ... pipe:1`
  pub fn pipe_stdout(mut self) -> Self {
    self.args(&["-"]);
    self
  }

  /// Run the ffmpeg command with the configured parameters.
  /// Consumes the instance and returns a receiver for events during processing.
  pub fn start(&mut self) -> io::Result<Receiver<FfmpegEvent>> {
    let mut child = Command::new(&self.ffmpeg_exe)
      .args(&self.args)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()?;
    self.stdout = child.stdout.take();
    self.stderr = child.stderr.take();
    self.stdin = child.stdin.take();
    self.child = Some(child);

    let (tx, rx) = sync_channel::<FfmpegEvent>(0);

    let stdout = self.stdout.take().unwrap();
    let stderr = self.stderr.take().unwrap();

    let stderr_thread = std::thread::spawn(move || {
      let reader = BufReader::new(stderr);
      for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("[info]") {
          // if line.starts_with("[info] frame=") ... // parse progress
          tx.send(FfmpegEvent::LogInfo(line)).unwrap();
        } else if line.starts_with("[warning]") {
          tx.send(FfmpegEvent::LogWarning(line)).unwrap();
        } else if line.starts_with("[error]") || line.starts_with("[fatal]") {
          tx.send(FfmpegEvent::LogError(line)).unwrap();
        } else {
          tx.send(FfmpegEvent::LogUnknown(line)).unwrap();
        }
      }
    });

    // TODO: parse output until all metadata has been read
    // Add messages types:
    // - ParsedVersion
    // - ParsedConfiguration
    // - ParsedInputs (1+, each with 1+ streams)
    // - ParsedOutputs (1+, each with 1+ streams)
    let mut output_width: u32;
    let mut output_height: u32;
    let mut output_pix_fmt: u32;
    while let Ok(event) = rx.recv() {}

    // Save these parsed values to a struct field (stateful)
    // ...or re-forward them onto the receiver a second time?
    // ...or both?

    // Then handle stdout
    let stdout_thread = std::thread::spawn(move || {
      let reader = BufReader::new(stdout);
      let buf = &mut [0u8; 4096]; // TODO determine buffer size from pix_fmt, width, and height
    });

    Ok(rx)
  }

  //// Setters
  pub fn set_ffmpeg_exe(mut self, ffmpeg_exe: &str) -> Self {
    self.ffmpeg_exe = ffmpeg_exe.to_string();
    self
  }

  pub fn args(&mut self, args: &[&str]) {
    self.args.extend(args.iter().map(|s| s.to_string()));
  }

  //// Constructor
  pub fn new() -> Self {
    Self {
      ffmpeg_exe: "ffmpeg".to_string(),
      child: None,
      args: Vec::new(),
      stdout: None,
      stderr: None,
      stdin: None,
    }
  }
}

impl Default for FfmpegSidecar {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_check_ffmpeg() {
    assert!(check_ffmpeg());
  }

  #[test]
  fn testsrc() {
    let rx = FfmpegSidecar::new()
      .testsrc()
      .pipe_stdout()
      .start()
      .unwrap();
    while let Ok(event) = rx.recv() {}
  }
}
