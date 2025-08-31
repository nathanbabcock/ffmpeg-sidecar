//! Use OpenAI whisper to transcribe audio from the FFmpeg CLI in realtime.
//! Caution: hacky prototype code

use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
  let _guard = temporarily_use_ffmpeg_from_system_path()?;

  // Download whisper model if it doesn't exist
  download_whisper_model()?;

  // Find default audio input device
  let audio_device = find_default_audio_device()?;
  println!("Listening to audio device: {}", audio_device);
  println!("Starting real-time transcription... (Say 'stop recording' or press Ctrl+C to stop)");

  // Run Whisper transcription with microphone input
  // destination=- uses FFmpeg AVIO syntax to direct output to stdout
  let whisper_filter = "whisper=model=./whisper.cpp/models/ggml-base.en.bin:destination=-";

  let mut command = FfmpegCommand::new();

  // Configure audio input based on platform
  if cfg!(windows) {
    command
      .format("dshow")
      .args("-audio_buffer_size 50".split(' ')) // reduces latency to 50ms
      .input(format!("audio={}", audio_device));
  } else {
    // For Linux/Mac - this is a simplified approach, may need adjustment
    command
      .format("pulse") // or "alsa" on Linux
      .input("default");
  }

  let iter = command
    .arg("-af")
    .arg(&whisper_filter)
    .format("null")
    .output("-")
    .spawn()?
    .iter()?;

  let mut transcription_parts = Vec::new();
  let mut last_transcription_time = Instant::now();
  let pause_threshold = Duration::from_secs(2); // 2 seconds of silence = line break

  for event in iter {
    match event {
      FfmpegEvent::ParsedConfiguration(config) => {
        if !config
          .configuration
          .contains(&"--enable-whisper".to_string())
        {
          anyhow::bail!("FFmpeg was not built with Whisper support (--enable-whisper)");
        }
      }
      FfmpegEvent::OutputChunk(chunk) => {
        // Convert raw bytes to text and collect transcription parts
        if let Ok(text) = String::from_utf8(chunk) {
          let trimmed = text.trim();
          if !trimmed.is_empty() {
            let now = Instant::now();

            // Check if there's been a pause since last transcription
            if now.duration_since(last_transcription_time) > pause_threshold
              && !transcription_parts.is_empty()
            {
              // Start a new line after a pause
              println!("\r{}", transcription_parts.join(" ")); // Finalize previous line
              transcription_parts.clear();
            }

            transcription_parts.push(trimmed.to_string());

            // Check for stop command
            let current_text = transcription_parts.join(" ").to_lowercase();
            if current_text.contains("stop recording") {
              println!("\r{}", transcription_parts.join(" "));
              println!("Stop command detected. Ending transcription session.");
              break;
            }

            // Print current transcription
            print!("\r{}", transcription_parts.join(" "));
            io::stdout().flush().unwrap();

            last_transcription_time = now;
          }
        }
      }
      FfmpegEvent::Done => {
        println!("\nTranscription complete!");
        break;
      }
      _ => {}
    }
  }

  Ok(())
}

fn find_default_audio_device() -> anyhow::Result<String> {
  if cfg!(windows) {
    // Windows: Use dshow to find audio devices
    let audio_device = FfmpegCommand::new()
      .hide_banner()
      .args(["-list_devices", "true"])
      .format("dshow")
      .input("dummy")
      .spawn()?
      .iter()?
      .into_ffmpeg_stderr()
      .find(|line| line.contains("(audio)"))
      .and_then(|line| line.split('\"').nth(1).map(|s| s.to_string()))
      .ok_or_else(|| anyhow::anyhow!("No audio device found on Windows"))?;

    Ok(audio_device)
  } else {
    // Linux/Mac: Use default device (could be improved with proper device detection)
    println!("Note: Using default audio device. On Linux/Mac, you may need to adjust audio format and device.");
    Ok("default".to_string())
  }
}

fn download_whisper_model() -> anyhow::Result<()> {
  let model_path = Path::new("whisper.cpp/models/ggml-base.en.bin");

  // Check if model already exists
  if model_path.exists() {
    println!("Whisper model already exists at {}", model_path.display());
    return Ok(());
  }

  println!("Downloading whisper.cpp and base.en model...");

  // Clone whisper.cpp repository if it doesn't exist
  if !Path::new("whisper.cpp").exists() {
    println!("Cloning whisper.cpp repository...");
    let output = Command::new("git")
      .args(&["clone", "https://github.com/ggml-org/whisper.cpp.git"])
      .output()?;

    if !output.status.success() {
      anyhow::bail!(
        "Failed to clone whisper.cpp: {}",
        String::from_utf8_lossy(&output.stderr)
      );
    }
  }

  // Download the model using the provided script
  println!("Downloading base.en model...");
  let output = Command::new("sh")
    .args(&["./models/download-ggml-model.sh", "base.en"])
    .current_dir("whisper.cpp")
    .output()?;

  if !output.status.success() {
    anyhow::bail!(
      "Failed to download model: {}",
      String::from_utf8_lossy(&output.stderr)
    );
  }

  println!(
    "Successfully downloaded whisper model to {}",
    model_path.display()
  );
  Ok(())
}

/// The `essentials` binary downloaded by the library doesn't have `whisper`
/// Temporarily hide local ffmpeg binaries to force system path usage
/// Requires ffmpeg-8-full to be installed in system PATH
fn temporarily_use_ffmpeg_from_system_path() -> anyhow::Result<RestoreGuard> {
  // Get the directory where the current executable is located
  let exe_dir = env::current_exe()?.parent().unwrap().to_path_buf();

  // Temporarily rename local ffmpeg binaries to force system path usage
  let ffmpeg_names = ["ffmpeg", "ffmpeg.exe"];
  let mut renamed_paths = Vec::new();

  // Rename any local ffmpeg binaries in the executable directory
  for name in &ffmpeg_names {
    let ffmpeg_path = exe_dir.join(name);
    if ffmpeg_path.exists() {
      let backup_path = exe_dir.join(format!("{}.backup", name));
      fs::rename(&ffmpeg_path, &backup_path)?;
      println!(
        "Temporarily renamed {} to {}",
        ffmpeg_path.display(),
        backup_path.display()
      );
      renamed_paths.push((ffmpeg_path, backup_path));
    }
  }

  Ok(RestoreGuard { renamed_paths })
}

struct RestoreGuard {
  renamed_paths: Vec<(std::path::PathBuf, std::path::PathBuf)>,
}

impl Drop for RestoreGuard {
  fn drop(&mut self) {
    for (original, backup) in &self.renamed_paths {
      if let Err(e) = fs::rename(backup, original) {
        eprintln!("Failed to restore {}: {}", original.display(), e);
      } else {
        println!("Restored {}", original.display());
      }
    }
  }
}
