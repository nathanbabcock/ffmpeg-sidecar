use crate::{command::FfmpegCommand, event::FfmpegEvent};

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
    .filter(|event| match event {
      FfmpegEvent::OutputFrame(_) => true,
      _ => false,
    })
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
    .for_each(|event| match event {
      FfmpegEvent::OutputFrame(frame) => {
        assert!(frame.pix_fmt == "rgb24");
        assert!(frame.data.len() as u32 == frame.width * frame.height * 3);
      }
      _ => {}
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

// #[test]
// fn test_kill_before_iter() {
//   let mut child = FfmpegCommand::new()
//     .args("-f lavfi -i testsrc=duration=1:rate=1 -f rawvideo -pix_fmt rgb24 -".split(' '))
//     .spawn()
//     .unwrap();
//   child.kill().unwrap();
//   let iter = child.events_iter();
//   assert!(iter.is_err());
// }

// #[test]
// fn test_kill_after_iter() {
//   let mut child = FfmpegCommand::new()
//     .args("-f lavfi -i testsrc=duration=1:rate=1 -f rawvideo -pix_fmt rgb24 -".split(' '))
//     .spawn()
//     .unwrap();
//   let mut iter = child.events_iter().unwrap();
//   // println!("{:?}", iter.next());
//   assert!(iter.next().is_some());
//   child.kill().unwrap();
//   child.as_inner_mut().wait().unwrap();
//   assert!(iter.next().is_none());
// }
