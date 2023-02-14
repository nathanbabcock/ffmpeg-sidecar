use std::{
  io::{self, BufReader, Read, Write},
  process::{Child, ChildStdin},
  sync::mpsc::{sync_channel, Receiver, SyncSender},
  thread::JoinHandle,
};

use crate::{
  event::{AVStream, FfmpegEvent, OutputVideoFrame},
  log_parser::FfmpegLogParser,
  pix_fmt::get_bytes_per_frame,
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
  pub fn events_rx(&mut self) -> Result<Receiver<FfmpegEvent>, String> {
    let (tx, rx) = sync_channel::<FfmpegEvent>(0);
    self
      .spawn_stderr_thread(tx.clone())
      .map_err(|e| e.to_string())?;

    // Await the output metadata
    let mut output_streams: Vec<AVStream> = Vec::new();
    let mut event_queue: Vec<FfmpegEvent> = Vec::new();
    while let Ok(event) = rx.recv() {
      event_queue.push(event.clone());
      match event {
        FfmpegEvent::ParsedOutputStream(stream) => output_streams.push(stream.clone()),
        FfmpegEvent::Progress(_) => break,
        _ => {}
      }
    }

    // No output detected
    if output_streams.len() == 0 {
      let err = "No output streams found".to_string();
      self.kill().map_err(|err| err.to_string())?;
      Err(err)?
    }

    // Handle stdout
    self
      .spawn_stdout_thread(tx.clone(), output_streams)
      .map_err(|e| e.to_string())?;

    // Send the events we've already received
    for event in event_queue {
      tx.send(event).map_err(|e| e.to_string())?;
    }

    Ok(rx)
  }

  fn spawn_stderr_thread(&mut self, tx: SyncSender<FfmpegEvent>) -> Result<JoinHandle<()>, String> {
    let stderr = self.inner.stderr.take().ok_or("No stderr")?;
    let stderr_thread = std::thread::spawn(move || {
      let reader = BufReader::new(stderr);
      let mut parser = FfmpegLogParser::new(reader);
      loop {
        match parser.parse_next_event() {
          Ok(event) => tx.send(event).ok(),
          Err(e) => {
            eprintln!("Error parsing ffmpeg output: {}", e);
            break;
          }
        };
      }
    });
    Ok(stderr_thread)
  }

  fn spawn_stdout_thread(
    &mut self,
    tx: SyncSender<FfmpegEvent>,
    output_streams: Vec<AVStream>,
  ) -> Result<JoinHandle<()>, String> {
    let stdout = self.inner.stdout.take().ok_or("No stdout")?;
    let stdout_thread = std::thread::spawn(move || {
      // Prepare buffers
      let mut buffers = output_streams
        .iter()
        .map(|stream| {
          let bytes_per_frame = get_bytes_per_frame(stream).unwrap();
          let buf_size = stream.width * stream.height * bytes_per_frame;
          vec![0u8; buf_size as usize]
        })
        .collect::<Vec<Vec<u8>>>();
      assert!(buffers.len() == output_streams.len());
      let mut iter = output_streams.iter().zip(buffers.iter_mut()).enumerate();

      // Read into buffers
      let mut reader = BufReader::new(stdout);
      loop {
        let (i, (stream, buffer)) = iter.next().unwrap();
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
          Err(e) => {
            eprintln!("Error reading ffmpeg output: {}", e);
            break;
          }
        };
      }
    });
    Ok(stdout_thread)
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
