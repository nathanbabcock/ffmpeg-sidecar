use std::{sync::mpsc, thread, time::Duration};

use crate::{
  command::{ffmpeg_is_installed, FfmpegCommand},
  event::{FfmpegEvent, LogLevel},
  version::ffmpeg_version,
};

fn approx_eq(a: f32, b: f32, error: f32) -> bool {
  (a - b).abs() < error
}

/// Returns `Err` if the timeout thread finishes before the FFmpeg process
fn spawn_with_timeout(command: &mut FfmpegCommand, timeout: u64) -> anyhow::Result<()> {
  let (sender, receiver) = mpsc::channel();

  // Thread 1: Waits for 1000ms and sends a message
  let timeout_sender = sender.clone();
  thread::spawn(move || {
    thread::sleep(Duration::from_millis(timeout));
    timeout_sender.send("timeout").ok();
  });

  // Thread 2: Consumes the FFmpeg events and sends a message
  let mut ffmpeg_child = command.spawn()?;
  let iter = ffmpeg_child.iter()?;
  thread::spawn(move || {
    iter.for_each(|_| {});
    // Note: `.wait()` would not work here, because it closes `stdin` automatically
    sender.send("ffmpeg").ok();
  });

  // Race the two threads
  let finished_first = receiver.recv()?;
  ffmpeg_child.kill()?;
  match finished_first {
    "timeout" => anyhow::bail!("Timeout thread expired before FFmpeg"),
    "ffmpeg" => Ok(()),
    _ => anyhow::bail!("Unknown message received"),
  }
}

/// Returns `Err` if the timeout thread finishes before the FFmpeg process
/// Note: this variant leaves behind a hung FFmpeg child process + thread until
/// the test suite exits.
fn wait_with_timeout(command: &mut FfmpegCommand, timeout: u64) -> anyhow::Result<()> {
  let (sender, receiver) = mpsc::channel();

  // Thread 1: Waits for 1000ms and sends a message
  let timeout_sender = sender.clone();
  thread::spawn(move || {
    thread::sleep(Duration::from_millis(timeout));
    timeout_sender.send("timeout").ok();
  });

  // Thread 2: Wait for the child to exit in another thread
  let mut ffmpeg_child = command.spawn()?;
  thread::spawn(move || {
    ffmpeg_child.wait().unwrap();
    sender.send("ffmpeg").ok();
  });

  // Race the two threads
  let finished_first = receiver.recv()?;
  match finished_first {
    "timeout" => anyhow::bail!("Timeout thread expired before FFmpeg"),
    "ffmpeg" => Ok(()),
    _ => anyhow::bail!("Unknown message received"),
  }
}

#[test]
fn test_installed() {
  assert!(ffmpeg_is_installed());
}

#[test]
fn test_version() {
  assert!(ffmpeg_version().is_ok());
}

#[test]
fn test_frame_count() {
  let fps = 1;
  let duration = 5;
  let expected_frame_count = fps * duration;
  let arg_string = format!(
    "-f lavfi -i testsrc=duration={duration}:rate={fps} -f rawvideo -pix_fmt rgb24 -"
  );

  let iter = FfmpegCommand::new()
    .args(arg_string.split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap();

  let frame_count = iter
    .filter(|event| matches!(event, FfmpegEvent::OutputFrame(_)))
    .count();

  assert_eq!(frame_count, expected_frame_count);
}

#[test]
fn test_output_format() {
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=1:rate=1 -f rawvideo -pix_fmt rgb24 -".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|event| {
      if let FfmpegEvent::OutputFrame(frame) = event {
        assert_eq!(frame.pix_fmt, "rgb24");
        assert!(frame.data.len() as u32 == frame.width * frame.height * 3);
      }
    });
}

