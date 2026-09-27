use ffmpeg_sidecar::command::FfmpegCommand;

/// Iterates over the frames of a `testsrc`.
///
/// ```console
/// cargo run --example hello_world
/// ```
fn main() -> anyhow::Result<()> {
  // Run an FFmpeg command that generates a test video
  let iter = FfmpegCommand::new() // <- Builder API like `std::process::Command`
    .testsrc() // <- Discoverable aliases for FFmpeg args
    .rawvideo() // <- Convenient argument presets
    .spawn()? // <- Ordinary `std::process::Child`
    .iter()?; // <- Blocking iterator over logs and output

  // Use a regular "for" loop to read decoded video data
  for frame in iter.filter_frames() {
    println!("frame: {}x{}", frame.width, frame.height);
    let _pixels: Vec<u8> = frame.data; // <- raw RGB pixels! 🎨
  }

  Ok(())
}
