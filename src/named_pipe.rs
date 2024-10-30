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
      bail!("Failed to create named pipe");
    }

    // // wait for the named pipe's creation
    // // https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-waitnamedpipew
    // // nTimeOut = 0 -> "The time-out interval is the default value specified by the server process in the CreateNamedPipe function."
    // unsafe {
    //   let wait_result = WaitNamedPipeW(path_wide.as_ptr(), 50);
    //   if wait_result == 0 {
    //     bail!("Failed to wait for named pipe");
    //   }
    // }

    Ok(Self { handle })
  }
}

// #[cfg(windows)]
// impl Drop for NamedPipe {
//   fn drop(&mut self) {
//     unsafe {
//       winapi::um::handleapi::CloseHandle(self.handle);
//     }
//   }
// }

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
        self.handle,
        buf.as_mut_ptr() as LPVOID,
        buf.len() as DWORD,
        &mut bytes_read,
        null_mut(),
      );
      if read_status == 0 {
        return std::io::Result::Err(Error::last_os_error());
      }
    };

    Ok(bytes_read as usize)
  }
}
