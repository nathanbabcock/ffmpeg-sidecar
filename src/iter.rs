//! A stream of events from an FFmpeg process.

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
    // todo: needs to handle audio streams as well!
    let stdout_output_video_streams = output_streams
      .iter()
      .filter(|stream| stream.is_video())
      .filter(|stream| {
        outputs
          .get(stream.parent_index as usize)
          .map(|o| o.is_stdout())
          .unwrap_or(false)
      });

    // Exit early if nothing is being sent to stdout
    if stdout_output_video_streams.clone().count() == 0 {
      return;
    }

    // If the size of a frame can't be determined, it will be read in arbitrary chunks.
    let mut chunked_mode = false;

    // Calculate frame buffer sizes up front.
    // Any sizes that cannot be calculated will trigger chunked mode.
    let frame_buffer_sizes: Vec<usize> = stdout_output_video_streams
      .clone()
      .map(|video_stream| {
        // Any non-rawvideo streams instantly enable chunked mode, since it's
        // impossible to tell when one chunk ends and another begins.
        if video_stream.format != "rawvideo" {
          chunked_mode = true;
          return 0;
        }

        // This is an unexpected error since we've already filtered for video streams.
        let Some(video_data) = video_stream.video_data() else {
          chunked_mode = true;
          return 0;
        };

        // This may trigger either on an unsupported pixel format, or
        // framebuffers with non-byte-aligned sizes. FFmpeg will pad these with
        // zeroes, but we can't predict the exact padding or end size on every format.
        let Some(bytes_per_frame) = get_bytes_per_frame(video_data) else {
          chunked_mode = true;
          return 0;
        };

        bytes_per_frame as usize
      })
      .collect();

    // Final check: FFmpeg supports multiple outputs interleaved on stdout,
    // but we can only keep track of them if the framerates match. It's
    // theoretically still possible to determine the expected frame order,
    // but it's not currently supported.
    let output_framerates: Vec<f32> = stdout_output_video_streams
      .clone()
      .filter(|s| s.format == "rawvideo")
      .map(|video_stream| {
        if let Some(video_data) = video_stream.video_data() {
          video_data.fps
        } else {
          -1.0
        }
      })
      .collect();
    let any_mismatched_framerates = output_framerates
      .iter()
      .any(|&fps| fps != output_framerates[0] || fps == -1.0);
    if any_mismatched_framerates {
      // This edge case is probably not what the user was intending,
      // so we'll notify with an error.
      tx.send(FfmpegEvent::Error(
        "Multiple output streams with different framerates are not supported when outputting to stdout. Falling back to chunked mode.".to_owned()
      )).ok();
      chunked_mode = true;
    }

    let mut reader = BufReader::new(stdout);
    if chunked_mode {
      // Arbitrary default buffer size for receiving indeterminate chunks
      // of any encoder or container output, when frame boundaries are unknown
      let mut chunk_buffer = vec![0u8; 65_536];
      loop {
        match reader.read(chunk_buffer.as_mut_slice()) {
          Ok(0) => break,
          Ok(bytes_read) => {
            let mut data = vec![0; bytes_read];
            data.clone_from_slice(&chunk_buffer[..bytes_read]);
            tx.send(FfmpegEvent::OutputChunk(data)).ok()
          }
          Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => break,
            e => tx.send(FfmpegEvent::Error(e.to_string())).ok(),
          },
        };
      }
    } else {
      // Prepare frame buffers
      let mut frame_buffers = frame_buffer_sizes
        .iter()
        .map(|&size| vec![0u8; size])
        .collect::<Vec<Vec<u8>>>();

      // Empty buffer array is unexpected at this point, since we've already ruled out
      // both chunked mode and non-stdout streams.
      if frame_buffers.is_empty() {
        tx.send(FfmpegEvent::Error("No frame buffers found".to_owned()))
          .ok();
        return;
      }

      // Read into buffers
      let num_frame_buffers = frame_buffers.len();
      let mut frame_buffer_index = (0..frame_buffers.len()).cycle();
      let mut frame_num = 0;
      loop {
        let i = frame_buffer_index.next().unwrap();
        let video_stream = &output_streams[i];
        let video_data = video_stream.video_data().unwrap();
        let buffer = &mut frame_buffers[i];
        let output_frame_num = frame_num / num_frame_buffers;
        let timestamp = output_frame_num as f32 / video_data.fps;
        frame_num += 1;

        match reader.read_exact(buffer.as_mut_slice()) {
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
        };
      }
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
