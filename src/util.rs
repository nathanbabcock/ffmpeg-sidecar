use std::str::Chars;

/// Consume chars until a comma is found,
/// **except** if the comma is contained within parentheses.
///
/// ## Examples
///
/// ### Ignore commas inside parentheses
///
/// ```rust
/// use ffmpeg_sidecar::util::collect_until_comma;
/// let string = "foo(bar,baz),quux";
/// let mut chars = string.chars();
/// let part = collect_until_comma(&mut chars);
/// assert_eq!(part, "foo(bar,baz)");
/// ```
///
/// ### Without parentheses
///
/// ```rust
/// use ffmpeg_sidecar::util::collect_until_comma;
/// let string2 = "a,b,c";
/// let part2 = collect_until_comma(&mut string2.chars());
/// assert_eq!(part2, "a");
/// ```
pub fn collect_until_comma<T: Iterator<Item = char> + Clone>(chars: &mut T) -> String {
  let chars_clone = chars.clone();
  let mut i = 0;
  while let Some(char) = chars.next() {
    match char {
      '(' => {
        // advance until closing paren (only handles one level nesting)
        while let Some(close_paren) = chars.next() {
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
  chars_clone.take(i).collect()
}
