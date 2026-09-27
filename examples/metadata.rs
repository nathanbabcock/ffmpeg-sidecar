use ffmpeg_sidecar::{
  command::FfmpegCommand,
  event::{FfmpegEvent, FfmpegProgress},
};

/// Add metadata to a video file, with progress updates and FFmpeg log output.
fn main() {
  let mut ffmpeg_runner = FfmpegCommand::new()
    .testsrc()
    .args(["-metadata", "title=some cool title"])
    .overwrite() // -y
    .output("output/metadata.mp4")
    .print_command()
    .spawn()
    .unwrap();

  ffmpeg_runner.iter().unwrap().for_each(|e| match e {
    FfmpegEvent::Progress(FfmpegProgress { frame, .. }) => println!("Current frame: {frame}"),
    FfmpegEvent::Log(_level, msg) => println!("[ffmpeg] {msg}"),
    _ => {}
  });
}
