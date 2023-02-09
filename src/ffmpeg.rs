use std::{
  io,
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
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
  pub fn run_version(mut self) -> io::Result<Self> {
    self.args.push("-version".to_string());
    self.run()
  }

  /// Run the ffmpeg command with the configured parameters
  pub fn run(mut self) -> io::Result<Self> {
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
    Ok(self)
  }

  /// Print the metadata about a particular
  pub fn metadata() {}

  //// Setters
  pub fn set_ffmpeg_exe(&mut self, ffmpeg_exe: &str) -> &mut Self {
    self.ffmpeg_exe = ffmpeg_exe.to_string();
    self
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
}
