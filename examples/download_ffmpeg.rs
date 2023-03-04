use ffmpeg_sidecar::{
  download::{check_latest_version, download_ffmpeg_package, ffmpeg_is_installed, unpack_ffmpeg},
  version::ffmpeg_version,
};

fn main() {
  if ffmpeg_is_installed() {
    println!("FFmpeg is already installed! 🎉");
    println!("For demo purposes, we'll re-download and unpack it anyway.");
  }

  let latest_version = check_latest_version().unwrap();
  println!("Latest available version: {}", latest_version);

  let archive_path = download_ffmpeg_package().unwrap();
  println!("Downloaded package: {:?}", archive_path);

  println!("Extracting...");
  unpack_ffmpeg(&archive_path).unwrap();

  let version = ffmpeg_version().unwrap();
  println!("Freshly installed FFmpeg version: {}", version);

  println!("Done! ✨");
}
