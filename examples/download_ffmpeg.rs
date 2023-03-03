use std::fs::rename;

use ffmpeg_sidecar::auto_download::{check_latest_version, download_ffmpeg, unpack_ffmpeg};

fn main() {
  let filename = download_ffmpeg().unwrap();
  unpack_ffmpeg(&filename).unwrap();
}
