use ffmpeg_sidecar::download::{
  check_latest_version, download_ffmpeg_package, ffmpeg_is_installed, unpack_ffmpeg,
};

fn main() {
  if ffmpeg_is_installed() {
    println!("FFmpeg is already installed! 🎉");
    println!("For demo purposes, we'll re-download and unpack it anyway.");
  }

  // TODO: check if the 3 binary files are already present
  // TODO: if so, prompt to delete them

  let latest_version = check_latest_version().unwrap();
  println!("Latest available version: {}", latest_version);

  let filename = download_ffmpeg_package().unwrap();
  println!("Downloaded package: {}", filename);

  println!("Extracting...");
  unpack_ffmpeg(&filename).unwrap();

  // TODO: verify that the 3 binary files are present
  if !ffmpeg_is_installed() {
    panic!()
  }
  println!("Done! ✨");
}
