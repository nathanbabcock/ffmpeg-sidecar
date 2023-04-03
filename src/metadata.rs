use crate::error::Result;
use crate::event::{AVStream, FfmpegEvent, FfmpegInput, FfmpegOutput};

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegMetadata {
  expected_output_streams: usize,
  pub outputs: Vec<FfmpegOutput>,
  pub output_streams: Vec<AVStream>,
  pub inputs: Vec<FfmpegInput>,
  pub input_streams: Vec<AVStream>,

  /// Whether all metadata from the parent process has been gathered into this struct
  completed: bool,
}

impl Default for FfmpegMetadata {
  fn default() -> Self {
    Self::new()
  }
}

impl FfmpegMetadata {
  pub fn new() -> Self {
    Self {
      expected_output_streams: 0,
      outputs: Vec::new(),
      output_streams: Vec::new(),
      inputs: Vec::new(),
      input_streams: Vec::new(),
      completed: false,
    }
  }

  pub fn is_completed(&self) -> bool {
    self.completed
  }

  /// A shortcut to obtain the expected duration (in seconds).
  ///
  /// Usually this is the duration of the first input stream. Theoretically
  /// different streams could have different (or conflicting) durations, but
  /// this handles the common case.
  pub fn duration(&self) -> Option<f64> {
    self.inputs[0].duration
  }

  pub fn handle_event(&mut self, item: &Option<FfmpegEvent>) -> Result<()> {
    if self.is_completed() {
      return Err("Metadata is already completed".into());
    }

    match item {
      // Every stream mapping corresponds to one output stream
      // We count these to know when we've received all the output streams
      Some(FfmpegEvent::ParsedStreamMapping(_)) => self.expected_output_streams += 1,
      Some(FfmpegEvent::ParsedInput(input)) => self.inputs.push(input.clone()),
      Some(FfmpegEvent::ParsedOutput(output)) => self.outputs.push(output.clone()),
      Some(FfmpegEvent::ParsedDuration(duration)) => {
        self.inputs[duration.input_index as usize].duration = Some(duration.duration)
      }
      Some(FfmpegEvent::ParsedOutputStream(stream)) => self.output_streams.push(stream.clone()),
      Some(FfmpegEvent::ParsedInputStream(stream)) => self.input_streams.push(stream.clone()),
      _ => (),
    }

    if self.expected_output_streams > 0 && self.output_streams.len() == self.expected_output_streams
    {
      self.completed = true;
    }

    Ok(())
  }
}
