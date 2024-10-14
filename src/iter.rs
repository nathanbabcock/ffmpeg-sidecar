use std::{
  io::{BufReader, ErrorKind, Read},
  process::{ChildStderr, ChildStdout},
  sync::mpsc::{sync_channel, Receiver, SyncSender},
  thread::JoinHandle,
};

use anyhow::Context;

use crate::{
  child::FfmpegChild,
  event::{FfmpegEvent, FfmpegOutput, FfmpegProgress, LogLevel, OutputVideoFrame, Stream},
  log_parser::FfmpegLogParser,
  metadata::FfmpegMetadata,
  pix_fmt::get_bytes_per_frame,
};

/// An iterator over events from an ffmpeg process, including parsed metadata, progress, and raw video frames.
pub struct FfmpegIterator {
  rx: Receiver<FfmpegEvent>,
  tx: Option<SyncSender<FfmpegEvent>>,
  stdout: Option<ChildStdout>,
  metadata: FfmpegMetadata,
}

impl FfmpegIterator {
  pub fn new(child: &mut FfmpegChild) -> anyhow::Result<Self> {
    let stderr = child.take_stderr().context("No stderr channel\n - Did you call `take_stderr` elsewhere?\n - Did you forget to call `.stderr(Stdio::piped)` on the `ChildProcess`?")?;
    let (tx, rx) = sync_channel::<FfmpegEvent>(0);
    spawn_stderr_thread(stderr, tx.clone());
    let stdout = child.take_stdout();

    Ok(Self {
      rx,
      tx: Some(tx),
      stdout,
      metadata: FfmpegMetadata::new(),
    })
  }

  /// Called after all metadata has been obtained to spawn the thread that will
  /// handle output. The metadata is needed to determine the output format and
  /// other parameters.
  fn start_stdout(&mut self) -> anyhow::Result<()> {
    // No output detected
    if self.metadata.output_streams.is_empty() || self.metadata.outputs.is_empty() {
      let err = "No output streams found";
      self.tx.take(); // drop the tx so that the channel closes
      anyhow::bail!(err)
    }

    // Handle stdout
    if let Some(stdout) = self.stdout.take() {
      spawn_stdout_thread(
        stdout,
        self.tx.take().context("missing channel tx")?,
        self.metadata.output_streams.clone(),
        self.metadata.outputs.clone(),
      );
    }

    Ok(())
  }

  /// Advance the iterator until all metadata has been collected, returning it.
  pub fn collect_metadata(&mut self) -> anyhow::Result<FfmpegMetadata> {
    let mut event_queue: Vec<FfmpegEvent> = Vec::new();

    while !self.metadata.is_completed() {
      let event = self.next();
      match event {
        Some(e) => event_queue.push(e),
        None => {
          let errors = event_queue
            .iter()
            .filter_map(|e| match e {
              FfmpegEvent::Error(e) | FfmpegEvent::Log(LogLevel::Error, e) => Some(e.to_string()),
              _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

          anyhow::bail!(
            "Iterator ran out before metadata was gathered. The following errors occurred: {errors}",
          )
        }
      }
    }

    Ok(self.metadata.clone())
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
      FfmpegEvent::ParsedInput(input) => Some(input.raw_log_message),
      FfmpegEvent::ParsedDuration(duration) => Some(duration.raw_log_message),
    })
  }
}

impl Iterator for FfmpegIterator {
  type Item = FfmpegEvent;

  fn next(&mut self) -> Option<Self::Item> {
    let item = self.rx.recv().ok();

    if let Some(FfmpegEvent::LogEOF) = item {
      self.tx.take(); // drop the tx so that the receiver can close
    }

    if !self.metadata.is_completed() {
      match self.metadata.handle_event(&item) {
        Err(e) => return Some(FfmpegEvent::Error(e.to_string())),
        // TODO in this case, the preceding `item` is lost;
        // Probably better to queue it as the next item.
        Ok(()) if self.metadata.is_completed() => {
          if let Err(e) = self.start_stdout() {
            return Some(FfmpegEvent::Error(e.to_string()));
            // Same problem as above
          }
        }

        _ => {}
      }
    }

    item
  }
}

/// Spawn a thread to read raw output frames from ffmpeg's stdout.
pub fn spawn_stdout_thread(
  stdout: ChildStdout,
  tx: SyncSender<FfmpegEvent>,
  output_streams: Vec<Stream>,
  outputs: Vec<FfmpegOutput>,
) -> JoinHandle<()> {
  std::thread::spawn(move || {
    // Filter streams which are sent to stdout
    let stdout_output_video_streams = output_streams
      .iter()
      .filter(|stream| stream.is_video())
      .filter(|stream| {
        outputs
          .get(stream.parent_index)
          .map(|o| o.is_stdout())
          .unwrap_or(false)
      });

    // Error on mixing rawvideo and non-rawvideo streams
    // TODO: Maybe just revert to chunk mode if this happens?
    let any_rawvideo = stdout_output_video_streams
      .clone()
      .any(|s| s.format == "rawvideo");
    let any_non_rawvideo = stdout_output_video_streams
      .clone()
      .any(|s| s.format != "rawvideo");
    if any_rawvideo && any_non_rawvideo {
      panic!("Cannot mix rawvideo and non-rawvideo streams");
    }

    // Prepare buffers
    let mut buffers = stdout_output_video_streams
      .map(|stream| {
        let video_data = stream.video_data();
        let bytes_per_frame = get_bytes_per_frame(&video_data);
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
      let video_data = stream.video_data();
      let buffer = &mut buffers[i];
      let output_frame_num = frame_num / num_buffers;
      let timestamp = output_frame_num as f32 / video_data.fps;
      frame_num += 1;

      // Handle two scenarios:
      match stream.format.as_str() {
        // 1. `rawvideo` with exactly known pixel layout
        "rawvideo" => match reader.read_exact(buffer.as_mut_slice()) {
          Ok(_) => tx
            .send(FfmpegEvent::OutputFrame(OutputVideoFrame {
              width: video_data.width,
              height: video_data.height,
              pix_fmt: video_data.pix_fmt.clone(),
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
