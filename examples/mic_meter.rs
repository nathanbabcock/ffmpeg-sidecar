use anyhow::{Context, Result};
use ffmpeg_sidecar::{
  command::FfmpegCommand,
  event::{FfmpegEvent, LogLevel},
};
use std::{cmp::max, iter::repeat};

/// Process microphone audio data in realtime and display a volume meter/level
/// indicator rendered to the terminal.
pub fn main() -> Result<()> {
  if cfg!(not(windows)) {
    eprintln!("Note: Methods for capturing audio are platform-specific and this demo is intended for Windows.");
    eprintln!("On Linux or Mac, you need to switch from the `dshow` format to a different one supported on your platform.");
    eprintln!("Make sure to also include format-specific arguments such as `-audio_buffer_size`.");
    eprintln!("Pull requests are welcome to make this demo cross-platform!");
  }

  // First step: find default audio input device
  // Runs an `ffmpeg -list_devices` command and selects the first one found
  // Sample log output: [dshow @ 000001c9babdb000] "Headset Microphone (Arctis 7 Chat)" (audio)

  let audio_device = FfmpegCommand::new()
    .hide_banner()
    .args(&["-list_devices", "true"])
    .format("dshow")
    .input("dummy")
    .spawn()?
    .iter()?
    .into_ffmpeg_stderr()
    .find(|line| line.contains("(audio)"))
    .map(|line| line.split('\"').nth(1).map(|s| s.to_string()))
    .context("No audio device found")?
    .context("Failed to parse audio device")?;

  println!("Listening to audio device: {}", audio_device);

  // Second step: Capture audio and analyze w/ `ebur128` audio filter
  // Loudness metadata will be printed to the FFmpeg logs
  // Docs: <https://ffmpeg.org/ffmpeg-filters.html#ebur128-1>

  let iter = FfmpegCommand::new()
    .format("dshow")
    .args("-audio_buffer_size 50".split(' ')) // reduces latency to 50ms (dshow-specific)
    .input(format!("audio={audio_device}"))
    .args("-af ebur128=metadata=1,ametadata=print".split(' '))
    .format("null")
    .output("-")
    .spawn()?
    .iter()?;

  // Note: even though the audio device name may have spaces, it should *not* be
  // in quotes (""). Quotes are only needed on the command line to separate
  // different arguments. Since Rust invokes the command directly without a
  // shell interpreter, args are already divided up correctly. Any quotes
  // would be included in the device name instead and the command would fail.
  // <https://github.com/fluent-ffmpeg/node-fluent-ffmpeg/issues/648#issuecomment-866242144>

  let mut first_volume_event = true;
  for event in iter {
    match event {
      FfmpegEvent::Error(e) | FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, e) => {
        eprintln!("{e}");
      }
      FfmpegEvent::Log(LogLevel::Info, msg) if msg.contains("lavfi.r128.M=") => {
        if let Some(volume) = msg.split("lavfi.r128.M=").last() {
          // Sample log output: [Parsed_ametadata_1 @ 0000024c27effdc0] [info] lavfi.r128.M=-120.691
          // M = "momentary loudness"; a sliding time window of 400ms
          // Volume scale is roughly -70 to 0 LUFS. Anything below -70 is silence.
          // See <https://en.wikipedia.org/wiki/EBU_R_128#Metering>
          let volume_f32 = volume.parse::<f32>().context("Failed to parse volume")?;
          let volume_normalized: usize = max(((volume_f32 / 5.0).round() as i32) + 14, 0) as usize;
          let volume_percent = ((volume_normalized as f32 / 14.0) * 100.0).round();

          // Clear previous line of output
          if !first_volume_event {
            print!("\x1b[1A\x1b[2K");
          } else {
            first_volume_event = false;
          }

          // Blinking red dot to indicate recording
          let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
          let recording_indicator = if time % 2 == 0 { "🔴" } else { "  " };

          println!(
            "{} {} {}%",
            recording_indicator,
            repeat('█').take(volume_normalized).collect::<String>(),
            volume_percent
          );
        }
      }
      _ => {}
    }
  }

  Ok(())
}
