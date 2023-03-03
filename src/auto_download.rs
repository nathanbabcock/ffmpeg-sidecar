use std::{
  io::Read,
  process::{Command, Stdio},
};

use crate::error::{Error, Result};

pub const LINUX_RELEASE: &str = "https://johnvansickle.com/ffmpeg/release-readme.txt";
pub const WINDOWS_RELEASE: &str = "https://www.gyan.dev/ffmpeg/builds/release-version";
pub const MACOS_RELEASE: &str = "https://evermeet.cx/ffmpeg/info/ffmpeg/release";

/// Parse the the MacOS version number from a JSON string
///
/// Example input: https://evermeet.cx/ffmpeg/info/ffmpeg/release
///
/// ```rust
/// use ffmpeg_sidecar::auto_download::{curl, parse_macos_version, MACOS_RELEASE};
/// let json_string = curl(MACOS_RELEASE).unwrap();
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
/// use ffmpeg_sidecar::auto_download::{curl, parse_linux_version, LINUX_RELEASE};
/// let text_file = curl(LINUX_RELEASE).unwrap();
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

/// Check the latest version available online
pub fn check_latest_version() -> Result<String> {
  let os = std::env::consts::OS;
  let manifest_url = match os {
    "linux" => Ok(LINUX_RELEASE),
    "windows" => Ok(WINDOWS_RELEASE),
    "macos" => Ok(MACOS_RELEASE),
    _ => Err(Error::msg(format!("Unsupported platform: {}", os))),
  }?;

  println!("Using url: {}", manifest_url);
  let string = curl(manifest_url)?;

  match os {
    "linux" => Ok(parse_linux_version(&string).ok_or(Error::msg("failed to parse linux version"))?),
    "windows" => Ok(string),
    "macos" => Ok(parse_macos_version(&string).ok_or(Error::msg("failed to parse macos version"))?),
    _ => Err(Error::msg(format!("Unsupported platform: {}", os))),
  }
}

// 2. curl -> release (+ download progress bar)
// 3. md5 checksum, unzip, move binaries, delete
// 4. check version
