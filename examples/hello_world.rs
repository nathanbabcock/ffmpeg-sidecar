use ffmpeg_sidecar::{command::FfmpegCommand, error::Result, event::FfmpegEvent};

/// Iterates over the frames of a `testsrc`.
///
/// ```console
/// cargo run --example hello_world
/// ```
fn main() -> Result<()> {
  FfmpegCommand::new() // <- Builder API like `std::process::Command`
    .testsrc() // <- Discoverable aliases for FFmpeg args
    .rawvideo() // <- Convenient argument presets
    .spawn()? // <- Uses an ordinary `std::process::Child`
    .iter()? // <- Iterator over all log messages and video output
    .for_each(|event: FfmpegEvent| {
      match event {
        FfmpegEvent::OutputFrame(frame) => {
          println!("frame: {}x{}", frame.width, frame.height);
          let _pixels: Vec<u8> = frame.data; // <- raw RGB pixels! 🎨
        }
        FfmpegEvent::Progress(progress) => {
          eprintln!("Current speed: {}x", progress.speed); // <- parsed progress updates
        }
        FfmpegEvent::Log(_level, msg) => {
          eprintln!("[ffmpeg] {}", msg); // <- granular log message from stderr
        }
        _ => {}
      }
    });
  Ok(())
}
