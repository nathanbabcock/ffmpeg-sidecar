use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};

fn main() {
  let mut chunks = 0;
  let mut frames = 0;

  FfmpegCommand::new()
    .testsrc()
    .codec_video("libx264")
    .format("mpegts")
    .pipe_stdout()
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| match e {
      FfmpegEvent::OutputChunk(_) => chunks += 1,
      FfmpegEvent::OutputFrame(_) => frames += 1,
      _ => {}
    });

  assert!(chunks > 0);
}
