#[cfg(all(windows, feature = "named_pipes"))]
fn main() -> anyhow::Result<()> {
  use anyhow::Result;
  use ffmpeg_sidecar::command::FfmpegCommand;
  use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
  use ffmpeg_sidecar::named_pipe::NamedPipe;
  use std::io::Read;
  use std::sync::mpsc;
  use std::thread;

  const VIDEO_PIPE_NAME: &'static str = r#"\\.\pipe\ffmpeg_video"#;
  const AUDIO_PIPE_NAME: &'static str = r#"\\.\pipe\ffmpeg_audio"#;
  const SUBTITLES_PIPE_NAME: &'static str = r#"\\.\pipe\ffmpeg_subtitles"#;

  // Prepare an FFmpeg command with separate outputs for video, audio, and subtitles
  let mut command = FfmpegCommand::new();
  command
    // Global flags
    .hide_banner()
    .overwrite() // <- overwrite reqired on windows
    // Generate test video
    .format("lavfi")
    .input(format!("testsrc=size=1920x1080:rate=60:duration=10"))
    // Generate test audio
    .format("lavfi")
    .input("sine=frequency=1000:duration=10")
    // Generate test subtitles
    .format("srt")
    .input(
      "data:text/plain;base64,MQ0KMDA6MDA6MDAsMDAwIC0tPiAwMDowMDoxMCw1MDANCkhlbGxvIFdvcmxkIQ==",
    )
    // Video output
    .map("0:v")
    .format("rawvideo")
    .pix_fmt("rgb24")
    .output(VIDEO_PIPE_NAME)
    // Audio output
    .map("1:a")
    .format("s16le")
    .output(AUDIO_PIPE_NAME)
    // Subtitles output
    .map("2:s")
    .format("srt")
    .output(SUBTITLES_PIPE_NAME);

  // Create a separate thread for each output pipe
  let threads = [VIDEO_PIPE_NAME, AUDIO_PIPE_NAME, SUBTITLES_PIPE_NAME]
    .iter()
    .map(|pipe_name| {
      let (ready_sender, ready_receiver) = mpsc::channel::<()>();
      let thread = thread::spawn(move || -> Result<()> {
        let mut pipe = NamedPipe::new(pipe_name)?;
        println!("[{pipe_name}] pipe created");

        // Wait for FFmpeg to start writing
        println!("[{pipe_name}] waiting for ready signal");
        ready_receiver.recv()?;

        // Read continuously until finished
        println!("[{pipe_name}] reading from pipe");
        let mut buf = vec![0; 1920 * 1080 * 3];
        let mut total_bytes_read = 0;
        loop {
          match pipe.read(&mut buf) {
            Ok(bytes_read) => {
              total_bytes_read += bytes_read;
              if bytes_read == 0 {
                break;
              }
            }
            Err(err) => {
              if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(err.into());
              } else {
                break;
              }
            }
          }
        }

        // Exit
        let size_str = if total_bytes_read < 1024 {
          format!("{}B", total_bytes_read)
        } else {
          format!("{}KiB", total_bytes_read / 1024)
        };
        println!("[{pipe_name}] done reading ({size_str} total)");
        Ok(())
      });

      return (thread, ready_sender);
    })
    .collect::<Vec<_>>();

  // Start FFmpeg
  let mut ready_signal_sent = false;
  command
    .print_command()
    .spawn()?
    .iter()?
    .for_each(|event| match event {
      // Sigbnal threads when output is ready
      FfmpegEvent::Progress(_) => {
        if !ready_signal_sent {
          threads.iter().for_each(|(_, sender)| {
            sender.send(()).ok();
          });
          ready_signal_sent = true;
        }
      }

      // Verify output size from FFmpeg logs (video/audio KiB)
      FfmpegEvent::Log(LogLevel::Info, msg) if msg.starts_with("[out#") => {
        println!("{msg}");
      }

      // Log any unexpected errors
      FfmpegEvent::Log(LogLevel::Warning | LogLevel::Error | LogLevel::Fatal, msg) => {
        eprintln!("{msg}");
      }

      _ => {}
    });

  for (thread, _) in threads {
    thread.join().unwrap()?;
  }

  Ok(())
}

#[cfg(not(all(windows, feature = "named_pipes")))]
fn main() {}
