/// One of them main reasons to use named pipes instead of stdout is the ability
/// to support multiple outputs from a single FFmpeg command. The creation and
/// behavior of named pipes is platform-specific, and some of the
/// synchronization logic can be a bit tricky. This example provides a starting
/// point and some cross-platform abstractions over named pipes to make things
/// easier.
///
/// If you need even more granular control over the output streams, you might
/// consider using local TCP sockets instead, which can be more flexible and
/// reliable with the same performance profile, if not better. See `examples/sockets.rs`.
#[cfg(feature = "named_pipes")]
fn main() -> anyhow::Result<()> {
  use anyhow::Result;
  use ffmpeg_sidecar::command::FfmpegCommand;
  use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
  use ffmpeg_sidecar::named_pipes::NamedPipe;
  use ffmpeg_sidecar::pipe_name;
  use std::io::Read;
  use std::sync::mpsc;
  use std::thread;

  const VIDEO_PIPE_NAME: &'static str = pipe_name!("ffmpeg_video");
  const AUDIO_PIPE_NAME: &'static str = pipe_name!("ffmpeg_audio");
  const SUBTITLES_PIPE_NAME: &'static str = pipe_name!("ffmpeg_subtitles");

  // Prepare an FFmpeg command with separate outputs for video, audio, and subtitles.
  let mut command = FfmpegCommand::new();
  command
    // Global flags
    .hide_banner()
    .overwrite() // <- overwrite required on windows
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
    .cloned()
    .map(|pipe_name| {
      // It's important to create the named pipe on the main thread before
      // sending it elsewhere so that any errors are caught at the top level.
      let mut pipe = NamedPipe::new(pipe_name)?;
      println!("[{pipe_name}] pipe created");
      let (ready_sender, ready_receiver) = mpsc::channel::<()>();
      let thread = thread::spawn(move || -> Result<()> {
        // Wait for FFmpeg to start writing
        // Only needed for Windows, since Unix will block until a writer has connected
        println!("[{pipe_name}] waiting for ready signal");
        ready_receiver.recv()?;

        // Read continuously until finished
        // Note that if the stream of output is interrupted or paused,
        // you may need additional logic to keep the read loop alive.
        println!("[{pipe_name}] reading from pipe");
        let mut buf = vec![0; 1920 * 1080 * 3];
        let mut total_bytes_read = 0;

        // In the case of subtitles, we'll decode the string contents directly
        let mut text_content = if pipe_name == SUBTITLES_PIPE_NAME {
          Some("".to_string())
        } else {
          None
        };

        loop {
          match pipe.read(&mut buf) {
            Ok(bytes_read) => {
              total_bytes_read += bytes_read;

              // read bytes into string
              if let Some(cur_str) = &mut text_content {
                let s = std::str::from_utf8(&buf[..bytes_read]).unwrap();
                text_content = Some(format!("{}{}", cur_str, s));
              }

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

        // Log how many bytes were received over this pipe.
        // You can visually compare this to the FFmpeg log output to confirm
        // that all the expected bytes were captured.
        let size_str = if total_bytes_read < 1024 {
          format!("{}B", total_bytes_read)
        } else {
          format!("{}KiB", total_bytes_read / 1024)
        };

        if let Some(text_content) = text_content {
          println!("[{pipe_name}] subtitle text content: ");
          println!("{}", text_content.trim());
        }

        println!("[{pipe_name}] done reading ({size_str} total)");
        Ok(())
      });

      return Ok((thread, ready_sender));
    })
    .collect::<Result<Vec<_>>>()?;

  // Start FFmpeg
  let mut ready_signal_sent = false;
  command
    .print_command()
    .spawn()?
    .iter()?
    .for_each(|event| match event {
      // Signal threads when output is ready
      FfmpegEvent::Progress(_) if !ready_signal_sent => {
        threads.iter().for_each(|(_, sender)| {
          sender.send(()).ok();
        });
        ready_signal_sent = true;
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

#[cfg(not(feature = "named_pipes"))]
fn main() {
  eprintln!(r#"Enable the "named_pipes" feature to run this example."#);
  println!("cargo run --features named_pipes --example named_pipes")
}
