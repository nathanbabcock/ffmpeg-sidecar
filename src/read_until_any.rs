//! Internal utility; `BufRead::read_until` with multiple delimiters.

use std::io::{BufRead, ErrorKind, Result};

/// `BufRead::read_until` with multiple delimiters.
pub fn read_until_any<R: BufRead + ?Sized>(
  r: &mut R,
  delims: &[u8],
  buf: &mut Vec<u8>,
) -> Result<usize> {
  let mut read = 0;
  loop {
    let (done, used) = {
      let available = match r.fill_buf() {
        Ok(n) => n,
        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
        Err(e) => return Err(e),
      };

      // NB: `memchr` crate would be faster, but it's unstable and not worth the dependency.
      let first_delim_index = available
        .iter()
        .position(|b| delims.iter().any(|d| *d == *b));

      match first_delim_index {
        Some(i) => {
          buf.extend_from_slice(&available[..=i]);
          (true, i + 1)
        }
        None => {
          buf.extend_from_slice(available);
          (false, available.len())
        }
      }
    };
    r.consume(used);
    read += used;
    if done || used == 0 {
      return Ok(read);
    }
  }
}
