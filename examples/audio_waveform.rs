use anyhow::{Context, Result};
use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};

pub fn main() -> Result<()> {
  const AUDIO_DEVICE_FORMAT: &str = "dshow";

  if cfg!(not(windows)) {
    println!("This code hasn't been tested on non-Windows platforms");
    println!("You will likely need to change the `AUDIO_DEVICE_FORMAT` constant");
    println!("Pull requests are welcome to make this demo cross-platform!");
  }

  // First run a `-list_devices` command to determine the audio device
  // Automatically selects the first one found
  // Sample stderr output: [dshow @ 000001c9babdb000] "Headset Microphone (Arctis 7 Chat)" (audio)

  let audio_device = FfmpegCommand::new()
    .hide_banner()
    .args(&["-list_devices", "true"])
    .format(AUDIO_DEVICE_FORMAT)
    .input("dummy")
    .print_command()
    .spawn()?
    .iter()?
    .into_ffmpeg_stderr()
    .find(|line| line.contains("(audio)"))
    .map(|line| {
      // find the part in quotes
      line.split('\"').nth(1).map(|s| s.to_string())
    })
    .context("No audio device found")?
    .context("Failed to parse audio device")?;

  println!("Found audio device: {}", audio_device);

  // Careful: don't include quotes in the device string!
  // https://github.com/fluent-ffmpeg/node-fluent-ffmpeg/issues/648#issuecomment-866242144
  let audio_device_str = format!("audio={audio_device}");

  // Capture audio with FFmpeg
  let iter = FfmpegCommand::new()
    .format(AUDIO_DEVICE_FORMAT)
    .input(audio_device_str)
    .format("s16le")
    .args(&["-ac", "1"]) // Mono audio
    .codec_audio("pcm_s16le")
    .args(&["-ar", "44100"]) // Sample rate 44.1kHz
    .pipe_stdout()
    .print_command()
    .spawn()?
    .iter()?;

  // todo: works when sent to file, and when command copy-pasted to CLI, but not here

  for event in iter {
    match event {
      FfmpegEvent::Error(e) | FfmpegEvent::Log(_, e) => {
        // todo: create `.inspect_err()` iter to simplify this pattern
        // todo: prevent reading newline events as Unknown
        println!("{e}");
      }
      FfmpegEvent::OutputChunk(chunk) => {
        println!("chunk: {}", chunk.len());
      }
      _ => {}
    }
  }

  Ok(())
}
