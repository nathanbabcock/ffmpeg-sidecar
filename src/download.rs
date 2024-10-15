use std::{
  fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename, File},
  io::{copy, Read},
  path::{Path, PathBuf},
  process::{Command, ExitStatus, Stdio},
};

use anyhow::Context;

use crate::{
  command::{ffmpeg_is_installed, BackgroundCommand},
  paths::sidecar_dir,
};

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
    Ok("https://evermeet.cx/ffmpeg/getrelease/zip")
  } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Ok("https://www.osxexperts.net/ffmpeg7arm.zip") // Mac M1
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
#[cfg(feature = "download_ffmpeg")]
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
#[deprecated]
pub fn curl(url: &str) -> anyhow::Result<String> {
  let mut child = Command::new("curl")
    .create_no_window()
    .args(["-L", url])
    .stderr(Stdio::null())
    .stdout(Stdio::piped())
    .spawn()?;

  let stdout = child.stdout.take().context("Failed to get stdout")?;

  let mut string = String::new();
  std::io::BufReader::new(stdout).read_to_string(&mut string)?;
  Ok(string)
}

/// Invoke cURL on the command line to download a file, writing to a file.
#[deprecated]
pub fn curl_to_file(url: &str, destination: &str) -> anyhow::Result<ExitStatus> {
  Command::new("curl")
    .create_no_window()
    .args(["-L", url])
    .args(["-o", destination])
    .status()
    .map_err(Into::into)
}

/// Makes an HTTP request to obtain the latest version available online,
/// automatically choosing the correct URL for the current platform.
#[cfg(feature = "download_ffmpeg")]
pub fn check_latest_version() -> anyhow::Result<String> {
  // Mac M1 doesn't have a manifest URL, so match the version provided in `ffmpeg_download_url`
  if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    return Ok("7.0".to_string());
  }

  let manifest_url = ffmpeg_manifest_url()?;
  let response = reqwest::blocking::get(manifest_url)
    .context("Failed to make a request for the latest version")?;

  if !response.status().is_success() {
    anyhow::bail!(
      "Failed to get the latest version, status: {}",
      response.status()
    );
  }

  let string = response.text().context("Failed to read response text")?;

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
#[cfg(feature = "download_ffmpeg")]
pub fn download_ffmpeg_package(url: &str, download_dir: &Path) -> anyhow::Result<PathBuf> {
  let filename = Path::new(url)
    .file_name()
    .context("Failed to get filename")?;

  let archive_path = download_dir.join(filename);

  let response =
    reqwest::blocking::get(url).context("Failed to make a request to download ffmpeg")?;

  if !response.status().is_success() {
    anyhow::bail!("Failed to download ffmpeg, status: {}", response.status());
  }

  let mut file =
    File::create(&archive_path).context("Failed to create file for ffmpeg download")?;

  copy(
    &mut response
      .bytes()
      .context("Failed to read response bytes")?
      .as_ref(),
    &mut file,
  )
  .context("Failed to write ffmpeg download to file")?;

  Ok(archive_path)
}

/// After downloading, unpacks the archive to a folder, moves the binaries to
/// their final location, and deletes the archive and temporary folder.
pub fn unpack_ffmpeg(from_archive: &PathBuf, binary_folder: &Path) -> anyhow::Result<()> {
  let temp_dirname = UNPACK_DIRNAME;
  let temp_folder = binary_folder.join(temp_dirname);
  create_dir_all(&temp_folder)?;

  // Extract archive using the tar crate
  let file = File::open(from_archive).context("Failed to open archive file")?;
  #[cfg(target_os = "linux")]
  {
    let tar_xz = xz2::read::XzDecoder::new(file); // Use XzDecoder to handle the .xz decompression
    let mut archive = tar::Archive::new(tar_xz);

    archive
      .unpack(&temp_folder)
      .context("Failed to unpack ffmpeg");
  }
  #[cfg(not(target_os = "linux"))]
  {
    let mut archive = zip::ZipArchive::new(file).context("Failed to read ZIP archive")?;
    archive
      .extract(&temp_folder)
      .context("Failed to unpack ffmpeg")?;
  }

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
