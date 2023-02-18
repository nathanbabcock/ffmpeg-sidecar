use ffmpeg_sidecar::{
  child::FfmpegChild, command::FfmpegCommand, event::FfmpegEvent, iter::FfmpegIterator,
};

/// Iterates over the frames of a testsrc.
fn main() {
  // similar to `std::process::Command`
  let mut command = FfmpegCommand::new();
  command
    .testsrc() // generate a test pattern video
    .rawvideo(); // pipe raw video output

  // similar to `std::process::Child`
  let mut child: FfmpegChild = command.spawn().unwrap();

  // Iterator over all messages and output
  let iter: FfmpegIterator = child.iter().unwrap();
  iter.for_each(|event: FfmpegEvent| {
    match event {
      FfmpegEvent::OutputFrame(frame) => {
        let _pixels = frame.data; // <- raw RGB pixels! 🎨
      }
      FfmpegEvent::Error(e) => eprintln!("Error: {}", e),
      _ => {}
    }
  });
}
