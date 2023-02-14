use crate::{command::FfmpegCommand, event::FfmpegEvent};
use std::io;

// #[test]
// fn test_output() {
//   println!("test_output");

//   let iter = FfmpegCommand::new()
//     .args("-f lavfi -i testsrc=duration=5 -r 1".split(' '))
//     .pipe_stdout()
//     .spawn()
//     .unwrap()
//     .events_iter();

//   let frame_count = iter
//     .filter(|event| {
//       println!("{:?}", event);
//       match event {
//         FfmpegEvent::OutputFrame(_) => true,
//         _ => false,
//       }
//     })
//     .count();

//   assert_eq!(frame_count, 5);
// }
