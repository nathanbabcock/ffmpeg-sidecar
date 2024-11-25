//! Internal utility; `BufRead::read_until` with multiple delimiters.

use std::io::{BufRead, ErrorKind, Result};

/// Reads from the provided buffer until any of the delimiter bytes match.
/// The output buffer will include the ending delimiter.
/// Also skips over zero-length reads.
/// See [`BufRead::read_until`](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_until).
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

      let start_delims = if read == 0 {
        available
          .iter()
          .take_while(|&&b| delims.iter().any(|&d| d == b))
          .count()
      } else {
        0
      };

      // NB: `memchr` crate would be faster, but it's unstable and not worth the dependency.
      let first_delim_index = available
        .iter()
        .skip(start_delims)
        .position(|&b| delims.iter().any(|&d| d == b))
        .map(|i| i + start_delims);

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

    if done {
      return Ok(read);
    }

    // Discard final trailing delimiters
    if used == 0 && buf.iter().all(|&b| delims.iter().any(|&d| d == b)) {
      return Ok(0);
    }

    if used == 0 {
      return Ok(read);
    }
  }
}
