use ffmpeg_sidecar::auto_download::check_latest_version;

fn main() {
  println!("{}", check_latest_version().unwrap());
}
