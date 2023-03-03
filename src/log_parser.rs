use std::{
  io::{BufReader, Read},
  str::from_utf8,
};

use crate::{
  comma_iter::CommaIter,
  error::{Error, Result},
  event::{
    AVStream, FfmpegConfiguration, FfmpegEvent, FfmpegOutput, FfmpegProgress, FfmpegVersion,
  },
  read_until_any::read_until_any,
};

#[derive(Debug, Clone, PartialEq)]
enum LogSection {
  Input(u32),
  Output(u32),
  StreamMapping,
  Other,
}

pub struct FfmpegLogParser<R: Read> {
  reader: BufReader<R>,
  cur_section: LogSection,
}

impl<R: Read> FfmpegLogParser<R> {
  /// Consume lines from the inner reader until obtaining a completed
  /// `FfmpegEvent`, returning it.
  ///
  /// Typically this consumes a single line, but in the case of multi-line
  /// input/output stream specifications, nested method calls will consume
  /// additional lines until the entire vector of Inputs/Outputs is parsed.
  ///
  /// Line endings can be marked by three possible delimiters:
  /// - `\n` (MacOS),
  /// - `\r\n` (Windows)
  /// - `\r` (Windows, progress updates which overwrite the previous line)
  pub fn parse_next_event(&mut self) -> Result<FfmpegEvent> {
    let mut buf = Vec::<u8>::new();
    let bytes_read = read_until_any(&mut self.reader, &[b'\r', b'\n'], &mut buf);
    let line = from_utf8(buf.as_slice())?.trim();
    match bytes_read {
      Ok(0) => Ok(FfmpegEvent::LogEOF),
      Ok(_) => {
        // Track log section
        if let Some(input_number) = try_parse_input(line) {
          self.cur_section = LogSection::Input(input_number);
        } else if let Some(output) = try_parse_output(line) {
          self.cur_section = LogSection::Output(output.index);
          return Ok(FfmpegEvent::ParsedOutput(output));
        } else if line.contains("Stream mapping:") {
          self.cur_section = LogSection::StreamMapping;
        }

        // Parse
        if let Some(version) = try_parse_version(line) {
          Ok(FfmpegEvent::ParsedVersion(FfmpegVersion {
            version,
            raw_log_message: line.to_string(),
          }))
        } else if let Some(configuration) = try_parse_configuration(line) {
          Ok(FfmpegEvent::ParsedConfiguration(FfmpegConfiguration {
            configuration,
            raw_log_message: line.to_string(),
          }))
        } else if let Some(stream) = try_parse_stream(line) {
          match self.cur_section {
            LogSection::Input(_) => Ok(FfmpegEvent::ParsedInputStream(stream)),
            LogSection::Output(_) => Ok(FfmpegEvent::ParsedOutputStream(stream)),
            LogSection::Other | LogSection::StreamMapping => Err(Error::msg(format!(
              "Unexpected stream specification: {}",
              line
            ))),
          }
        } else if self.cur_section == LogSection::StreamMapping && line.contains("  Stream #") {
          Ok(FfmpegEvent::ParsedStreamMapping(line.to_string()))
        } else if let Some(progress) = try_parse_progress(line) {
          self.cur_section = LogSection::Other;
          Ok(FfmpegEvent::Progress(progress))
        } else if line.starts_with("[info]") {
          Ok(FfmpegEvent::LogInfo(line.to_string()))
        } else if line.starts_with("[warning]") {
          Ok(FfmpegEvent::LogWarning(line.to_string()))
        } else if line.starts_with("[error]") || line.starts_with("[fatal]") {
          Ok(FfmpegEvent::LogError(line.to_string()))
        } else {
          Ok(FfmpegEvent::LogUnknown(line.to_string()))
        }
      }
      Err(e) => Err(Error::from_std(e)),
    }
  }

  pub fn new(inner: R) -> Self {
    Self {
      reader: BufReader::new(inner),
      cur_section: LogSection::Other,
    }
  }
}

