use std::{
  io::{BufRead, BufReader, Read},
  sync::mpsc::SyncSender,
};

use crate::event::{FfmpegConfiguration, FfmpegEvent, FfmpegVersion};

pub struct FfmpegLogParser<R: Read> {
  reader: BufReader<R>,
}

impl<R: Read> FfmpegLogParser<R> {
  /// Consume lines from the inner reader until obtaining a completed
  /// `FfmpegEvent`, returning it.
  ///
  /// Typically this consumes a single line, but in the case of multi-line
  /// input/output stream specifications, nested method calls will consume
  /// additional lines until the entire vector of Inputs/Outputs is parsed.
  pub fn parse_next_event(&mut self) -> Result<FfmpegEvent, String> {
    let mut buf = String::new();
    let bytes_read = self.reader.read_line(&mut buf);
    let line = buf.as_str();
    match bytes_read {
      Ok(0) => Err("EOF".to_string()),
      Ok(_) => {
        if let Some(version) = try_parse_version(line) {
          Ok(FfmpegEvent::ParsedVersion(FfmpegVersion {
            version,
            raw_log_message: line.to_string(),
          }))
        } else if let Some(configuration) = try_parse_configuration(line) {
          Ok(FfmpegEvent::ParsedConfiguration(FfmpegConfiguration {
            configuration,
            raw_log_message: line.to_string(),
          }))
        } else if line.starts_with("[info]") {
          Ok(FfmpegEvent::LogInfo(line.to_string()))
        } else if line.starts_with("[warning]") {
          Ok(FfmpegEvent::LogWarning(line.to_string()))
        } else if line.starts_with("[error]") || line.starts_with("[fatal]") {
          Ok(FfmpegEvent::LogError(line.to_string()))
        } else {
          Ok(FfmpegEvent::LogUnknown(line.to_string()))
        }
      }
      Err(e) => Err(e.to_string()),
    }
  }

  pub fn new(inner: R) -> Self {
    Self {
      reader: BufReader::new(inner),
    }
  }
}

/// Parses the ffmpeg version string from the stderr stream,
/// typically the very first line of output:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_version;
///
/// let line = "ffmpeg version 2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev Copyright (c) 2000-2023 the FFmpeg developers\n";
///
/// let version = try_parse_version(line).unwrap();
///
/// assert!(version == "2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev");
/// ```
pub fn try_parse_version(mut string: &str) -> Option<String> {
  if string.starts_with("[info]") {
    string = &string[6..];
  }
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
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_configuration;
///
/// let line = "configuration: --enable-gpl --enable-version3 --enable-static\n";
/// // Typically much longer, 20-30+ flags
///
/// let version = try_parse_configuration(line).unwrap();
///
/// assert!(version.len() == 3);
/// assert!(version[0] == "--enable-gpl");
/// assert!(version[1] == "--enable-version3");
/// assert!(version[2] == "--enable-static");
/// ```
///
pub fn try_parse_configuration(mut string: &str) -> Option<Vec<String>> {
  if string.starts_with("[info]") {
    string = &string[6..];
  }
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
    let mut parser = FfmpegLogParser::new(stdout);
    while let Ok(event) = parser.parse_next_event() {
      match event {
        FfmpegEvent::ParsedVersion(_) => return,
        _ => {}
      }
    }
    panic!() // should have found a version
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
    let mut parser = FfmpegLogParser::new(stdout);
    while let Ok(event) = parser.parse_next_event() {
      match event {
        FfmpegEvent::ParsedConfiguration(_) => return,
        _ => {}
      }
    }
    panic!() // should have found a configuration
  }
}
