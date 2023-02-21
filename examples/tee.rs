use ffmpeg_sidecar::{command::FfmpegCommand, error::Error};

fn main() -> Result<(), Error> {
  let _cmd = FfmpegCommand::new()
    .testsrc()
    .output("pipe")
    .spawn()?
    .iter()?;

  Ok(())
}