/// Parses the ffmpeg version string from the stderr stream,
/// typically the very first line of output:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_version;
///
/// let line = "[info] ffmpeg version 2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev Copyright (c) 2000-2023 the FFmpeg developers\n";
///
/// let version = try_parse_version(line).unwrap();
///
/// assert!(version == "2023-01-18-git-ba36e6ed52-full_build-www.gyan.dev");
/// ```
pub fn try_parse_version(string: &str) -> Option<String> {
  string
    .strip_prefix("[info]")
    .unwrap_or(string)
    .trim()
    .strip_prefix("ffmpeg version ")?
    .split_whitespace()
    .next()
    .map(|s| s.to_string())
}

/// Parses the list of configuration flags ffmpeg was built with.
/// Typically the second line of log output.
///
/// ## Example:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_configuration;
///
/// let line = "[info]   configuration: --enable-gpl --enable-version3 --enable-static\n";
/// // Typically much longer, 20-30+ flags
///
/// let version = try_parse_configuration(line).unwrap();
///
/// assert!(version.len() == 3);
/// assert!(version[0] == "--enable-gpl");
/// assert!(version[1] == "--enable-version3");
/// assert!(version[2] == "--enable-static");
/// ```
///
pub fn try_parse_configuration(string: &str) -> Option<Vec<String>> {
  string
    .strip_prefix("[info]")
    .unwrap_or(string)
    .trim()
    .strip_prefix("configuration: ")
    .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
}

/// Parse an input section like the following, extracting the index of the input:
///
/// ## Example:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_input;
/// let line = "[info] Input #0, lavfi, from 'testsrc=duration=5':\n";
/// let input = try_parse_input(line);
/// assert!(input == Some(0));
/// ```
///
pub fn try_parse_input(string: &str) -> Option<u32> {
  string
    .strip_prefix("[info]")
    .unwrap_or(string)
    .trim()
    .strip_prefix("Input #")?
    .split_whitespace()
    .next()
    .and_then(|s| s.split(',').next())
    .and_then(|s| s.parse::<u32>().ok())
}

/// Parse an output section like the following, extracting the index of the input:
///
/// ## Example:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_output;
/// use ffmpeg_sidecar::event::FfmpegOutput;
/// let line = "[info] Output #0, mp4, to 'test.mp4':\n";
/// let output = try_parse_output(line);
/// assert!(output == Some(FfmpegOutput {
///   index: 0,
///   to: "test.mp4".to_string(),
///   raw_log_message: line.to_string(),
/// }));
/// ```
///
pub fn try_parse_output(mut string: &str) -> Option<FfmpegOutput> {
  let raw_log_message = string.to_string();

  string = string
    .strip_prefix("[info]")
    .unwrap_or(string)
    .trim()
    .strip_prefix("Output #")?;

  let index = string
    .split_whitespace()
    .next()
    .and_then(|s| s.split(',').next())
    .and_then(|s| s.parse::<u32>().ok())?;

  let to = string
    .split(" to '")
    .nth(1)?
    .split('\'')
    .next()?
    .to_string();

  Some(FfmpegOutput {
    index,
    to,
    raw_log_message,
  })
}

