use crate::{
  error::{Error, Result},
  event::FfmpegEvent,
  log_parser::FfmpegLogParser,
  paths::sidecar_path,
};
use std::{ffi::OsStr, path::PathBuf};
use std::{
  path::Path,
  process::{Command, Stdio},
};

/// Returns the path of the downloaded FFprobe executable, or falls back to
/// assuming its installed in the system path. Note that not all FFmpeg
/// distributions include FFprobe.
pub fn ffprobe_path() -> PathBuf {
  let default = Path::new("ffprobe").to_path_buf();
  match sidecar_path() {
    Ok(sidecar_path) => match sidecar_path.exists() {
      true => sidecar_path,
      false => default,
    },
    Err(_) => default,
  }
}

/// Alias for `ffprobe -version`, parsing the version number and returning it.
pub fn ffprobe_version() -> Result<String> {
  ffprobe_version_with_path(ffprobe_path())
}

/// Lower level variant of `ffprobe_version` that exposes a customized the path
/// to the ffmpeg binary.
pub fn ffprobe_version_with_path<S: AsRef<OsStr>>(path: S) -> Result<String> {
  let mut cmd = Command::new(&path)
    .arg("-version")
    .stdout(Stdio::piped()) // not stderr when calling `-version`
    .spawn()?;
  let stdout = cmd.stdout.take().ok_or("No standard output channel")?;
  let mut parser = FfmpegLogParser::new(stdout);

  let mut version: Option<String> = None;
  while let Ok(event) = parser.parse_next_event() {
    match event {
      FfmpegEvent::ParsedVersion(v) => version = Some(v.version),
      FfmpegEvent::LogEOF => break,
      _ => {}
    }
  }
  let exit_status = cmd.wait()?;
  if !exit_status.success() {
    return Err(Error::msg("ffprobe -version exited with non-zero status"));
  }
  version.ok_or_else(|| Error::msg("Failed to parse ffprobe version"))
}

/// Verify whether ffprobe is installed on the system. This will return true if
/// there is an ffprobe binary in the PATH, or in the same directory as the Rust
/// executable.
pub fn ffprobe_is_installed() -> bool {
  Command::new(ffprobe_path())
    .arg("-version")
    .stderr(Stdio::null())
    .stdout(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or_else(|_| false)
}
