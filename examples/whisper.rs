//! Use OpenAI whisper to transcribe audio from the FFmpeg CLI in realtime.
//! Caution: hacky prototype code

use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
  let _guard = temporarily_use_ffmpeg_from_system_path()?;

  // Download whisper model if it doesn't exist
  download_whisper_model()?;

  println!("Attempting to use Whisper filter...");

  // Run Whisper transcription with generated silence (using system FFmpeg)
  let whisper_filter =
    "whisper=model=./whisper.cpp/models/ggml-base.en.bin:language=en:queue=3:destination=output.srt:format=srt";

  let iter = FfmpegCommand::new()
    .format("lavfi")
    .input("anullsrc=duration=10")
    .arg("-af")
    .arg(&whisper_filter)
    .format("null")
    .output("-")
    .spawn()?
    .iter()?;

  for event in iter {
    match event {
      FfmpegEvent::ParsedConfiguration(config) => {
        println!("Found configuration: {:?}", config);
        if config
          .configuration
          .contains(&"--enable-whisper".to_string())
        {
          println!("Whisper is enabled!");
        } else {
          anyhow::bail!("FFmpeg was not built with Whisper support (--enable-whisper)");
        }
      }
      FfmpegEvent::Log(_, message) => println!("{}", message),
      FfmpegEvent::Progress(progress) => println!("Progress: {}", progress.time),
      FfmpegEvent::Done => {
        println!("Transcription complete! Check output.srt");
        break;
      }
      _ => {}
    }
  }

  Ok(())
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
