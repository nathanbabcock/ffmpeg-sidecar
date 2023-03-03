use ffmpeg_sidecar::auto_download::{check_latest_version, download_ffmpeg};

fn main() {
  let latest_version = check_latest_version().unwrap();
  println!("{}", latest_version);

  let filename = download_ffmpeg().unwrap();
  println!("{}", filename);
}
