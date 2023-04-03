use std::{
  fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename},
  io::Read,
  path::{Path, PathBuf},
  process::{Command, ExitStatus, Stdio},
};

use crate::{
  command::ffmpeg_is_installed,
  error::{Error, Result},
  paths::sidecar_dir,
};

pub const UNPACK_DIRNAME: &str = "ffmpeg_release_temp";

/// URL of a manifest file containing the latest published build of FFmpeg. The
/// correct URL for the target platform is baked in at compile time.
pub fn ffmpeg_manifest_url() -> Result<&'static str> {
  if cfg!(not(target_arch = "x86_64")) {
    return Err(Error::msg(
      "Downloads must be manually provided for non-x86_64 architectures",
    ));
  }

  if cfg!(target_os = "windows") {
    Ok("https://www.gyan.dev/ffmpeg/builds/release-version")
  } else if cfg!(target_os = "macos") {
    Ok("https://evermeet.cx/ffmpeg/info/ffmpeg/release")
  } else if cfg!(target_os = "linux") {
    Ok("https://johnvansickle.com/ffmpeg/release-readme.txt")
  } else {
    Err(Error::msg("Unsupported platform"))
  }
}

/// URL for the latest published FFmpeg release. The correct URL for the target
/// platform is baked in at compile time.
pub fn ffmpeg_download_url() -> Result<&'static str> {
  if cfg!(not(target_arch = "x86_64")) {
    return Err(Error::msg(
      "Downloads must be manually provided for non-x86_64 architectures",
    ));
  }

  if cfg!(target_os = "windows") {
    Ok("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip")
  } else if cfg!(target_os = "macos") {
    Ok("https://evermeet.cx/ffmpeg/getrelease")
  } else if cfg!(target_os = "linux") {
    Ok("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz")
  } else {
    Err(Error::msg("Unsupported platform"))
  }
}

/// Check if FFmpeg is installed, and if it's not, download and unpack it.
/// Automatically selects the correct binaries for Windows, Linux, and MacOS.
/// The binaries will be placed in the same directory as the Rust executable.
///
/// If FFmpeg is already installed, the method exits early without downloading
/// anything.
pub fn auto_download() -> Result<()> {
  if ffmpeg_is_installed() {
    return Ok(());
  }

  let download_url = ffmpeg_download_url()?;
  let destination = sidecar_dir()?;
  let archive_path = download_ffmpeg_package(download_url, &destination)?;
  unpack_ffmpeg(&archive_path, &destination)?;

  match ffmpeg_is_installed() {
    false => Err(Error::msg(
      "FFmpeg failed to install, please install manually.",
    )),
    true => Ok(()),
  }
}

/// Parse the the MacOS version number from a JSON string manifest file.
///
/// Example input: https://evermeet.cx/ffmpeg/info/ffmpeg/release
///
/// ```rust
/// use ffmpeg_sidecar::download::parse_macos_version;
/// let json_string = "{\"name\":\"ffmpeg\",\"type\":\"release\",\"version\":\"6.0\",...}";
/// let parsed = parse_macos_version(&json_string).unwrap();
/// assert!(parsed == "6.0");
/// ```
pub fn parse_macos_version(version: &str) -> Option<String> {
  version
    .split("\"version\":")
    .nth(1)?
    .trim()
    .split('\"')
    .nth(1)
    .map(|s| s.to_string())
}

/// Parse the the Linux version number from a long manifest text file.
///
/// Example input: https://johnvansickle.com/ffmpeg/release-readme.txt
///
/// ```rust
/// use ffmpeg_sidecar::download::parse_linux_version;
/// let json_string = "build: ffmpeg-5.1.1-amd64-static.tar.xz\nversion: 5.1.1\n\ngcc: 8.3.0";
/// let parsed = parse_linux_version(&json_string).unwrap();
/// assert!(parsed == "5.1.1");
/// ```
pub fn parse_linux_version(version: &str) -> Option<String> {
  version
    .split("version:")
    .nth(1)?
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
    .ok_or_else(|| Error::msg("Failed to get stdout"))?;

  let mut string = String::new();
  std::io::BufReader::new(stdout).read_to_string(&mut string)?;
  Ok(string)
}

