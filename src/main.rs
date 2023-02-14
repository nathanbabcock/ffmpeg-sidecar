use ffmpeg_sidecar::command::FfmpegCommand;

fn main() {
  FfmpegCommand::new()
    .args("-f lavfi -i testsrc=duration=1:rate=1 -f rawvideo -pix_fmt rgb24".split(' '))
    .print_command()
    .spawn()
    .unwrap();
}
