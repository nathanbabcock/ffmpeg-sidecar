use anyhow::Context;

use crate::{
  event::FfmpegEvent,
  log_parser::FfmpegLogParser,
  paths::ffmpeg_path,
};
use std::ffi::OsStr;
use std::process::{Command, Stdio};

/// Alias for `ffmpeg -version`, parsing the version number and returning it.
pub fn ffmpeg_version() -> anyhow::Result<String> {
  ffmpeg_version_with_path(ffmpeg_path())
}

/// Lower level variant of `ffmpeg_version` that exposes a customized the path
/// to the ffmpeg binary.
pub fn ffmpeg_version_with_path<S: AsRef<OsStr>>(path: S) -> anyhow::Result<String> {
  let mut cmd = Command::new(&path)
    .arg("-version")
    .stdout(Stdio::piped()) // not stderr when calling `-version`
    .spawn()?;
  let stdout = cmd.stdout.take().context("No standard output channel")?;
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
    anyhow::bail!("ffmpeg -version exited with non-zero status");
  }
  version.context("Failed to parse ffmpeg version")
}
