use anyhow::{bail, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::named_pipe::NamedPipe;
use std::io::Read;
use std::thread;

#[cfg(all(windows, feature = "named_pipes"))]
fn main() -> Result<()> {
  use std::{io::Error, ptr::null_mut};
  use winapi::um::namedpipeapi::ConnectNamedPipe;

  const VIDEO_PIPE_NAME: &'static str = r#"\\.\pipe\ffmpeg_video"#;
  const AUDIO_PIPE_NAME: &'static str = r#"\\.\pipe\ffmpeg_audio"#;

  // Thread & named pipe for video
  let video_thread = thread::spawn(move || -> Result<()> {
    // Create
    let mut video_pipe = NamedPipe::new(VIDEO_PIPE_NAME)?;
    let mut video_buf = vec![0; 1920 * 1080 * 3];
    println!("[video] pipe created");

    // Wait
    println!("[video] waiting for connection");
    unsafe {
      let wait_result = ConnectNamedPipe(video_pipe.handle, null_mut());
      if wait_result != 0 {
        eprintln!("Error: {:?}", Error::last_os_error());
        bail!(Error::last_os_error());
      }
    }
    // todo!: this won't work yet
    // need to open pipe with FILE_FLAG_OVERLAPPED,
    // then use OVERLAPPED struct to wait for connection

    // Read
    println!("[video] reading from pipe");
    while let Ok(bytes_read) = video_pipe.read(&mut video_buf) {
      println!("[video] Read {} bytes", bytes_read);
      if bytes_read == 0 {
        break;
      }
    }

    // Exit
    println!("[video] done reading");
    Ok(())
  });

  // Thread & named pipe for audio
  let audio_thread = thread::spawn(move || -> Result<()> {
    // Create
    let mut audio_pipe = NamedPipe::new(AUDIO_PIPE_NAME)?;
    let mut audio_buf = vec![0; 1920 * 1080 * 3];
    println!("[audio] pipe created");

    // Wait
    println!("[audio] waiting for connection");
    unsafe {
      let wait_result = ConnectNamedPipe(audio_pipe.handle, null_mut());
      if wait_result != 0 {
        bail!(Error::last_os_error());
      }
    }

    // Read
    println!("[audio] reading from pipe");
    while let Ok(bytes_read) = audio_pipe.read(&mut audio_buf) {
      println!("[audio] Read {} bytes", bytes_read);
      if bytes_read == 0 {
        break;
      }
    }

    // Exit
    println!("[audio] done reading");
    Ok(())
  });

  // Start the FFmpeg command
  let mut child = FfmpegCommand::new()
    // Global flags:
    .hide_banner()
    .overwrite() // <-- required w/ named pipes on Windows
    // Generate test video:
    .format("lavfi")
    .input(format!("testsrc=size=1920x1080:rate=60:duration=10"))
    // Generate test audio:
    .format("lavfi")
    .input("sine=frequency=1000:duration=10")
    // Split video onto one pipe:
    .map("0:v")
    .format("rawvideo")
    .pix_fmt("rgb24")
    .args(["-flush_packets", "1"])
    .output(VIDEO_PIPE_NAME)
    // Split audio onto the other pipe:
    .map("1:a")
    .format("s16le")
    .args(["-flush_packets", "1"])
    .output(AUDIO_PIPE_NAME)
    .print_command()
    .spawn()?;

  let iter = child.iter()?;
  iter.into_ffmpeg_stderr().for_each(|e| println!("{e}"));
  video_thread.join().unwrap()?;
  audio_thread.join().unwrap()?;

  Ok(())
}