/// Two inputs with the same parameters should produce the same output.
/// This might help catch off-by-one errors where buffers aren't perfectly
/// aligned with output frame boundaries.
#[test]
fn test_deterministic() {
  let arg_str = "-f lavfi -i testsrc=duration=5:rate=1 -f rawvideo -pix_fmt rgb24 -";

  let vec1: Vec<Vec<u8>> = FfmpegCommand::new()
    .args(arg_str.split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_map(|event| match event {
      FfmpegEvent::OutputFrame(frame) => Some(frame.data),
      _ => None,
    })
    .collect();

  let vec2: Vec<Vec<u8>> = FfmpegCommand::new()
    .args(arg_str.split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_map(|event| match event {
      FfmpegEvent::OutputFrame(frame) => Some(frame.data),
      _ => None,
    })
    .collect();

  assert_eq!(vec1, vec2)
}

/// Pass simple raw pixels across stdin and stdout to check that the frame
/// buffers are pixel-perfect across multiple frames.
#[test]
fn test_passthrough() -> anyhow::Result<()> {
  use std::io::Write;

  let mut child = FfmpegCommand::new()
    .args("-f rawvideo -pix_fmt rgb24 -s 2x2 -i - -f rawvideo -pix_fmt rgb24 -".split(' '))
    .spawn()?;

  // Send hardcoded RGB values over stdin as three identical 2x2 frames
  let input_raw_pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
  let mut stdin = child.take_stdin().unwrap();
  stdin.write_all(&input_raw_pixels)?;
  stdin.write_all(&input_raw_pixels)?;
  stdin.write_all(&input_raw_pixels)?;
  stdin.flush()?;
  drop(stdin); // otherwise FFmpeg will hang waiting for more input

  let output_raw_pixels: Vec<Vec<u8>> = child
    .iter()?
    .filter_frames()
    .map(|frame| frame.data)
    .collect();

  assert!(output_raw_pixels.len() == 3);
  assert_eq!(input_raw_pixels, output_raw_pixels[0]);
  assert_eq!(input_raw_pixels, output_raw_pixels[1]);
  assert_eq!(input_raw_pixels, output_raw_pixels[2]);

  Ok(())
}

#[test]
fn test_to_file() {
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=5:rate=1 -y output/test.mp4".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|event| match event {
      FfmpegEvent::ParsedOutput(output) => assert!(!output.is_stdout()),
      FfmpegEvent::OutputFrame(_) => {
        panic!("Should not have received any frames when outputting to file.")
      }
      _ => {}
    });
}

#[test]
fn test_progress() {
  let mut progress_events = 0;
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=5:rate=1 -y output/test.mp4".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_progress()
    .for_each(|_| progress_events += 1);
  assert!(progress_events > 0);
}

#[test]
fn test_error() {
  let errors = FfmpegCommand::new()
    // output format and pix_fmt are deliberately missing, and cannot be inferred
    .args("-f lavfi -i testsrc=duration=1:rate=1 -".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_errors()
    .count();

  assert!(errors > 0);
}

#[test]
fn test_chunks() {
  let mut chunks = 0;
  let mut frames = 0;

  FfmpegCommand::new()
    .testsrc()
    .codec_video("libx264")
    .format("h264")
    .pipe_stdout()
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| match e {
      FfmpegEvent::OutputChunk(_) => chunks += 1,
      FfmpegEvent::OutputFrame(_) => frames += 1,
      _ => {}
    });

  assert!(chunks > 0);
}

#[test]
fn test_chunks_with_video_and_audio() {
  let mut chunks = 0;
  let mut frames = 0;

  FfmpegCommand::new()
    .testsrc()
    .args("-f lavfi -i sine=frequency=1000 -shortest".split(' '))
    .codec_video("libx264")
    .format("mpegts")
    .pipe_stdout()
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| match e {
      FfmpegEvent::OutputChunk(_) => chunks += 1,
      FfmpegEvent::OutputFrame(_) => frames += 1,
      _ => {}
    });

  assert!(chunks > 0);
  assert_eq!(frames, 0);
}

#[test]
fn test_chunks_with_audio_only() -> anyhow::Result<()> {
  let chunks = FfmpegCommand::new()
    .args("-f lavfi -i sine=frequency=1000:duration=10".split(' '))
    .format("s16le")
    .args(["-ac", "1"]) // Mono audio
    .codec_audio("pcm_s16le")
    .args(["-ar", "44100"]) // Sample rate 44.1kHz
    .pipe_stdout()
    .spawn()?
    .iter()?
    .filter(|e| matches!(e, FfmpegEvent::OutputChunk(_)))
    .count();

  assert!(chunks > 0);

  Ok(())
}

