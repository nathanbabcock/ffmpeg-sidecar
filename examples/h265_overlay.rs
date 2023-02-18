use std::path::Path;

use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};

/// 1. Read an H265 source video from file
/// 2. Decode video
/// 3. Composite with an overlay image rendered by Rust
/// 4. Re-encode back to H265 to file
fn main() {
  // Create an H265 source video as a starting point
  let input_path = "output/h265.mp4";
  if !Path::new(input_path).exists() {
    create_h265_source(input_path);
  }
}

/// Create a H265 source video from scratch
fn create_h265_source(path_str: &str) {
  println!("Creating H265 source video: {}", path_str);
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=size=600x800:rate=30:duration=15 -c:v libx265".split(' '))
    .arg(path_str)
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| match e {
      FfmpegEvent::LogError(e) => println!("Error: {}", e),
      FfmpegEvent::Progress(p) => println!("Progress: {} / 00:00:10", p.time),
      _ => {}
    });
  println!("Created H265 source video: {}", path_str);
}
