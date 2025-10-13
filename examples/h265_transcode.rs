use std::{io::Write, path::Path, thread};

use ffmpeg_sidecar::{
  command::FfmpegCommand,
  event::{FfmpegEvent, LogLevel},
};

/// 1. Read an H265 source video from file
/// 2. Decode video
/// 3. Composite with an overlay image rendered by Rust
/// 4. Re-encode back to H265 to file
///
/// ```console
/// cargo run --example h265_transcode
/// ```
fn main() {
  // Create an H265 source video as a starting point
  let input_path = "output/h265.mp4";
  if !Path::new(input_path).exists() {
    create_h265_source(input_path);
  }

  // One instance decodes H265 to raw frames
  let mut input = FfmpegCommand::new()
    .input(input_path)
    .rawvideo()
    .spawn()
    .unwrap();

  // Frames can be transformed by Iterator `.map()`.
  // This example is a no-op, with frames passed through unaltered.
  let transformed_frames = input.iter().unwrap().filter_frames();

  // You could easily add some "middleware" processing here:
  // - overlay or composite another RGB image (or even another Ffmpeg Iterator)
  // - apply a filter like blur or convolution
  // Note: some of these operations are also possible with FFmpeg's (somewhat arcane)
  // `filtergraph` API, but doing it in Rust gives you much finer-grained
  // control, debuggability, and modularity -- you can pull in any Rust crate
  // you need.

  // A second instance encodes the updated frames back to H265
  let mut output = FfmpegCommand::new()
    .args([
      "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", "600x800", "-r", "30",
    ]) // note: should be possible to infer these params from the source input stream
    .input("-")
    .args(["-c:v", "libx265"])
    .args(["-y", "output/h265_overlay.mp4"])
    .spawn()
    .unwrap();

  // Connect the two instances
  let mut stdin = output.take_stdin().unwrap();
  thread::spawn(move || {
    // `for_each` blocks through the end of the iterator,
    // so we run it in another thread.
    transformed_frames.for_each(|f| {
      stdin.write_all(&f.data).ok();
    });
  });

  // On the main thread, run the output instance to completion
  output.iter().unwrap().for_each(|e| match e {
    FfmpegEvent::Log(LogLevel::Error, e) => println!("Error: {e}"),
    FfmpegEvent::Progress(p) => println!("Progress: {} / 00:00:15", p.time),
    _ => {}
  });
}

/// Create a H265 source video from scratch
fn create_h265_source(path_str: &str) {
  println!("Creating H265 source video: {path_str}");
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=size=600x800:rate=30:duration=15 -c:v libx265".split(' '))
    .arg(path_str)
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| match e {
      FfmpegEvent::Log(LogLevel::Error, e) => println!("Error: {e}"),
      FfmpegEvent::Progress(p) => println!("Progress: {} / 00:00:15", p.time),
      _ => {}
    });
  println!("Created H265 source video: {path_str}");
}
