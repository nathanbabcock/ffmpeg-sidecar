use std::{
  collections::VecDeque,
  io::{BufReader, ErrorKind, Read},
  process::{ChildStderr, ChildStdout},
  sync::mpsc::{sync_channel, Receiver, SyncSender},
  thread::JoinHandle,
};

use crate::{
  child::FfmpegChild,
  error::{Error, Result},
  event::{AVStream, FfmpegEvent, FfmpegOutput, FfmpegProgress, LogLevel, OutputVideoFrame},
  log_parser::FfmpegLogParser,
  pix_fmt::get_bytes_per_frame,
};

/// An iterator over events from an ffmpeg process, including parsed metadata, progress, and raw video frames.
pub struct FfmpegIterator {
  rx: Receiver<FfmpegEvent>,
  event_queue: VecDeque<FfmpegEvent>,
}

impl FfmpegIterator {
  pub fn new(child: &mut FfmpegChild) -> Result<Self> {
    let stderr = child.take_stderr().ok_or_else(|| Error::msg("No stderr channel\n - Did you call `take_stderr` elsewhere?\n - Did you forget to call `.stderr(Stdio::piped)` on the `ChildProcess`?"))?;
    let (tx, rx) = sync_channel::<FfmpegEvent>(0);
    spawn_stderr_thread(stderr, tx.clone());

    // Await the output metadata
    let mut output_streams: Vec<AVStream> = Vec::new();
    let mut outputs: Vec<FfmpegOutput> = Vec::new();
    let mut expected_output_streams = 0;
    let mut event_queue: VecDeque<FfmpegEvent> = VecDeque::new();
    while let Ok(event) = rx.recv() {
      event_queue.push_back(event.clone());
      match event {
        // Every stream mapping corresponds to one output stream
        // We count these to know when we've received all the output streams
        FfmpegEvent::ParsedStreamMapping(_) => expected_output_streams += 1,
        FfmpegEvent::ParsedOutput(output) => outputs.push(output),
        FfmpegEvent::ParsedOutputStream(stream) => {
          output_streams.push(stream.clone());
          if output_streams.len() == expected_output_streams {
            break;
          }
        }
        FfmpegEvent::LogEOF => {
          // An unexpected EOF means we bail out here,
          // but still pass on the events we've already received
          return Ok(Self { rx, event_queue });
        }
        _ => {}
      }
    }

    // No output detected
    if output_streams.is_empty() || outputs.is_empty() {
      let err = Error::msg("No output streams found");
      child.kill()?;
      Err(err)? // this is just a cute way of saying `return err`
    }

    // Handle stdout
    if let Some(stdout) = child.take_stdout() {
      spawn_stdout_thread(stdout, tx, output_streams, outputs);
    }

    Ok(Self { rx, event_queue })
  }

  //// Iterator filters

  /// Returns an iterator over error messages (`FfmpegEvent::Error` and `FfmpegEvent::LogError`).
  pub fn filter_errors(self) -> impl Iterator<Item = String> {
    self.filter_map(|event| match event {
      FfmpegEvent::Error(e) | FfmpegEvent::Log(LogLevel::Error, e) => Some(e),
      _ => None,
    })
  }

  /// Filter out all events except for progress (`FfmpegEvent::Progress`).
  pub fn filter_progress(self) -> impl Iterator<Item = FfmpegProgress> {
    self.filter_map(|event| match event {
      FfmpegEvent::Progress(p) => Some(p),
      _ => None,
    })
  }

  /// Filter out all events except for output frames (`FfmpegEvent::OutputFrame`).
  pub fn filter_frames(self) -> impl Iterator<Item = OutputVideoFrame> {
    self.filter_map(|event| match event {
      FfmpegEvent::OutputFrame(o) => Some(o),
      _ => None,
    })
  }

  /// Filter out all events except for output chunks (`FfmpegEvent::OutputChunk`).
  pub fn filter_chunks(self) -> impl Iterator<Item = Vec<u8>> {
    self.filter_map(|event| match event {
      FfmpegEvent::OutputChunk(vec) => Some(vec),
      _ => None,
    })
  }

  /// Iterator over every message from ffmpeg's stderr as a raw string.
  /// Conceptually equivalent to `BufReader::new(ffmpeg_stderr).lines()`.
  pub fn into_ffmpeg_stderr(self) -> impl Iterator<Item = String> {
    self.filter_map(|event| match event {
      FfmpegEvent::ParsedVersion(x) => Some(x.raw_log_message),
      FfmpegEvent::ParsedConfiguration(x) => Some(x.raw_log_message),
      FfmpegEvent::ParsedStreamMapping(x) => Some(x),
      FfmpegEvent::ParsedOutput(x) => Some(x.raw_log_message),
      FfmpegEvent::ParsedInputStream(x) => Some(x.raw_log_message),
      FfmpegEvent::ParsedOutputStream(x) => Some(x.raw_log_message),
      FfmpegEvent::Log(_, x) => Some(x),
      FfmpegEvent::LogEOF => None,
      FfmpegEvent::Error(_) => None,
      FfmpegEvent::Progress(x) => Some(x.raw_log_message),
      FfmpegEvent::OutputFrame(_) => None,
      FfmpegEvent::OutputChunk(_) => None,
      FfmpegEvent::Done => None,
    })
  }
}

