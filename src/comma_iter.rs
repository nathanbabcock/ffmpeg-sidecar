//! An internal utility used to parse comma-separated values in FFmpeg logs.

use std::str::Chars;

/// An iterator over comma-separated values, **ignoring commas inside parentheses**.
///
/// ## Examples
///
/// ```rust
/// use ffmpeg_sidecar::comma_iter::CommaIter;
///
/// let string = "foo(bar,baz),quux";
/// let mut iter = CommaIter::new(string);
///
/// assert_eq!(iter.next(), Some("foo(bar,baz)"));
/// assert_eq!(iter.next(), Some("quux"));
/// assert_eq!(iter.next(), None);
/// ```
pub struct CommaIter<'a> {
  chars: Chars<'a>,
}

impl<'a> CommaIter<'a> {
  pub fn new(string: &'a str) -> Self {
    Self {
      chars: string.chars(),
    }
  }
}

impl<'a> Iterator for CommaIter<'a> {
  type Item = &'a str;

  /// Return the next comma-separated section, not including the comma.
  fn next(&mut self) -> Option<Self::Item> {
    let chars_clone = self.chars.clone();
    let mut i = 0;

    while let Some(char) = self.chars.next() {
      match char {
        '(' => {
          // advance until closing paren (only handles one level nesting)
          for close_paren in self.chars.by_ref() {
            i += 1;
            if close_paren == ')' {
              break;
            }
          }
        }
        ',' => break,
        _ => {}
      }
      i += 1;
    }

    match i {
      0 => None,
      _ => Some(&chars_clone.as_str()[..i]),
    }
  }
}
