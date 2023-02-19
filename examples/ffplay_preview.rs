use std::{
  io::{Read, Write},
  process::{Command, Stdio},
};

use ffmpeg_sidecar::command::FfmpegCommand;

/// Pipe from ffmpeg to ffplay for debugging purposes
///
/// ```console
/// cargo run --example ffplay_preview
/// ```
fn main() {
  let mut ffmpeg = FfmpegCommand::new()
    .realtime()
    .format("lavfi")
    .input("testsrc=size=1920x1080:rate=60")
    .codec_video("rawvideo")
    .format("avi")
    .output("-")
    .spawn()
    .unwrap();

  let mut ffplay = Command::new("ffplay")
    .args("-i -".split(' '))
    .stdin(Stdio::piped())
    .spawn()
    .unwrap();

  let mut ffmpeg_stdout = ffmpeg.take_stdout().unwrap();
  let mut ffplay_stdin = ffplay.stdin.take().unwrap();

  // pipe from ffmpeg stdout to ffplay stdin
  let buf = &mut [0u8; 4096];
  loop {
    let n = ffmpeg_stdout.read(buf).unwrap();
    if n == 0 {
      break;
    }
    ffplay_stdin.write_all(&buf[..n]).unwrap();
  }
}
