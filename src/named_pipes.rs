//! Cross-platform abstraction over Windows async named pipes and Unix FIFO.
//!
//! The primary use-case is streaming multiple outputs from FFmpeg into a Rust program.
//! For more commentary and end-to-end usage, see `examples/named_pipes.rs`:
//! <https://github.com/nathanbabcock/ffmpeg-sidecar/blob/main/examples/named_pipes.rs>

use anyhow::Result;
use std::io::Read;

/// On Windows, prepend the pipe name with `\\.\pipe\`.
/// On Unix, return the name as-is.
#[macro_export]
macro_rules! pipe_name {
  ($name:expr) => {
    if cfg!(windows) {
      concat!(r#"\\.\pipe\"#, $name)
    } else {
      $name
    }
  };
}

/// Windows-only; an FFI pointer to a named pipe handle.
#[cfg(windows)]
pub struct NamedPipeHandle(*mut winapi::ctypes::c_void);

/// <https://github.com/retep998/winapi-rs/issues/396>
#[cfg(windows)]
unsafe impl Send for NamedPipeHandle {}

/// Cross-platform abstraction over Windows async named pipes and Unix FIFO.
pub struct NamedPipe {
  /// The name that the pipe was opened with. It will start with `\\.\pipe\` on Windows.
  pub name: String,

  /// Windows-only; an FFI pointer to a named pipe handle.
  #[cfg(windows)]
  pub handle: NamedPipeHandle,

  /// Unix-only; a blocking file handle to the FIFO.
  #[cfg(unix)]
  pub file: std::fs::File,
}

#[cfg(windows)]
impl NamedPipe {
  /// On Windows the pipe name must be in the format `\\.\pipe\{pipe_name}`.
  /// @see <https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew>
  pub fn new<S: AsRef<str>>(pipe_name: S) -> Result<Self> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::namedpipeapi::CreateNamedPipeW;
    use winapi::um::winbase::{PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE, PIPE_WAIT};

    let path_wide: Vec<u16> = OsStr::new(pipe_name.as_ref())
      .encode_wide()
      .chain(Some(0))
      .collect();

    let handle = unsafe {
      CreateNamedPipeW(
        path_wide.as_ptr(),
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_WAIT,
        1,
        1024 * 1024 * 64,
        1024 * 1024 * 64,
        0, // "A value of zero will result in a default time-out of 50 milliseconds."
        null_mut(),
      )
    };

    if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
      anyhow::bail!("Failed to create named pipe");
    }

    Ok(Self {
      handle: NamedPipeHandle(handle),
      name: pipe_name.as_ref().to_string(),
    })
  }
}

#[cfg(windows)]
impl Drop for NamedPipe {
  fn drop(&mut self) {
    unsafe {
      winapi::um::handleapi::CloseHandle(self.handle.0);
    }
  }
}

#[cfg(windows)]
impl Read for NamedPipe {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    use std::io::Error;
    use std::ptr::null_mut;
    use winapi::{
      shared::minwindef::{DWORD, LPVOID},
      um::fileapi::ReadFile,
    };

    let mut bytes_read: DWORD = 0;
    unsafe {
      let read_status = ReadFile(
        self.handle.0,
        buf.as_mut_ptr() as LPVOID,
        buf.len() as DWORD,
        &mut bytes_read,
        null_mut(),
      );
      if read_status == 0 {
        let error = Error::last_os_error();
        if error.raw_os_error() == Some(109) {
          // pipe has been closed since last read
          return Ok(0);
        } else {
          return std::io::Result::Err(error);
        }
      }
    };

    Ok(bytes_read as usize)
  }
}

// The unix implementation is comparatively quite simple...

#[cfg(unix)]
impl NamedPipe {
  pub fn new<S: AsRef<str>>(pipe_name: S) -> Result<Self> {
    use nix::{fcntl::OFlag, sys::stat, unistd};
    use std::os::unix::fs::OpenOptionsExt;
    unistd::mkfifo(pipe_name.as_ref(), stat::Mode::S_IRWXU)?;

    // Open in non-blocking mode so the function completes
    let file = std::fs::OpenOptions::new()
      .read(true)
      .custom_flags(OFlag::O_NONBLOCK.bits())
      .open(pipe_name.as_ref())?;

    // Switch to blocking mode so it doesn't read too early
    nix::fcntl::fcntl(&file, nix::fcntl::FcntlArg::F_SETFL(OFlag::empty()))?;

    Ok(Self {
      file,
      name: pipe_name.as_ref().to_string(),
    })
  }
}

#[cfg(unix)]
impl Read for NamedPipe {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    self.file.read(buf)
  }
}

#[cfg(unix)]
impl Drop for NamedPipe {
  fn drop(&mut self) {
    use nix::unistd;
    use std::path::Path;
    unistd::unlink(Path::new(&self.name)).ok();
  }
}
