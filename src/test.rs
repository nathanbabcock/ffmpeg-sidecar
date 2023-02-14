use crate::{command::FfmpegCommand, event::FfmpegEvent};

#[test]
fn test_output() {
  let iter = FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=5:rate=1 -f rawvideo -pix_fmt rgb24".split(' '))
    .pipe_stdout()
    .spawn()
    .unwrap()
    .events_iter()
    .unwrap();

  let frame_count = iter
    .filter(|event| match event {
      FfmpegEvent::OutputFrame(_) => true,
      _ => false,
    })
    .count();

  assert_eq!(frame_count, 5);
}
