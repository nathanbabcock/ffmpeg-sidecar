pub mod command;

use ffmpeg_sidecar::stderr_parser::StderrParser;
use std::process::{Command, Stdio};

pub fn main() {
  let cmd = Command::new("ffmpeg")
    .arg("-version")
    .stdout(Stdio::piped())
    // ⚠ notice that ffmpeg emits on stdout when `-version` or `-help` is passed!
    .spawn()
    .unwrap();

  let stdout = cmd.stdout.unwrap();
  let mut parser = StderrParser::new(stdout);
  let configuration = parser.ffmpeg_configuration();
  configuration
    .unwrap()
    .iter()
    .for_each(|s| println!("{}", s));
}
