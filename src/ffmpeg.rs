use std::{
  io::{self, BufRead, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
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
  pub fn run_version(&mut self) -> io::Result<()> {
    self.args(&["-version"]);
    self.spawn()
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

  /// Run the ffmpeg command with the configured parameters
  pub fn spawn(&mut self) -> io::Result<()> {
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
    Ok(())
  }

  /// Run the command and wait for it to finish. If a fatal error occurs,
  /// returns the error message.
  pub fn run(mut self) -> Result<(), String> {
    self.spawn().map_err(|e| e.to_string())?;

    let stderr = self.stderr.unwrap();
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
      let bytes_read = reader.read_line(&mut line);

      match bytes_read {
        Ok(0) => break, // EOF
        Ok(_) => {
          println!("{}", line.trim_end());
          line.clear();
        }
        Err(e) => return Err(e.to_string()),
      }
    }

    // stateful; parsing from stderr as you go along
    // - prelude (config + metadata)
    // - progress messages
    // - warnings
    // - fatal errors

    Ok(())
  }

  /// Wait for the ffmpeg process to exit and return the exit status.
  pub fn wait(&mut self) -> io::Result<ExitStatus> {
    self.child.take().unwrap().wait()
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
    assert!(FfmpegSidecar::new().testsrc().pipe_stdout().run().is_ok());
  }
}
