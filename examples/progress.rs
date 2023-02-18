use ffmpeg_sidecar::command::FfmpegCommand;

/// Output progress events from a standard ffmpeg command
/// which writes to a file.
fn main() {
  let fps = 60;
  let duration = 10;
  let total_frames = fps * duration;
  let arg_string = format!(
    "-f lavfi -i testsrc=duration={}:size=1920x1080:rate={} -y output/test.mp4",
    duration, fps
  );
  FfmpegCommand::new()
    .args(arg_string.split(' '))
    .spawn()
    .unwrap()
    .events_iter()
    .unwrap()
    .filter_progress()
    .for_each(|progress| println!("{}%", (progress.frame * 100) / total_frames));
}
