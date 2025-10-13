use anyhow::Result;
use std::process::Command;

/// Conway's Game of Life in FFmpeg
/// <https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life>
/// <https://ffmpeg.org/ffmpeg-filters.html#life>
pub fn main() -> Result<()> {
  Command::new("ffplay")
    .arg("-hide_banner")
    .arg("-f").arg("lavfi")
    .arg("-i").arg("life=s=300x200:mold=10:r=60:ratio=0.08:death_color=#C83232:life_color=#00ff00,scale=1200:800:flags=16")
    .spawn()?
    .wait()?;
  Ok(())
}
