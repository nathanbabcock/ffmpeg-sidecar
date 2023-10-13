use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;
use std::result::Result as StdResult;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

/// Shorthand alias for `Result<T, Error>` using `ffmpeg_sidecar` error type.
pub type Result<T> = StdResult<T, Error>;

/// A generic error type for the `ffmpeg-sidecar` crate.
#[derive(Debug)]
pub struct Error {
  pub message: String,
  pub source: Option<Box<dyn StdError + 'static>>,
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl StdError for Error {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    self.source.as_deref()
  }
}

impl Error {
  /// Wrap any standard Error into a library Error.
  /// Similar to [`anyhow`](https://github.com/dtolnay/anyhow/blob/master/src/error.rs#L88).
  pub fn from_std<E>(e: E) -> Self
  where
    E: StdError + 'static,
  {
    Error {
      message: e.to_string(),
      source: Some(Box::new(e)),
    }
  }

  /// Wrap any Display into a library Error.
  pub fn from_display<E>(e: E) -> Self
  where
    E: Display,
  {
    Error {
      message: e.to_string(),
      source: None,
    }
  }

  /// Create an error message from a string.
  pub fn msg<S: AsRef<str>>(message: S) -> Self {
    Error {
      message: message.as_ref().to_string(),
      source: None,
    }
  }
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self {
    Error::from_std(e)
  }
}

impl From<Utf8Error> for Error {
  fn from(e: Utf8Error) -> Self {
    Error::from_std(e)
  }
}

impl From<FromUtf8Error> for Error {
  fn from(e: FromUtf8Error) -> Self {
    Error::from_std(e)
  }
}

impl From<&str> for Error {
  fn from(e: &str) -> Self {
    Error::from_display(e)
  }
}

impl From<String> for Error {
  fn from(e: String) -> Self {
    Error::from_display(e)
  }
}

impl From<()> for Error {
  fn from(_: ()) -> Self {
    Error::from_display("empty error")
  }
}