/// ## Example
///
/// Input stream:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_stream;
/// let line = "[info]   Stream #0:0: Video: wrapped_avframe, rgb24, 320x240 [SAR 1:1 DAR 4:3], 25 fps, 25 tbr, 25 tbn\n";
/// let stream = try_parse_stream(line).unwrap();
/// assert!(stream.format == "wrapped_avframe");
/// assert!(stream.pix_fmt == "rgb24");
/// assert!(stream.width == 320);
/// assert!(stream.height == 240);
/// assert!(stream.parent_index == 0);
/// ```
///
/// Output stream:
///
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_stream;
/// let line = "[info]   Stream #1:0: Video: h264 (avc1 / 0x31637661), yuv444p(tv, progressive), 320x240 [SAR 1:1 DAR 4:3], q=2-31, 25 fps, 12800 tbn\n";
/// let stream = try_parse_stream(line).unwrap();
/// assert!(stream.format == "h264");
/// assert!(stream.pix_fmt == "yuv444p");
/// assert!(stream.width == 320);
/// assert!(stream.height == 240);
/// assert!(stream.parent_index == 1);
/// ```
pub fn try_parse_stream(mut string: &str) -> Option<AVStream> {
  let raw_log_message = string.to_string();

  string = string
    .strip_prefix("[info]")
    .unwrap_or(string)
    .trim()
    .strip_prefix("Stream #")?;

  let mut colon_parts = string.split(':');
  let parent_index = colon_parts.next()?.parse::<usize>().ok()?;
  let stream_type = colon_parts.nth(1)?.trim();
  if stream_type != "Video" {
    return None; // Audio not supported yet (PRs welcome)
  }
  let comma_string = colon_parts.next()?.trim();
  let mut comma_iter = CommaIter::new(comma_string);

  let format = comma_iter
    .next()?
    .trim()
    .split(&[' ', '(']) // trim trailing junk like " (avc1 / 0x31637661)"
    .next()?
    .to_string();

  let pix_fmt = comma_iter
    .next()?
    .trim()
    .split(&[' ', '(']) // trim trailing junk like "(tv, progressive)"
    .next()?
    .to_string();
  let dims = comma_iter.next()?.split_whitespace().next()?;
  let mut dims_iter = dims.split('x');
  let width = dims_iter.next()?.parse::<u32>().ok()?;
  let height = dims_iter.next()?.parse::<u32>().ok()?;

  Some(AVStream {
    parent_index,
    format,
    pix_fmt,
    width,
    height,
    raw_log_message,
  })
}

