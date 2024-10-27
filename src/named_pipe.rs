use anyhow::{bail, Result};
use std::io::Read;

/// Cross-platform abstraction over Windows async named pipes and Unix FIFO.
pub struct NamedPipe {
  #[cfg(windows)]
  pub handle: *mut winapi::ctypes::c_void,
  // #[cfg(unix)] // todo
}

#[cfg(windows)]
impl NamedPipe {
  /// On Windows the pipe name must be in the format `\\.\pipe\{pipe_name}`.
  /// @see https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew
  pub fn new<S: AsRef<str>>(pipe_name: S) -> Result<Self> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::namedpipeapi::CreateNamedPipeW; // Corrected import
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
        1024 * 1024,
        1024 * 1024,
        0,
        null_mut(),
      )
    };

    if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
      bail!("Failed to create named pipe");
    }

    Ok(Self { handle })
  }
}

#[cfg(windows)]
impl Drop for NamedPipe {
  fn drop(&mut self) {
    unsafe {
      winapi::um::handleapi::CloseHandle(self.handle);
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
    let success = unsafe {
      ReadFile(
        self.handle,
        buf.as_mut_ptr() as LPVOID,
        buf.len() as DWORD,
        &mut bytes_read,
        null_mut(),
      )
    };

    match success {
      0 => std::io::Result::Err(Error::last_os_error()),
      _ => Ok(bytes_read as usize),
    }
  }
}