#[test]
fn test_duration() {
  // Prepare the input file.
  // TODO construct this in-memory instead of writing to disk.
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=5:rate=1 -y output/test_duration.mp4".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .count();

  let mut duration_received = false;

  FfmpegCommand::new()
    .input("output/test_duration.mp4")
    .format("mpegts")
    .pipe_stdout()
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .for_each(|e| {
      if let FfmpegEvent::ParsedDuration(duration) = e {
        match duration_received {
          false => {
            assert_eq!(duration.duration, 5.0);
            duration_received = true
          }
          true => panic!("Received multiple duration events."),
        }
      }
    });

  assert!(duration_received);
}

#[test]
fn test_metadata_duration() {
  // Prepare the input file.
  // TODO construct this in-memory instead of writing to disk.
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=5:rate=1 -y output/test_metadata_duration.mp4".split(' '))
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .count();

  let mut child = FfmpegCommand::new()
    .input("output/test_metadata_duration.mp4")
    .format("mpegts")
    .pipe_stdout()
    .spawn()
    .unwrap();

  let metadata = child.iter().unwrap().collect_metadata().unwrap();
  child.kill().unwrap();

  assert!(metadata.duration() == Some(5.0));
}

#[ignore = "flaky behavior across different platforms"]
#[test]
fn test_kill_before_iter() {
  let mut child = FfmpegCommand::new().testsrc().rawvideo().spawn().unwrap();
  child.kill().unwrap();
  let vec: Vec<FfmpegEvent> = child.iter().unwrap().collect();
  // On Linux, an error may be thrown before the EOF event is sent.
  assert!(vec.len() <= 1);
  if vec.len() == 1 {
    assert_eq!(vec[0], FfmpegEvent::LogEOF);
  }
}

#[test]
fn test_kill_after_iter() {
  let mut child = FfmpegCommand::new().testsrc().rawvideo().spawn().unwrap();
  let mut iter = child.iter().unwrap();
  assert!(iter.next().is_some());
  child.kill().unwrap();
  child.as_inner_mut().wait().unwrap();
  let count = iter
    .filter(|e| matches!(e, FfmpegEvent::Progress(_)))
    .count();
  assert!(count <= 1);
}

#[test]
fn test_quit() {
  let mut child = FfmpegCommand::new().testsrc().rawvideo().spawn().unwrap();
  child.quit().unwrap();
  let count = child.iter().unwrap().filter_progress().count();
  assert!(count <= 1);
}

#[test]
fn test_frame_timestamp() {
  let mut last_timestamp: Option<f32> = None;
  FfmpegCommand::new()
    .format("lavfi")
    .input("testsrc=duration=1:rate=10")
    .rawvideo()
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_frames()
    .for_each(|frame| {
      match last_timestamp {
        None => assert_eq!(frame.timestamp, 0.0),
        Some(last_timestamp) => assert!(approx_eq(frame.timestamp, last_timestamp + 0.1, 0.001)),
      }
      last_timestamp = Some(frame.timestamp);
    });
  assert!(approx_eq(last_timestamp.unwrap(), 0.9, 0.001));
}

#[test]
fn test_filter_complex() {
  let num_frames = FfmpegCommand::new()
    .format("lavfi")
    .input("testsrc=duration=1:rate=10")
    .rawvideo()
    .filter_complex("fps=5")
    .spawn()
    .unwrap()
    .iter()
    .unwrap()
    .filter_frames()
    .count();
  assert_eq!(num_frames, 5);
}

/// Should not hang prompting for user input on overwrite
/// https://github.com/nathanbabcock/ffmpeg-sidecar/issues/35
#[test]
fn test_overwrite_fallback() -> anyhow::Result<()> {
  let output_path = "output/test_overwrite_fallback.jpg";
  let timeout_ms = 1000;

  let write_file_with_timeout = || {
    let mut command = FfmpegCommand::new();
    command.testsrc().frames(1).output(output_path);
    spawn_with_timeout(&mut command, timeout_ms)
  };

  write_file_with_timeout()?;
  let time1 = std::fs::metadata(output_path)?.modified()?;

  write_file_with_timeout()?;
  let time2 = std::fs::metadata(output_path)?.modified()?;

  assert_eq!(time1, time2);

  Ok(())
}

#[test]
fn test_overwrite_nostdin() -> anyhow::Result<()> {
  let output_path = "output/test_overwrite_nostdin.jpg";

  let write_file = || -> anyhow::Result<_> {
    FfmpegCommand::new()
      .arg("-nostdin")
      .testsrc()
      .frames(1)
      .output(output_path)
      .spawn()?
      .wait()
      .map_err(Into::into)
  };

  write_file()?;
  let time1 = std::fs::metadata(output_path)?.modified()?;

  write_file()?;
  let time2 = std::fs::metadata(output_path)?.modified()?;

  assert_eq!(time1, time2);

  Ok(())
}

