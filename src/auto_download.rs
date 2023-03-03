use std::{
  env::{consts::OS, current_dir},
  fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename},
  io::Read,
  path::Path,
  process::{Command, ExitStatus, Stdio},
};

use crate::error::{Error, Result};

pub const LINUX_VERSION: &str = "https://johnvansickle.com/ffmpeg/release-readme.txt";
pub const WINDOWS_VERSION: &str = "https://www.gyan.dev/ffmpeg/builds/release-version";
pub const MACOS_VERSION: &str = "https://evermeet.cx/ffmpeg/info/ffmpeg/release";

pub const LINUX_DOWNLOAD: &str =
  "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz";
pub const WINDOWS_DOWNLOAD: &str =
  "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
pub const MACOS_DOWNLOAD: &str = "https://evermeet.cx/ffmpeg/getrelease";

pub const UNPACK_DIR: &str = "ffmpeg_release_temp";

/// Parse the the MacOS version number from a JSON string
///
/// Example input: https://evermeet.cx/ffmpeg/info/ffmpeg/release
///
/// ```rust
/// use ffmpeg_sidecar::auto_download::{curl, parse_macos_version, MACOS_VERSION};
/// let json_string = curl(MACOS_VERSION).unwrap();
/// assert!(parse_macos_version(&json_string).is_some());
/// ```
pub fn parse_macos_version(version: &str) -> Option<String> {
  version
    .split("\"version\":")
    .nth(1)?
    .trim()
    .split("\"")
    .next()
    .map(|s| s.to_string())
}

/// Parse the the Linux version number from a long text file.
///
/// Example input: https://johnvansickle.com/ffmpeg/release-readme.txt
///
/// ```rust
/// use ffmpeg_sidecar::auto_download::{curl, parse_linux_version, LINUX_VERSION};
/// let text_file = curl(LINUX_VERSION).unwrap();
/// assert!(parse_linux_version(&text_file).is_some());
/// ```
pub fn parse_linux_version(version: &str) -> Option<String> {
  version
    .split("version:")
    .nth(1)?
    .trim()
    .split_whitespace()
    .next()
    .map(|s| s.to_string())
}

/// Invoke cURL on the command line to download a file, returning it as a string.
pub fn curl(url: &str) -> Result<String> {
  let mut child = Command::new("curl")
    .args(["-L", url])
    .stderr(Stdio::null())
    .stdout(Stdio::piped())
    .spawn()?;

  let stdout = child
    .stdout
    .take()
    .ok_or(Error::msg("Failed to get stdout"))?;

  let mut string = String::new();
  std::io::BufReader::new(stdout).read_to_string(&mut string)?;
  Ok(string)
}

/// Invoke cURL on the command line to download a file, writing to a file.
pub fn curl_to_file(url: &str, filename: &str) -> Result<ExitStatus> {
  Command::new("curl")
    .args(["-L", url])
    .args(["-o", filename])
    .status()
    .map_err(Error::from)
}

/// Check the latest version available online
pub fn check_latest_version() -> Result<String> {
  let manifest_url = match OS {
    "linux" => Ok(LINUX_VERSION),
    "windows" => Ok(WINDOWS_VERSION),
    "macos" => Ok(MACOS_VERSION),
    _ => Err(Error::msg(format!("Unsupported platform: {}", OS))),
  }?;

  println!("Using url: {}", manifest_url);
  let string = curl(manifest_url)?;

  match OS {
    "linux" => Ok(parse_linux_version(&string).ok_or(Error::msg("failed to parse linux version"))?),
    "windows" => Ok(string),
    "macos" => Ok(parse_macos_version(&string).ok_or(Error::msg("failed to parse macos version"))?),
    _ => Err(Error::msg(format!("Unsupported platform: {}", OS))),
  }
}

pub fn get_download_url() -> Option<&'static str> {
  match OS {
    "linux" => Some(LINUX_DOWNLOAD),
    "windows" => Some(WINDOWS_DOWNLOAD),
    "macos" => Some(MACOS_DOWNLOAD),
    _ => None,
  }
}

pub fn download_ffmpeg() -> Result<String> {
  let url = get_download_url().ok_or(Error::msg("Unsupported platform"))?;

  let filename = Path::new(url)
    .file_name()
    .ok_or(Error::msg("Failed to get filename"))?;

  let exit_status = curl_to_file(
    url,
    filename
      .to_str()
      .ok_or(Error::msg("Failed to get filename"))?,
  )?;

  if !exit_status.success() {
    return Err(Error::msg("Failed to download ffmpeg"));
  }

  Ok(filename.to_string_lossy().to_string())
}

pub fn unpack_ffmpeg(filename: &str) -> Result<()> {
  create_dir_all(UNPACK_DIR)?;

  // Extract archive
  Command::new("tar")
    .arg("-xf")
    .arg(filename)
    .args(["-C", UNPACK_DIR])
    .status()?
    .success()
    .then_some(())
    .ok_or("Failed to unpack ffmpeg")?;

  // Move binaries
  let cwd = current_dir()?;
  let (ffmpeg, ffplay, ffprobe) = match OS {
    "windows" => {
      let inner_folder = read_dir(UNPACK_DIR)?
        .next()
        .ok_or("Failed to get inner folder")??;

      inner_folder
        .file_type()?
        .is_dir()
        .then_some(())
        .ok_or("No top level directory inside archive")?;

      (
        inner_folder.path().clone().join("bin/ffmpeg.exe"),
        inner_folder.path().clone().join("bin/ffplay.exe"),
        inner_folder.path().clone().join("bin/ffprobe.exe"),
      )
    }
    "linux" => todo!(),
    "macos" => todo!(),
    _ => return Err(Error::msg("Unsupported platform")),
  };

  rename(&ffmpeg, cwd.join(ffmpeg.file_name().ok_or(())?))?;
  rename(&ffplay, cwd.join(ffplay.file_name().ok_or(())?))?;
  rename(&ffprobe, cwd.join(ffprobe.file_name().ok_or(())?))?;

  remove_dir_all(UNPACK_DIR)?;
  remove_file(filename)?;

  Ok(())
}
