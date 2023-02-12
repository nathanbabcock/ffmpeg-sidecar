use std::{
  io::{self, BufRead, BufReader, Write},
  process::{Child, ChildStdin},
  sync::mpsc::{sync_channel, Receiver, SyncSender},
  thread::JoinHandle,
};

use crate::{
  event::{FfmpegEvent, FfmpegOutputs},
  log_parser::FfmpegLogParser,
};

/// A wrapper around [`std::process::Child`] containing a spawned FFmpeg command.
/// Provides interfaces for reading parsed metadata, progress updates, warnings and errors, and
/// piped output frames if applicable.
pub struct FfmpegChild {
  inner: Child,
  stdin: ChildStdin,
}

impl FfmpegChild {
  /// Creates a receiver for events emitted by ffmpeg.
  pub(crate) fn events_rx(&mut self) -> Receiver<FfmpegEvent> {
    let (tx, rx) = sync_channel::<FfmpegEvent>(0);
    self.spawn_stderr_thread(tx.clone());

    // Await the output metadata
    let mut outputs: Option<FfmpegOutputs> = None;
    let mut event_queue: Vec<FfmpegEvent> = Vec::new();
    while let Ok(event) = rx.recv() {
      match event {
        FfmpegEvent::ParsedOutputs(_outputs) => {
          outputs = Some(_outputs);
          break;
        }
        FfmpegEvent::Progress(progress) => {
          panic!("unexpected progress event before output metadata")
        }
        _ => {}
      }
      event_queue.push(event);
    }

    // Once processing has started, make sure we have the output metadata
    let output_stream = outputs.unwrap().streams.first().unwrap();
    todo!("handle 0 or >1 output streams");
    let width = output_stream.width;
    let height = output_stream.height;
    let pix_fmt = output_stream.pix_fmt;
    let bytes_per_pixel: u32 = todo!("retrieve bytes per pixel from pix_fmt.rs");
    let frame_size = width * height * bytes_per_pixel;
    let buffer = vec![0; frame_size as usize];
    todo!("spawn a thread to read stdout into buffer");

    rx
  }

  fn spawn_stderr_thread(&mut self, tx: SyncSender<FfmpegEvent>) -> JoinHandle<()> {
    let stderr = self.inner.stderr.take().unwrap();
    let stderr_thread = std::thread::spawn(move || {
      let reader = BufReader::new(stderr);
      let mut parser = FfmpegLogParser::new(reader);
      loop {
        match parser.parse_next_line() {
          Ok(event) => tx.send(event).unwrap(),
          Err(e) => {
            eprintln!("Error parsing ffmpeg output: {}", e);
            break;
          }
        };
      }
    });
    stderr_thread
  }

  /// Creates an iterator over events emitted by ffmpeg. Functions similarly to
  /// `Lines` from [`std::io::BufReader`], but providing a variety of parsed
  /// events:
  /// - Log messages
  /// - Parsed metadata
  /// - Progress updates
  /// - Errors and warnings
  /// - Raw output frames
  pub fn events_iter() {
    todo!()
  }

  /// Send a command to ffmpeg over stdin, used during interactive mode.
  ///
  /// This method does not validate that the command is expected or handled
  /// correctly by ffmpeg. The returned `io::Result` indicates only whether the
  /// command was successfully sent or not.
  ///
  /// In a typical ffmpeg build, these are the supported commands:
  ///
  /// ```txt
  /// ?      show this help
  /// +      increase verbosity
  /// -      decrease verbosity
  /// c      Send command to first matching filter supporting it
  /// C      Send/Queue command to all matching filters
  /// D      cycle through available debug modes
  /// h      dump packets/hex press to cycle through the 3 states
  /// q      quit
  /// s      Show QP histogram
  /// ```
  pub fn send_stdin_command(&mut self, command: &[u8]) -> io::Result<()> {
    self.stdin.write_all(command)
  }

  /// Send a `q` command to ffmpeg over stdin,
  /// requesting a graceful shutdown as soon as possible.
  ///
  /// This method returns after the command has been sent; the actual shut down
  /// may take a few more frames as ffmpeg flushes its buffers and writes the
  /// trailer, if applicable.
  pub fn quit(&mut self) -> io::Result<()> {
    self.send_stdin_command(b"q")
  }

  /// Forcibly terminate the inner child process.
  ///
  /// Alternatively, you may choose to gracefully stop the child process by
  /// sending a command over stdin, using the `quit` method.
  ///
  /// Identical to `kill` in [`std::process::Child`].
  pub fn kill(&mut self) -> io::Result<()> {
    self.inner.kill()
  }

  /// Wrap a [`std::process::Child`] in a `FfmpegChild`.
  /// Should typically only be called by `FfmpegCommand::spawn`.
  ///
  /// ## Panics
  ///
  /// Panics if the child process's stdin was not piped. This could be because
  /// ffmpeg was spawned with `-nostdin`, or if the `Child` instance was not
  /// configured with `stdin(Stdio::piped())`.
  pub(crate) fn from_inner(mut inner: Child) -> Self {
    let stdin = inner.stdin.take().expect("Child stdin was not piped");
    Self { inner, stdin }
  }

  /// Escape hatch to access the inner `Child`.
  pub fn as_inner(&mut self) -> &Child {
    &self.inner
  }

  /// Escape hatch to mutably access the inner `Child`.
  pub fn as_inner_mut(&mut self) -> &mut Child {
    &mut self.inner
  }
}