#[test]
fn test_overwrite() -> anyhow::Result<()> {
  let output_path = "output/test_overwrite.jpg";

  let write_file = || -> anyhow::Result<_> {
    FfmpegCommand::new()
      .overwrite()
      .testsrc()
      .frames(1)
      .output(output_path)
      .spawn()?
      .wait()
      .map_err(Into::into)
  };

  write_file()?;
  let time1 = std::fs::metadata(output_path)?.modified()?;

  write_file()?;
  let time2 = std::fs::metadata(output_path)?.modified()?;

  assert_ne!(time1, time2);

  Ok(())
}

#[test]
fn test_no_overwrite() -> anyhow::Result<()> {
  let output_path = "output/test_no_overwrite.jpg"; // same file, ok if it exists

  let write_file = || -> anyhow::Result<_> {
    FfmpegCommand::new()
      .no_overwrite()
      .testsrc()
      .frames(1)
      .output(output_path)
      .spawn()?
      .wait()
      .map_err(Into::into)
  };

  write_file()?;
  let time1 = std::fs::metadata(output_path)?.modified()?;

  write_file()?;
  let time2 = std::fs::metadata(output_path)?.modified()?;

  assert_eq!(time1, time2);

  Ok(())
}

#[test]
#[cfg(feature = "named_pipes")]
fn test_named_pipe() -> anyhow::Result<()> {
  use crate::{event::LogLevel, named_pipes::NamedPipe, pipe_name};
  use std::{io::Read, thread::JoinHandle};

  let pipe_name = pipe_name!("test_pipe");

  // Create FFmpeg command
  let mut command = FfmpegCommand::new();
  command
    .overwrite()
    .format("lavfi")
    .input("testsrc=size=320x240:rate=1:duration=1")
    .frames(1)
    .format("rawvideo")
    .pix_fmt("rgb24")
    .output(pipe_name);

  // Open the named pipe
  let (sender, receiver) = mpsc::channel::<bool>();
  let thread: JoinHandle<Result<(), anyhow::Error>> = thread::spawn(move || {
    let mut named_pipe = NamedPipe::new(pipe_name)?;
    let mut buffer = [0u8; 65536];
    receiver.recv()?;

    let mut total_bytes_read = 0;
    loop {
      match named_pipe.read(&mut buffer) {
        Ok(bytes_read) => {
          total_bytes_read += bytes_read;
          if bytes_read == 0 {
            break;
          }
        }
        Err(err) => anyhow::bail!(err),
      }
    }
    assert_eq!(total_bytes_read, 320 * 240 * 3);
    Ok(())
  });

  // Start the source process
  let mut ready_signal_sent = false;
  command.spawn()?.iter()?.for_each(|event| match event {
    FfmpegEvent::Progress(e) if !ready_signal_sent => {
      println!("Progress: {:?}", e);
      sender.send(true).ok();
      ready_signal_sent = true;
    }
    FfmpegEvent::Log(LogLevel::Warning | LogLevel::Error | LogLevel::Fatal, msg) => {
      eprintln!("{msg}");
    }
    _ => {}
  });

  thread.join().unwrap()?;

  Ok(())
}

/// Ensure non-byte-aligned pixel formats are still processed correctly.
/// YUV420 has 12 bits per pixel, but the whole frame buffer will still be
/// enforced to be byte-aligned.
/// See <https://github.com/nathanbabcock/ffmpeg-sidecar/pull/61>
#[test]
fn test_yuv420() -> anyhow::Result<()> {
  let iter = FfmpegCommand::new()
    .hide_banner()
    .testsrc()
    .format("rawvideo")
    .pix_fmt("yuv420p")
    .pipe_stdout()
    .spawn()?
    .iter()?;

  let mut frames_received = 0;

  for event in iter {
    match event {
      FfmpegEvent::OutputFrame(frame) => {
        frames_received += 1;
        assert_eq!(frame.pix_fmt, "yuv420p");
        // Expect 12 bits per pixel, but with an assumption that valid sizes
        // will still result in a byte-aligned frame buffer (divisible by 8).
        assert!(frame.data.len() as u32 == frame.width * frame.height * 12 / 8);
      }
      FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, _) => {
        panic!("Error or fatal log message received");
      }
      _ => {}
    }
  }

  assert_eq!(frames_received, 10 * 25); // 10 seconds at 25 fps

  Ok(())
}

