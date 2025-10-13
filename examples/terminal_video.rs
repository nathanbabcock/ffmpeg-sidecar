use anyhow::Result;
use ffmpeg_sidecar::command::FfmpegCommand;

const OUTPUT_WIDTH: u32 = 80;
const OUTPUT_HEIGHT: u32 = 30;
const OUTPUT_FRAMERATE: u32 = 60;

/// Render video to the terminal
fn main() -> Result<()> {
  let iter = FfmpegCommand::new()
    .format("lavfi")
    .arg("-re") // "realtime"
    .input(format!(
      "testsrc=size={OUTPUT_WIDTH}x{OUTPUT_HEIGHT}:rate={OUTPUT_FRAMERATE}"
    ))
    .rawvideo()
    .spawn()?
    .iter()?
    .filter_frames();

  for frame in iter {
    // clear the previous frame
    if frame.frame_num > 0 {
      for _ in 0..frame.height {
        print!("\x1B[{}A", 1);
      }
    }

    // Print the pixels colored with ANSI codes
    for y in 0..frame.height {
      for x in 0..frame.width {
        let idx = (y * frame.width + x) as usize * 3;
        let r = frame.data[idx] as u32;
        let g = frame.data[idx + 1] as u32;
        let b = frame.data[idx + 2] as u32;
        print!("\x1B[48;2;{r};{g};{b}m ");
      }
      println!("\x1B[0m");
    }
  }

  Ok(())
}