impl Iterator for FfmpegIterator {
  type Item = FfmpegEvent;

  fn next(&mut self) -> Option<Self::Item> {
    match self.event_queue.pop_front() {
      // First, re-send the queued events that were used to parse metadata
      Some(event) => Some(event),

      // Then, read from the channel or return `None` when it closes
      None => self.rx.recv().ok(),
    }
  }
}

/// Spawn a thread to read raw output frames from ffmpeg's stdout.
pub fn spawn_stdout_thread(
  stdout: ChildStdout,
  tx: SyncSender<FfmpegEvent>,
  output_streams: Vec<AVStream>,
  outputs: Vec<FfmpegOutput>,
) -> JoinHandle<()> {
  std::thread::spawn(move || {
    // Filter streams which are sent to stdout
    let stdout_output_streams = output_streams.iter().filter(|stream| {
      outputs
        .get(stream.parent_index)
        .map(|o| o.is_stdout())
        .unwrap_or(false)
    });

    // Limit to exactly one non-rawvideo stream,
    // or unlimited rawvideo streams
    if stdout_output_streams
      .clone()
      .any(|s| s.format != "rawvideo")
    {
      assert!(
        stdout_output_streams.clone().count() == 1,
        "Only one non-rawvideo output stream can be sent to stdout",
      );
    }

    // Prepare buffers
    let mut buffers = stdout_output_streams
      .map(|stream| {
        let bytes_per_frame = get_bytes_per_frame(stream);
        let buf_size = match stream.format.as_str() {
          "rawvideo" => bytes_per_frame.expect("Should use a known pix_fmt") as usize,

          // Arbitrary default buffer size for receiving indeterminate chunks
          // of any encoder or container output, when frame boundaries are unknown
          _ => 32_768, // ~= 32mb (plenty large enough for any chunk of video at reasonable bitrate)
        };

        // Catch unsupported pixel formats
        assert!(
          buf_size > 0,
          "Unsupported pixel format with 0 bytes per pixel"
        );

        vec![0u8; buf_size]
      })
      .collect::<Vec<Vec<u8>>>();

    // No buffers probably indicates that output is being sent to file
    if buffers.is_empty() {
      return;
    }

    // Read into buffers
    let num_buffers = buffers.len();
    let mut buffer_index = (0..buffers.len()).cycle();
    let mut reader = BufReader::new(stdout);
    let mut frame_num = 0;
    loop {
      let i = buffer_index.next().unwrap();
      let stream = &output_streams[i];
      let buffer = &mut buffers[i];
      let output_frame_num = frame_num / num_buffers;
      let timestamp = output_frame_num as f32 / stream.fps;
      frame_num += 1;

      // Handle two scenarios:
      match stream.format.as_str() {
        // 1. `rawvideo` with exactly known pixel layout
        "rawvideo" => match reader.read_exact(buffer.as_mut_slice()) {
          Ok(_) => tx
            .send(FfmpegEvent::OutputFrame(OutputVideoFrame {
              width: stream.width,
              height: stream.height,
              pix_fmt: stream.pix_fmt.clone(),
              output_index: i as u32,
              data: buffer.clone(),
              frame_num: output_frame_num as u32,
              timestamp,
            }))
            .ok(),
          Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => break,
            e => tx.send(FfmpegEvent::Error(e.to_string())).ok(),
          },
        },

        // 2. Anything else, with unknown buffer size
        _ => match reader.read(buffer.as_mut_slice()) {
          Ok(0) => break,
          Ok(bytes_read) => {
            let mut data = vec![0; bytes_read];
            data.clone_from_slice(&buffer[..bytes_read]);
            tx.send(FfmpegEvent::OutputChunk(data)).ok()
          }
          Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => break,
            e => tx.send(FfmpegEvent::Error(e.to_string())).ok(),
          },
        },
      };
    }
    tx.send(FfmpegEvent::Done).ok();
  })
}

/// Spawn a thread which reads and parses lines from ffmpeg's stderr channel.
/// The cadence is controlled by the synchronous `tx` channel, which blocks
/// until a receiver is ready to receive the next event.
pub fn spawn_stderr_thread(stderr: ChildStderr, tx: SyncSender<FfmpegEvent>) -> JoinHandle<()> {
  std::thread::spawn(move || {
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);
    loop {
      match parser.parse_next_event() {
        Ok(FfmpegEvent::LogEOF) => {
          tx.send(FfmpegEvent::LogEOF).ok();
          break;
        }
        Ok(event) => tx.send(event).ok(),
        Err(e) => {
          eprintln!("Error parsing ffmpeg output: {}", e);
          break;
        }
      };
    }
  })
}
