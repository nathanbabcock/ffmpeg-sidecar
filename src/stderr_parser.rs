use std::{
  io::{BufRead, BufReader, Read},
  process::{ChildStderr, Stdio},
};

pub struct StderrParser<R: Read> {
  version: Option<String>,
  cur_line: String,
  reader: BufReader<R>,
}

impl<R: Read> StderrParser<R> {
  pub fn parse_next_line(&mut self) -> Result<(), String> {
    let bytes_read = self.reader.read_line(&mut self.cur_line);
    match bytes_read {
      Ok(0) => Err("EOF".to_string()),
      Ok(_) => {
        if let Some(version) = parse_version(&self.cur_line) {
          self.version = Some(version);
        }
        Ok(())
      }
      Err(e) => Err(e.to_string()),
    }
  }

  /// Gets the ffmpeg version identifier string. If it hasn't already been
  /// parsed, reads from the stderr stream until it finds it.
  pub fn ffmpeg_version(&mut self) -> Result<String, String> {
    loop {
      if let Some(version) = &self.version {
        return Ok(version.clone());
      }
      self.parse_next_line()?;
    }
  }

  pub fn new(inner: R) -> Self {
    Self {
      version: None,
      cur_line: String::new(),
      reader: BufReader::new(inner),
    }
  }
}

/// Parses the ffmpeg version string from the stderr stream,
/// typically the very first line of output:
///
/// ```
/// use ffmpeg_sidecar::stderr_parser::parse_version;
///
/// let stderr_line = "ffmpeg version 2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev Copyright (c) 2000-2023 the FFmpeg developers";
///
/// let version = parse_version(stderr_line).unwrap();
///
/// assert!(version == "2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev");
/// ```
pub fn parse_version(string: &str) -> Option<String> {
  let version_prefix = "ffmpeg version ";
  if string.starts_with(version_prefix) {
    string[version_prefix.len()..]
      .trim_end()
      .split(' ')
      .next()
      .map(|s| s.to_string())
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::process::{Command, Stdio};

  #[test]
  fn test_version() {
    let cmd = Command::new("ffmpeg")
      .arg("-version")
      .stdout(Stdio::piped())
      // ⚠ notice that ffmpeg emits on stdout when `-version` or `-help` is passed!
      .spawn()
      .unwrap();

    let stdout = cmd.stdout.unwrap();
    let mut parser = StderrParser::new(stdout);
    let version = parser.ffmpeg_version();
    assert!(version.is_ok());
  }
}
