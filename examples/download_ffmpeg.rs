use ffmpeg_sidecar::{
  command::ffmpeg_is_installed,
  download::{check_latest_version, download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
  error::Result,
  paths::sidecar_dir,
  version::ffmpeg_version,
};

fn main() -> Result<()> {
  if ffmpeg_is_installed() {
    println!("FFmpeg is already installed! 🎉");
    println!("For demo purposes, we'll re-download and unpack it anyway.");
    println!("TIP: Use `auto_download()` to skip manual customization.");
  }

  // Short version without customization:
  // ```rust
  // ffmpeg_sidecar::download::auto_download().unwrap();
  // ```

  // Checking the version number before downloading is actually not necessary,
  // but it's a good way to check that the download URL is correct.
  match check_latest_version() {
    Ok(version) => println!("Latest available version: {}", version),
    Err(_) => println!("Skipping version check on this platform."),
  }

  // These defaults will automatically select the correct download URL for your
  // platform.
  let download_url = ffmpeg_download_url()?;
  let destination = sidecar_dir()?;

  // By default the download will use a `curl` command. You could also write
  // your own download function and use another package like `reqwest` instead.
  let archive_path = download_ffmpeg_package(download_url, &destination)?;
  println!("Downloaded package: {:?}", archive_path);

  // Extraction uses `tar` on all platforms (available in Windows since version 1803)
  println!("Extracting...");
  unpack_ffmpeg(&archive_path, &destination)?;

  // Use the freshly installed FFmpeg to check the version number
  let version = ffmpeg_version()?;
  println!("FFmpeg version: {}", version);

  println!("Done! 🏁");
  Ok(())
}
