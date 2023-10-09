use ffmpeg_sidecar::{download::auto_download, ffprobe::ffprobe_version};

fn main() {
  // Download ffprobe from a configured source.
  // Note that not all distributions include ffprobe in their bundle.
  auto_download().unwrap();

  // Try running the executable and printing the version number.
  let version = ffprobe_version().unwrap();
  println!("ffprobe version: {}", version);
}
