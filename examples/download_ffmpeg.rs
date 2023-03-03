use std::fs::rename;

use ffmpeg_sidecar::auto_download::{check_latest_version, download_ffmpeg, unpack_ffmpeg};

fn main() {
  // let latest_version = check_latest_version().unwrap();
  // println!("{}", latest_version);

  // let filename = download_ffmpeg().unwrap();
  // println!("{}", filename);

  unpack_ffmpeg("ffmpeg-release-essentials.zip").unwrap();

  // rename("temp/bop.txt", "bop.txt").unwrap();
}
