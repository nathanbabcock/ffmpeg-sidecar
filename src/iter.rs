use std::{
  collections::VecDeque,
  io::{BufReader, ErrorKind, Read},
  process::{ChildStderr, ChildStdout},
  sync::mpsc::{sync_channel, Receiver, SyncSender},
  thread::JoinHandle,
};

use crate::{
  child::FfmpegChild,
  event::{AVStream, FfmpegEvent, FfmpegOutput, FfmpegProgress, OutputVideoFrame},
  log_parser::FfmpegLogParser,
  pix_fmt::get_bytes_per_frame,
};

/// An iterator over events from an ffmpeg process, including parsed metadata, progress, and raw video frames.
pub struct FfmpegIterator {
  rx: Receiver<FfmpegEvent>,
  event_queue: VecDeque<FfmpegEvent>,
}

impl FfmpegIterator {
  pub fn new(child: &mut FfmpegChild) -> Result<Self, String> {
    let stderr = child.take_stderr().ok_or("No stderr channel\n - Did you call `take_stderr` elsewhere?\n - Did you forget to call `.stderr(Stdio::piped)` on the `ChildProcess`?")?;
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
      let err = "No output streams found".to_string();
      child.kill().map_err(|e| e.to_string())?;
      Err(err)?
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
      FfmpegEvent::Error(e) | FfmpegEvent::LogError(e) => Some(e),
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
  pub fn filter_output(self) -> impl Iterator<Item = OutputVideoFrame> {
    self.filter_map(|event| match event {
      FfmpegEvent::OutputFrame(o) => Some(o),
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
      FfmpegEvent::LogInfo(x) => Some(x),
      FfmpegEvent::LogWarning(x) => Some(x),
      FfmpegEvent::LogError(x) => Some(x),
      FfmpegEvent::LogUnknown(x) => Some(x),
      FfmpegEvent::LogEOF => None,
      FfmpegEvent::Error(_) => None,
      FfmpegEvent::Progress(x) => Some(x.raw_log_message),
      FfmpegEvent::OutputFrame(_) => None,
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

    // Prepare buffers
    let mut buffers = stdout_output_streams
      .map(|stream| {
        let bytes_per_frame = get_bytes_per_frame(stream).unwrap();
        vec![0u8; bytes_per_frame as usize]
      })
      .collect::<Vec<Vec<u8>>>();

    // No buffers probably indicates that output is being sent to file
    if buffers.is_empty() {
      return;
    }

    // Read into buffers
    let mut buffer_index = (0..buffers.len()).cycle();
    let mut reader = BufReader::new(stdout);
    loop {
      let i = buffer_index.next().unwrap();
      let stream = &output_streams[i];
      let buffer = &mut buffers[i];
      match reader.read_exact(buffer.as_mut_slice()) {
        Ok(_) => tx
          .send(FfmpegEvent::OutputFrame(OutputVideoFrame {
            width: stream.width,
            height: stream.height,
            pix_fmt: stream.pix_fmt.clone(),
            output_index: i as u32,
            data: buffer.clone(),
          }))
          .ok(),
        Err(e) => match e.kind() {
          ErrorKind::UnexpectedEof => break,
          e => tx.send(FfmpegEvent::Error(e.to_string())).ok(),
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
