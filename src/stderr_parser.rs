use std::io::{BufRead, BufReader, Read};

pub struct StderrParser<R: Read> {
  version: Option<String>,
  configuration: Option<Vec<String>>,
  cur_line: String,
  reader: BufReader<R>,
}

impl<R: Read> StderrParser<R> {
  pub fn parse_next_line(&mut self) -> Result<(), String> {
    let bytes_read = self.reader.read_line(&mut self.cur_line);
    match bytes_read {
      Ok(0) => Err("EOF".to_string()),
      Ok(_) => {
        if self.version.is_none() {
          if let Some(version) = parse_version(&self.cur_line) {
            self.version = Some(version);
            self.cur_line.clear();
            return Ok(());
          }
        }

        if self.configuration.is_none() {
          if let Some(configuration) = parse_configuration(&self.cur_line) {
            self.configuration = Some(configuration);
            self.cur_line.clear();
            return Ok(());
          }
        }

        self.cur_line.clear();
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

  /// Return the list of ffmpeg build flags, such as `--enable-gpl`, `--enable-libx264`, etc.
  /// If it hasn't already been parsed, reads from the stderr stream until it finds it.
  pub fn ffmpeg_configuration(&mut self) -> Result<Vec<String>, String> {
    loop {
      if let Some(configuration) = &self.configuration {
        return Ok(configuration.clone());
      }
      self.parse_next_line()?;
    }
  }

  pub fn new(inner: R) -> Self {
    Self {
      version: None,
      configuration: None,
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
/// let line = "ffmpeg version 2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev Copyright (c) 2000-2023 the FFmpeg developers\n";
///
/// let version = parse_version(line).unwrap();
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

/// Parses the list of configuration flags ffmpeg was built with.
/// Typically the second line of log output.
///
/// ## Example:
///
/// ```
/// use ffmpeg_sidecar::stderr_parser::parse_configuration;
///
/// let line = "configuration: --enable-gpl --enable-version3 --enable-static\n";
/// // Typically much longer, 20-30+ flags
///
/// let version = parse_configuration(line).unwrap();
///
/// assert!(version.len() == 3);
/// assert!(version[0] == "--enable-gpl");
/// assert!(version[1] == "--enable-version3");
/// assert!(version[2] == "--enable-static");
///
pub fn parse_configuration(string: &str) -> Option<Vec<String>> {
  let configuration_prefix = "configuration: ";
  if string.starts_with(configuration_prefix) {
    Some(
      string[configuration_prefix.len()..]
        .trim_end()
        .split(' ')
        .map(|s| s.to_string())
        .collect(),
    )
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

  #[test]
  fn test_configuration() {
    let cmd = Command::new("ffmpeg")
      .arg("-version")
      .stdout(Stdio::piped())
      // ⚠ notice that ffmpeg emits on stdout when `-version` or `-help` is passed!
      .spawn()
      .unwrap();

    let stdout = cmd.stdout.unwrap();
    let mut parser = StderrParser::new(stdout);
    let configuration = parser.ffmpeg_configuration();
    assert!(configuration.is_ok());
  }
}
