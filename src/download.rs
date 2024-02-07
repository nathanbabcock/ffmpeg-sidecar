use std::{
  fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename},
  io::Read,
  path::{Path, PathBuf},
  process::{Command, ExitStatus, Stdio},
};

use anyhow::Context;

use crate::{command::ffmpeg_is_installed, paths::sidecar_dir};

pub const UNPACK_DIRNAME: &str = "ffmpeg_release_temp";

/// URL of a manifest file containing the latest published build of FFmpeg. The
/// correct URL for the target platform is baked in at compile time.
pub fn ffmpeg_manifest_url() -> anyhow::Result<&'static str> {
  if cfg!(not(target_arch = "x86_64")) {
    anyhow::bail!("Downloads must be manually provided for non-x86_64 architectures");
  }

  if cfg!(target_os = "windows") {
    Ok("https://www.gyan.dev/ffmpeg/builds/release-version")
  } else if cfg!(target_os = "macos") {
    Ok("https://evermeet.cx/ffmpeg/info/ffmpeg/release")
  } else if cfg!(target_os = "linux") {
    Ok("https://johnvansickle.com/ffmpeg/release-readme.txt")
  } else {
    anyhow::bail!("Unsupported platform")
  }
}

/// URL for the latest published FFmpeg release. The correct URL for the target
/// platform is baked in at compile time.
pub fn ffmpeg_download_url() -> anyhow::Result<&'static str> {
  if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Ok("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip")
  } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Ok("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz")
  } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
    Ok("https://evermeet.cx/ffmpeg/getrelease")
  } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Ok("https://www.osxexperts.net/ffmpeg6arm.zip") // Mac M1
  } else {
    anyhow::bail!("Unsupported platform; you can provide your own URL instead and call download_ffmpeg_package directly.")
  }
}

/// Check if FFmpeg is installed, and if it's not, download and unpack it.
/// Automatically selects the correct binaries for Windows, Linux, and MacOS.
/// The binaries will be placed in the same directory as the Rust executable.
///
/// If FFmpeg is already installed, the method exits early without downloading
/// anything.
pub fn auto_download() -> anyhow::Result<()> {
  if ffmpeg_is_installed() {
    return Ok(());
  }

  let download_url = ffmpeg_download_url()?;
  let destination = sidecar_dir()?;
  let archive_path = download_ffmpeg_package(download_url, &destination)?;
  unpack_ffmpeg(&archive_path, &destination)?;

  if !ffmpeg_is_installed() {
    anyhow::bail!("FFmpeg failed to install, please install manually.");
  }

  Ok(())
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
pub fn curl(url: &str) -> anyhow::Result<String> {
  let mut child = Command::new("curl")
    .args(["-L", url])
    .stderr(Stdio::null())
    .stdout(Stdio::piped())
    .spawn()?;

  let stdout = child
    .stdout
    .take()
    .context("Failed to get stdout")?;

  let mut string = String::new();
  std::io::BufReader::new(stdout).read_to_string(&mut string)?;
  Ok(string)
}

/// Invoke cURL on the command line to download a file, writing to a file.
pub fn curl_to_file(url: &str, destination: &str) -> anyhow::Result<ExitStatus> {
  Command::new("curl")
    .args(["-L", url])
    .args(["-o", destination])
    .status()
    .map_err(Into::into)
}

/// Makes an HTTP request to obtain the latest version available online,
/// automatically choosing the correct URL for the current platform.
pub fn check_latest_version() -> anyhow::Result<String> {
  let string = curl(ffmpeg_manifest_url()?)?;

  if cfg!(target_os = "windows") {
    Ok(string)
  } else if cfg!(target_os = "macos") {
    parse_macos_version(&string).context("failed to parse version number (macos variant)")
  } else if cfg!(target_os = "linux") {
    parse_linux_version(&string).context("failed to parse version number (linux variant)")
  } else {
    Err(anyhow::Error::msg("Unsupported platform"))
  }
}

/// Invoke `curl` to download an archive (ZIP on windows, TAR on linux and mac)
/// from the latest published release online.
pub fn download_ffmpeg_package(url: &str, download_dir: &Path) -> anyhow::Result<PathBuf> {
  let filename = Path::new(url)
    .file_name()
    .context("Failed to get filename")?;

  let archive_path = download_dir.join(filename);

  let archive_filename = archive_path
    .to_str()
    .context("invalid download path")?;

  let exit_status = curl_to_file(url, archive_filename)?;

  if !exit_status.success() {
    anyhow::bail!("Failed to download ffmpeg");
  }

  Ok(archive_path)
}

/// After downloading, unpacks the archive to a folder, moves the binaries to
/// their final location, and deletes the archive and temporary folder.
pub fn unpack_ffmpeg(from_archive: &PathBuf, binary_folder: &Path) -> anyhow::Result<()> {
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
    .context("Failed to unpack ffmpeg")?;

  // Move binaries
  let (ffmpeg, ffplay, ffprobe) = if cfg!(target_os = "windows") {
    let inner_folder = read_dir(&temp_folder)?
      .next()
      .context("Failed to get inner folder")??;
    (
      inner_folder.path().join("bin/ffmpeg.exe"),
      inner_folder.path().join("bin/ffplay.exe"),
      inner_folder.path().join("bin/ffprobe.exe"),
    )
  } else if cfg!(target_os = "linux") {
    let inner_folder = read_dir(&temp_folder)?
      .next()
      .context("Failed to get inner folder")??;
    (
      inner_folder.path().join("./ffmpeg"),
      inner_folder.path().join("./ffplay"), // <- no ffplay on linux
      inner_folder.path().join("./ffprobe"),
    )
  } else if cfg!(target_os = "macos") {
    (
      temp_folder.join("ffmpeg"),
      temp_folder.join("ffplay"),  // <-- no ffplay on mac
      temp_folder.join("ffprobe"), // <-- no ffprobe on mac
    )
  } else {
    anyhow::bail!("Unsupported platform");
  };

  // Move binaries
  let move_bin = |path: &Path| {
    let file_name = binary_folder.join(
      path
        .file_name()
        .with_context(|| format!("Path {} does not have a file_name", path.to_string_lossy()))?,
    );
    rename(path, file_name)?;
    anyhow::Ok(())
  };

  move_bin(&ffmpeg)?;

  if ffprobe.exists() {
    move_bin(&ffprobe)?;
  }

  if ffplay.exists() {
    move_bin(&ffplay)?;
  }

  // Delete archive and unpacked files
  if temp_folder.exists() && temp_folder.is_dir() {
    remove_dir_all(&temp_folder)?;
  }

  if from_archive.exists() {
    remove_file(from_archive)?;
  }

  Ok(())
}
