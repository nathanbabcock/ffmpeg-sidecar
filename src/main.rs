pub mod stderr_parser;

use ffmpeg_sidecar::ffmpeg::FfmpegSidecar;

pub fn main() -> Result<(), String> {
  FfmpegSidecar::new().testsrc().pipe_stdout().run()
}
