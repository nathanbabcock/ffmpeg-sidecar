use std::{
  env::{
    consts::{ARCH, OS},
    current_exe,
  },
  fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename},
  io::Read,
  path::{Path, PathBuf},
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

pub const UNPACK_DIRNAME: &str = "ffmpeg_release_temp";

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

  let download_url = get_package_url()?;
  let destination = get_download_dir()?;
  let archive_path = download_ffmpeg_package(download_url, &destination)?;
  unpack_ffmpeg(&archive_path, &destination)?;

  match ffmpeg_is_installed() {
    false => Err(Error::msg(
      "FFmpeg failed to install, please install manually.",
    )),
    true => Ok(()),
  }
}

/// Parse the the MacOS version number from a JSON string
///
/// Example input: https://evermeet.cx/ffmpeg/info/ffmpeg/release
///
/// ```rust
/// use ffmpeg_sidecar::download::{curl, parse_macos_version, MACOS_VERSION};
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
/// use ffmpeg_sidecar::download::{curl, parse_linux_version, LINUX_VERSION};
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
pub fn curl_to_file(url: &str, destination: &str) -> Result<ExitStatus> {
  Command::new("curl")
    .args(["-L", url])
    .args(["-o", destination])
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

/// Gets the URL to the latest publish FFmpeg release, automatically detecting the platform.
pub fn get_package_url() -> Result<&'static str> {
  if ARCH != "x86_64" {
    return Err(Error::msg(format!("Unsupported architecture: {}", ARCH)));
  }

  match OS {
    "linux" => Ok(LINUX_DOWNLOAD),
    "windows" => Ok(WINDOWS_DOWNLOAD),
    "macos" => Ok(MACOS_DOWNLOAD),
    _ => Err(Error::msg(format!("Unsupported platform: {}", OS))),
  }
}

/// By default, downloads all temporary files to the same directory as the Rust executable.
pub fn get_download_dir() -> Result<PathBuf> {
  current_exe()?
    .parent()
    .ok_or_else(|| Error::from(()))
    .map(|p| p.to_owned())
}

/// Downloads an archive (ZIP on windows, TAR on linux and mac)
/// from the latest published release online.
pub fn download_ffmpeg_package(url: &str, download_dir: &PathBuf) -> Result<PathBuf> {
  let filename = Path::new(url)
    .file_name()
    .ok_or(Error::msg("Failed to get filename"))?;

  let archive_path = download_dir.join(filename);

  let archive_filename = archive_path.to_str().ok_or("invalid download path")?;

  let exit_status = curl_to_file(url, archive_filename)?;

  if !exit_status.success() {
    return Err(Error::msg("Failed to download ffmpeg"));
  }

  Ok(archive_path)
}

/// By default, extracts all temporary files to a folder in the same directory as the Rust executable.
pub fn get_unpack_dirname() -> PathBuf {
  Path::new(UNPACK_DIRNAME).to_owned()
}

/// After downloading, unpacks the archive to a folder, moves the binaries to
/// their final location, and deletes the archive and temporary folder.
pub fn unpack_ffmpeg(from_archive: &PathBuf, binary_folder: &PathBuf) -> Result<()> {
  let temp_dirname = UNPACK_DIRNAME;
  let temp_folder = binary_folder.join(temp_dirname);
  create_dir_all(&temp_folder)?;

  // Extract archive
  Command::new("tar")
    .arg("-xf")
    .arg(from_archive)
    // .arg("-C")
    // .arg(temp_dirname)
    .current_dir(&temp_folder)
    .status()?
    .success()
    .then_some(())
    .ok_or("Failed to unpack ffmpeg")?;

  // Move binaries
  let (ffmpeg, ffplay, ffprobe) = match OS {
    "windows" => {
      let inner_folder = read_dir(&temp_folder)?
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
    "linux" => todo!(), // PR's welcome here!
    "macos" => todo!(), // And here!
    _ => return Err(Error::msg(format!("Unsupported platform: {}", OS))),
  };

  // Move binaries
  rename(&ffmpeg, binary_folder.join(ffmpeg.file_name().ok_or(())?))?;
  rename(&ffplay, binary_folder.join(ffplay.file_name().ok_or(())?))?;
  rename(&ffprobe, binary_folder.join(ffprobe.file_name().ok_or(())?))?;

  // Delete archive and unpacked files
  remove_dir_all(&temp_folder)?;
  remove_file(from_archive)?;

  Ok(())
}

/// Verify whether ffmpeg is installed on the system. This will return true if
/// there is an ffmpeg binary in the PATH, or in the same directory as the Rust
/// executable.
pub fn ffmpeg_is_installed() -> bool {
  Command::new("ffmpeg")
    .arg("-version")
    .stderr(Stdio::null())
    .stdout(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or_else(|_| false)
}