/// Invoke cURL on the command line to download a file, writing to a file.
pub fn curl_to_file(url: &str, destination: &str) -> Result<ExitStatus> {
  Command::new("curl")
    .args(["-L", url])
    .args(["-o", destination])
    .status()
    .map_err(Error::from)
}

/// Makes an HTTP request to obtain the latest version available online,
/// automatically choosing the correct URL for the current platform.
pub fn check_latest_version() -> Result<String> {
  let string = curl(ffmpeg_manifest_url()?)?;

  if cfg!(target_os = "windows") {
    Ok(string)
  } else if cfg!(target_os = "macos") {
    Ok(parse_macos_version(&string).ok_or("failed to parse version number (macos variant)")?)
  } else if cfg!(target_os = "linux") {
    Ok(parse_linux_version(&string).ok_or("failed to parse version number (linux variant)")?)
  } else {
    Err(Error::msg("Unsupported platform"))
  }
}

/// Invoke `curl` to download an archive (ZIP on windows, TAR on linux and mac)
/// from the latest published release online.
pub fn download_ffmpeg_package(url: &str, download_dir: &Path) -> Result<PathBuf> {
  let filename = Path::new(url)
    .file_name()
    .ok_or_else(|| Error::msg("Failed to get filename"))?;

  let archive_path = download_dir.join(filename);

  let archive_filename = archive_path.to_str().ok_or("invalid download path")?;

  let exit_status = curl_to_file(url, archive_filename)?;

  if !exit_status.success() {
    return Err(Error::msg("Failed to download ffmpeg"));
  }

  Ok(archive_path)
}

/// After downloading, unpacks the archive to a folder, moves the binaries to
/// their final location, and deletes the archive and temporary folder.
pub fn unpack_ffmpeg(from_archive: &PathBuf, binary_folder: &Path) -> Result<()> {
  let temp_dirname = UNPACK_DIRNAME;
  let temp_folder = binary_folder.join(temp_dirname);
  create_dir_all(&temp_folder)?;

  // Extract archive
  Command::new("tar")
    .arg("-xf")
    .arg(from_archive)
    .current_dir(&temp_folder)
    .status()?
    .success()
    .then_some(())
    .ok_or("Failed to unpack ffmpeg")?;

  // Move binaries
  let inner_folder = read_dir(&temp_folder)?
    .next()
    .ok_or("Failed to get inner folder")??;

  if !inner_folder.file_type()?.is_dir() {
    return Err(Error::msg("No top level directory inside archive"));
  }

  let (ffmpeg, ffplay, ffprobe) = if cfg!(target_os = "windows") {
    (
      inner_folder.path().join("bin/ffmpeg.exe"),
      inner_folder.path().join("bin/ffplay.exe"),
      inner_folder.path().join("bin/ffprobe.exe"),
    )
  } else if cfg!(any(target_os = "linux", target_os = "macos")) {
    (
      inner_folder.path().join("./ffmpeg"),
      inner_folder.path().join("./ffplay"), // <- this typically only exists in Windows builds
      inner_folder.path().join("./ffprobe"),
    )
  } else {
    return Err(Error::msg("Unsupported platform"));
  };

  // Move binaries
  rename(&ffmpeg, binary_folder.join(ffmpeg.file_name().ok_or(())?))?;

  if ffprobe.exists() {
    rename(&ffprobe, binary_folder.join(ffprobe.file_name().ok_or(())?))?;
  }

  if ffplay.exists() {
    rename(&ffplay, binary_folder.join(ffplay.file_name().ok_or(())?))?;
  }

  // Delete archive and unpacked files
  if temp_folder.exists() {
    remove_dir_all(&temp_folder)?;
  }

  if from_archive.exists() {
    remove_file(from_archive)?;
  }

  Ok(())
}
