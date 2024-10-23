use std::{sync::mpsc, thread, time::Duration};

use crate::{
  command::{ffmpeg_is_installed, FfmpegCommand},
  event::FfmpegEvent,
  version::ffmpeg_version,
};

fn approx_eq(a: f32, b: f32, error: f32) -> bool {
  (a - b).abs() < error
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
    "-f lavfi -i testsrc=duration={}:rate={} -f rawvideo -pix_fmt rgb24 -",
    duration, fps
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
        assert!(frame.pix_fmt == "rgb24");
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

  assert!(vec1 == vec2)
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
fn test_chunks_with_audio() {
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
            assert!(duration.duration == 5.0);
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

#[test]
fn test_kill_before_iter() {
  let mut child = FfmpegCommand::new().testsrc().rawvideo().spawn().unwrap();
  child.kill().unwrap();
  let vec: Vec<FfmpegEvent> = child.iter().unwrap().collect();
  assert!(vec.len() == 1);
  assert!(vec[0] == FfmpegEvent::LogEOF);
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
        None => assert!(frame.timestamp == 0.0),
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
  assert!(num_frames == 5);
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
  // Prepare an FFmpeg command and create a named pipe
  let mut binding = FfmpegCommand::new();
  let from_command = binding
    .overwrite()
    .testsrc()
    .frames(1)
    .format("rawvideo")
    .named_pipe("\\\\.\\pipe\\test_pipe")?;

  // In a seperate thread, read from the named pipe as input
  thread::spawn(|| {
    let to_command = FfmpegCommand::new()
      .overwrite()
      .format("rawvideo")
      .input("\\\\.\\pipe\\test_pipe")
      .output("output/test_named_pipe.jpg")
      .spawn()
      .unwrap()
      .iter()
      .unwrap()
      .for_each(|e| println!("[to] {:?}", e));
  });

  // Start the source process
  from_command
    .spawn()?
    .iter()?
    .for_each(|e| println!("[from] {:?}", e));

  Ok(())
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
