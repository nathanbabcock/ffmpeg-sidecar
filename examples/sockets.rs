use anyhow::{bail, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
  // Set up a TCP listener
  const TCP_PORT: u32 = 3000;
  let (exit_sender, exit_receiver) = channel::<()>();
  let listener_thread = thread::spawn(|| listen_for_connections(TCP_PORT, exit_receiver));

  // Wait for the listener to start
  thread::sleep(Duration::from_millis(1000));

  // Prepare an FFmpeg command with separate outputs for video, audio, and subtitles.
  FfmpegCommand::new()
    // Global flags
    .hide_banner()
    .overwrite() // <- overwrite required on windows
    // Generate test video
    .format("lavfi")
    .input("testsrc=size=1920x1080:rate=60:duration=10")
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
    .output(format!("tcp://127.0.0.1:{TCP_PORT}"))
    // Audio output
    .map("1:a")
    .format("s16le")
    .output(format!("tcp://127.0.0.1:{TCP_PORT}"))
    // Subtitles output
    .map("2:s")
    .format("srt")
    .output(format!("tcp://127.0.0.1:{TCP_PORT}"))
    .print_command()
    .spawn()?
    .iter()?
    .for_each(|event| match event {
      // Verify output size from FFmpeg logs (video/audio KiB)
      FfmpegEvent::Log(LogLevel::Info, msg) if msg.starts_with("[out#") => {
        println!("{msg}");
      }

      // Log any unexpected errors
      FfmpegEvent::Log(LogLevel::Warning | LogLevel::Error | LogLevel::Fatal, msg) => {
        eprintln!("{msg}");
      }

      // _ => {}
      e => {
        println!("{e:?}");
      }
    });
  exit_sender.send(())?;
  listener_thread.join().unwrap()?;
  Ok(())
}

fn listen_for_connections(tcp_port: u32, exit_receiver: Receiver<()>) -> Result<()> {
  let listener = TcpListener::bind(format!("127.0.0.1:{tcp_port}"))?;
  listener.set_nonblocking(true)?;
  println!("Server listening on port {tcp_port}");

  let mut handler_threads = Vec::new();
  loop {
    if exit_receiver.try_recv().is_ok() {
      break;
    }
    match listener.accept() {
      Ok((stream, _)) => {
        handler_threads.push(thread::spawn(move || handle_connection(stream)));
      }
      Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        thread::sleep(Duration::from_millis(10));
      }
      Err(e) => {
        bail!(e);
      }
    }
  }

  for handler in handler_threads {
    handler.join().unwrap()?;
  }

  println!("Listener thread exiting");
  Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
  let mut buffer = [0; 1024];
  let mut total_bytes_read = 0;
  loop {
    match stream.read(&mut buffer) {
      Ok(bytes_read) if bytes_read > 0 => {
        total_bytes_read += bytes_read;
      }
      Ok(0) => {
        break;
      }
      Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        thread::sleep(Duration::from_millis(10));
      }
      Err(e) => {
        bail!(e);
      }
      _ => {}
    }
  }
  let bytes_str = if total_bytes_read < 1024 {
    format!("{total_bytes_read}B")
  } else {
    format!("{}KiB", total_bytes_read / 1024)
  };
  println!("Read {bytes_str} from client");
  Ok(())
}
