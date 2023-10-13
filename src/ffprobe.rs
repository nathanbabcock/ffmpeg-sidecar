use crate::error::{Error, Result};
use std::{env::current_exe, ffi::OsStr, path::PathBuf};
use std::{
  path::Path,
  process::{Command, Stdio},
};

/// Returns the path of the downloaded FFprobe executable, or falls back to
/// assuming its installed in the system path. Note that not all FFmpeg
/// distributions include FFprobe.
pub fn ffprobe_path() -> PathBuf {
  let default = Path::new("ffprobe").to_path_buf();
  match ffprobe_sidecar_path() {
    Ok(sidecar_path) => match sidecar_path.exists() {
      true => sidecar_path,
      false => default,
    },
    Err(_) => default,
  }
}

/// The (expected) path to an FFmpeg binary adjacent to the Rust binary.
///
/// The extension between platforms, with Windows using `.exe`, while Mac and
/// Linux have no extension.
pub fn ffprobe_sidecar_path() -> Result<PathBuf> {
  let mut path = current_exe()?
    .parent()
    .ok_or("Can't get parent of current_exe")?
    .join("ffprobe");
  if cfg!(windows) {
    path.set_extension("exe");
  }
  Ok(path)
}

/// Alias for `ffprobe -version`, parsing the version number and returning it.
pub fn ffprobe_version() -> Result<String> {
  ffprobe_version_with_path(ffprobe_path())
}

/// Lower level variant of `ffprobe_version` that exposes a customized the path
/// to the ffmpeg binary.
pub fn ffprobe_version_with_path<S: AsRef<OsStr>>(path: S) -> Result<String> {
  let output = Command::new(&path).arg("-version").output()?;

  // note:version parsing is not implemented for ffprobe

  String::from_utf8(output.stdout).map_err(Error::from)
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