/// Parse a progress update line from ffmpeg.
///
/// ## Example
/// ```rust
/// use ffmpeg_sidecar::log_parser::try_parse_progress;
/// let line = "[info] frame= 1996 fps=1984 q=-1.0 Lsize=     372kB time=00:01:19.72 bitrate=  38.2kbits/s speed=79.2x\n";
/// let progress = try_parse_progress(line).unwrap();
/// assert!(progress.frame == 1996);
/// assert!(progress.fps == 1984.0);
/// assert!(progress.q == -1.0);
/// assert!(progress.size_kb == 372);
/// assert!(progress.time == "00:01:19.72");
/// assert!(progress.bitrate_kbps == 38.2);
/// assert!(progress.speed == 79.2);
/// ```
pub fn try_parse_progress(mut string: &str) -> Option<FfmpegProgress> {
  let raw_log_message = string.to_string();

  string = string.strip_prefix("[info]").unwrap_or(string).trim();

  let frame = string
    .split("frame=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .parse::<u32>()
    .ok()?;
  let fps = string
    .split("fps=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .parse::<f32>()
    .ok()?;
  let q = string
    .split("q=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .parse::<f32>()
    .ok()?;
  let size_kb = string
    .split("size=") // captures "Lsize=" AND "size="
    .nth(1)?
    .split_whitespace()
    .next()?
    .trim()
    .strip_suffix("kB")?
    .parse::<u32>()
    .ok()?;
  let time = string
    .split("time=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .to_string();
  let bitrate_kbps = string
    .split("bitrate=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .trim()
    .strip_suffix("kbits/s")?
    .parse::<f32>()
    .ok()?;
  let speed = string
    .split("speed=")
    .nth(1)?
    .split_whitespace()
    .next()?
    .strip_suffix('x')
    .map(|s| s.parse::<f32>().unwrap_or(0.0))
    .unwrap_or(0.0);

  Some(FfmpegProgress {
    frame,
    fps,
    q,
    size_kb,
    time,
    bitrate_kbps,
    speed,
    raw_log_message,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    io::{Cursor, Seek, SeekFrom, Write},
    process::{Command, Stdio},
  };

  #[test]
  fn test_version() {
    let cmd = Command::new("ffmpeg")
      .arg("-version")
      .stdout(Stdio::piped())
      // ⚠ notice that ffmpeg emits on stdout when `-version` or `-help` is passed!
      .spawn()
      .unwrap();

    let stdout = cmd.stdout.unwrap();
    let mut parser = FfmpegLogParser::new(stdout);
    while let Ok(event) = parser.parse_next_event() {
      match event {
        FfmpegEvent::ParsedVersion(_) => return,
        _ => {}
      }
    }
    panic!() // should have found a version
  }

  #[test]
  fn test_configuration() {
    let cmd = Command::new("ffmpeg")
      .arg("-version")
      .stdout(Stdio::piped())
      // ⚠ notice that ffmpeg emits on stdout when `-version` or `-help` is passed!
      .spawn()
      .unwrap();

    let stdout = cmd.stdout.unwrap();
    let mut parser = FfmpegLogParser::new(stdout);
    while let Ok(event) = parser.parse_next_event() {
      match event {
        FfmpegEvent::ParsedConfiguration(_) => return,
        _ => {}
      }
    }
    panic!() // should have found a configuration
  }

  /// Test case from https://github.com/nathanbabcock/ffmpeg-sidecar/issues/2#issue-1606661255
  #[test]
  fn test_macos_line_endings() {
    let stdout_str = "[info] ffmpeg version N-109875-geabc304d12-tessus  https://evermeet.cx/ffmpeg/  Copyright (c) 2000-2023 the FFmpeg developers\n[info]   built with Apple clang version 11.0.0 (clang-1100.0.33.17)\n[info]   configuration: --cc=/usr/bin/clang --prefix=/opt/ffmpeg --extra-version=tessus --enable-avisynth --enable-fontconfig --enable-gpl --enable-libaom --enable-libass --enable-libbluray --enable-libdav1d --enable-libfreetype --enable-libgsm --enable-libmodplug --enable-libmp3lame --enable-libmysofa --enable-libopencore-amrnb --enable-libopencore-amrwb --enable-libopenh264 --enable-libopenjpeg --enable-libopus --enable-librubberband --enable-libshine --enable-libsnappy --enable-libsoxr --enable-libspeex --enable-libtheora --enable-libtwolame --enable-libvidstab --enable-libvmaf --enable-libvo-amrwbenc --enable-libvorbis --enable-libvpx --enable-libwebp --enable-libx264 --enable-libx265 --enable-libxavs --enable-libxvid --enable-libzimg --enable-libzmq --enable-libzvbi --enable-version3 --pkg-config-flags=--static --disable-ffplay\n[info]   libavutil      58.  1.100 / 58.  1.100\n[info]   libavcodec     60.  2.100 / 60.  2.100\n[info]   libavformat    60.  2.100 / 60.  2.100\n[info]   libavdevice    60.  0.100 / 60.  0.100\n[info]   libavfilter     9.  2.100 /  9.  2.100\n[info]   libswscale      7.  0.100 /  7.  0.100\n[info]   libswresample   4.  9.100 /  4.  9.100\n[info]   libpostproc    57.  0.100 / 57.  0.100\n[info] Input #0, lavfi, from 'testsrc=duration=10':\n[info]   Duration: N/A, start: 0.000000, bitrate: N/A\n[info]   Stream #0:0: Video: wrapped_avframe, rgb24, 320x240 [SAR 1:1 DAR 4:3], 25 fps, 25 tbr, 25 tbn\n[info] Stream mapping:\n[info]   Stream #0:0 -> #0:0 (wrapped_avframe (native) -> rawvideo (native))\n[info] Press [q] to stop, [?] for help\n[info] Output #0, rawvideo, to 'pipe:':\n[info]   Metadata:\n[info]     encoder         : Lavf60.2.100\n[info]   Stream #0:0: Video: rawvideo (RGB[24] / 0x18424752), rgb24(progressive), 320x240 [SAR 1:1 DAR 4:3], q=2-31, 46080 kb/s, 25 fps, 25 tbn\n[info]     Metadata:\n[info]       encoder         : Lavc60.2.100 rawvideo\n[info] frame=    0 fps=0.0 q=0.0 size=       0kB time=-577014:32:22.77 bitrate=  -0.0kbits/s speed=N/A";

    // Emulate a stderr channel
    let mut cursor = Cursor::new(Vec::new());
    cursor.write_all(stdout_str.as_bytes()).unwrap();
    cursor.seek(SeekFrom::Start(0)).unwrap();

    let reader = BufReader::new(cursor);
    let mut parser = FfmpegLogParser::new(reader);
    let mut num_events = 0;
    while let Ok(event) = parser.parse_next_event() {
      match event {
        FfmpegEvent::LogEOF => break,
        _ => num_events += 1,
      }
    }
    assert!(num_events > 1);
  }
}