/// Make sure that the iterator doesn't hang forever if there's an invalid
/// framebuffer size; instead, it should fall back to chunked mode.
/// See <https://github.com/nathanbabcock/ffmpeg-sidecar/pull/61>
#[test]
fn test_yuv420_invalid_size() -> anyhow::Result<()> {
  let iter = FfmpegCommand::new()
    .hide_banner()
    .format("lavfi")
    .input("testsrc=duration=10:size=321x241")
    .format("rawvideo")
    .pix_fmt("yuv420p")
    .pipe_stdout()
    .spawn()?
    .iter()?;

  let mut chunks_received = 0;

  for event in iter {
    match event {
      FfmpegEvent::OutputFrame(_) => {
        panic!("Should not use OutputFrame for non-byte-aligned sizes");
      }
      FfmpegEvent::OutputChunk(_) => {
        chunks_received += 1;
      }
      FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, _) => {
        panic!("Error or fatal log message received");
      }
      _ => {}
    }
  }

  assert!(chunks_received > 0);

  Ok(())
}

/// Multiple `rawvideo` outputs can be interleaved on stdout.
#[test]
fn test_stdout_interleaved_frames() -> anyhow::Result<()> {
  let iter = FfmpegCommand::new()
    .testsrc()
    .rawvideo()
    .testsrc()
    .rawvideo()
    .spawn()?
    .iter()?
    .filter_frames();

  let mut output_1_frames = 0;
  let mut output_2_frames = 0;

  for frame in iter {
    match frame.output_index {
      0 => output_1_frames += 1,
      1 => output_2_frames += 1,
      _ => panic!("Unexpected stream index"),
    }
  }

  assert_eq!(output_1_frames, 10 * 25); // 10 sec at 25fps
  assert_eq!(output_2_frames, 10 * 25); // 10 sec at 25fps

  Ok(())
}

/// Multiple interleaved outputs can't be supported with non-uniform framerate.
#[test]
fn test_stdout_interleaved_frames_fallback() -> anyhow::Result<()> {
  let iter = FfmpegCommand::new()
    .testsrc()
    .rate(25.0)
    .rawvideo()
    .testsrc()
    .rate(30.0)
    .rawvideo()
    .spawn()?
    .iter()?;

  let mut output_chunks = 0;
  for event in iter {
    match event {
      FfmpegEvent::OutputFrame(_) => {
        panic!("Should not use OutputFrame for interleaved streams");
      }
      FfmpegEvent::OutputChunk(_) => {
        output_chunks += 1;
      }
      _ => {}
    }
  }
  assert!(output_chunks > 0);

  Ok(())
}

/// Make sure consecutive new lines in logs don't result in empty events.
#[test]
fn test_no_empty_events() -> anyhow::Result<()> {
  let empty_events = FfmpegCommand::new()
    .testsrc()
    .rawvideo()
    .spawn()?
    .iter()?
    .filter(|event| match event {
      FfmpegEvent::Log(_, msg) if msg.is_empty() => true,
      _ => false,
    })
    .count();

  assert_eq!(empty_events, 0);

  Ok(())
}

/// This command generates an warning on every frame, e.g.:
///
/// ```txt
/// [Parsed_palettegen_4 @ 0x600001574bb0] [warning] The input frame is not in sRGB, colors may be off
///```
///
/// When used in combination with `.wait()`, these error messages can completely
/// fill the stderr buffer and cause a deadlock. The solution is to
/// automatically drop the stderr channel when `.wait()` is called.
///
/// <https://github.com/nathanbabcock/ffmpeg-sidecar/issues/70>
#[test]
fn test_wait() -> anyhow::Result<()> {
  let mut command = FfmpegCommand::new();
  command
    .args("-color_primaries 1".split(' '))
    .args("-color_trc 1".split(' '))
    .args("-colorspace 1".split(' '))
    .format("lavfi")
    .input("yuvtestsrc=size=64x64:rate=60:duration=60")
    .args("-vf palettegen=max_colors=164".split(' '))
    .codec_video("gif")
    .format("null")
    .output("-");
  wait_with_timeout(&mut command, 5000)
}
