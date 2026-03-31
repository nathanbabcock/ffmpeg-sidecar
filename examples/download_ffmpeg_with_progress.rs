use ffmpeg_sidecar::download::{
  download_ffmpeg_package_with_progress, ffmpeg_download_url, unpack_ffmpeg,
  unpack_ffmpeg_without_extras, FfmpegDownloadProgressEvent,
};
use ffmpeg_sidecar::version::ffmpeg_version;
use std::io::Write;

#[cfg(feature = "download_ffmpeg")]
fn main() -> anyhow::Result<()> {
  use ffmpeg_sidecar::command::ffmpeg_is_installed;

  if ffmpeg_is_installed() {
    println!("FFmpeg is already installed! 🎉");
    println!("For demo purposes, we'll re-download and unpack it anyway.");
    println!(
      "TIP: Use `auto_download_with_progress(progress_callback)` to skip manual customization."
    );
  }

  let progress_callback = |e: FfmpegDownloadProgressEvent| match e {
    FfmpegDownloadProgressEvent::Starting => {
      println!("Starting download...");
    }
    FfmpegDownloadProgressEvent::Downloading {
      downloaded_bytes,
      total_bytes,
    } => {
      print!(
        "\rDownloaded {:.1}/{:.1} mB    ",
        downloaded_bytes as f64 / 1024.0 / 1024.0,
        total_bytes as f64 / 1024.0 / 1024.0
      );
      std::io::stdout().flush().unwrap();
    }
    FfmpegDownloadProgressEvent::UnpackingArchive => {
      println!("\nUnpacking archive...");
    }
    FfmpegDownloadProgressEvent::Done => {
      println!("Ffmpeg downloaded successfully!")
    }
  };

  force_download_with_progress(progress_callback)?;

  let version = ffmpeg_version()?;
  println!("FFmpeg version: {version}");
  Ok(())
}

#[cfg(feature = "download_ffmpeg")]
pub fn force_download_with_progress(
  progress_callback: impl Fn(FfmpegDownloadProgressEvent),
) -> anyhow::Result<()> {
  use ffmpeg_sidecar::{command::ffmpeg_is_installed, paths::sidecar_dir};
  use std::env::var;

  progress_callback(FfmpegDownloadProgressEvent::Starting);
  let download_url = ffmpeg_download_url()?;
  let destination = sidecar_dir()?;
  let archive_path =
    download_ffmpeg_package_with_progress(download_url, &destination, |e| progress_callback(e))?;
  progress_callback(FfmpegDownloadProgressEvent::UnpackingArchive);
  let keep_only_ffmpeg = var("KEEP_ONLY_FFMPEG")
    .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
    .unwrap_or(false);

  if keep_only_ffmpeg {
    println!("KEEP_ONLY_FFMPEG is set, skipping ffplay and ffprobe.");
    unpack_ffmpeg_without_extras(&archive_path, &destination)?;
  } else {
    unpack_ffmpeg(&archive_path, &destination)?;
  }
  progress_callback(FfmpegDownloadProgressEvent::Done);

  if !ffmpeg_is_installed() {
    anyhow::bail!("FFmpeg failed to install, please install manually.");
  }

  Ok(())
}

#[cfg(not(feature = "download_ffmpeg"))]
fn main() {
  eprintln!(r#"This example requires the "download_ffmpeg" feature to be enabled."#);
  println!("The feature is included by default unless manually disabled.");
  println!("Please run `cargo run --example download_ffmpeg`.");
}
