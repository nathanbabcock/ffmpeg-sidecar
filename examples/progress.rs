use ffmpeg_sidecar::command::FfmpegCommand;

/// Output progress events from a standard ffmpeg command
/// which writes to a file.
///
/// ```console
/// cargo run --example progress
/// ```
fn main() {
  let fps = 60;
  let duration = 10;
  let total_frames = fps * duration;
  let arg_string = format!(
    "-f lavfi -i testsrc=duration={duration}:size=1920x1080:rate={fps} -y output/test.mp4"
  );
  FfmpegCommand::new()
    .args(arg_string.split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_progress()
    .for_each(|progress| println!("{}%", (progress.frame * 100) / total_frames));
}
