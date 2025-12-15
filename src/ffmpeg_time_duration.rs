use std::borrow::Cow;
use std::fmt;
use std::convert::TryFrom;
use std::ops::{Add, Sub};
use std::time::Duration;

const MICROS_PER_SEC: f64 = 1_000_000.0;
const MILLIS_PER_SEC: f64 = 1_000.0;

/// A [`FfmpegTimeDuration`] type to represent a ffmpeg time duration described in
/// [FFmpeg documentation](https://ffmpeg.org/ffmpeg-utils.html#Time-duration)
///
/// This type wraps an `i64` internally, representing time in microseconds, similar to how
/// FFmpeg handles time durations internally.
///
/// # Conversions
///
/// The `From` trait implementations for numeric types (such as `f64`, `f32`, `i64`, etc.)
/// convert between `FfmpegTimeDuration` and numeric values in **seconds**:
/// - Converting from a numeric type to `FfmpegTimeDuration` interprets the numeric value as seconds
/// - Converting from `FfmpegTimeDuration` to a numeric type returns the duration in seconds
/// - Adding or substructing to numeric values add or subtracts seconds
///
/// # Traits
///
/// [`FfmpegTimeDuration`] implement many common traits, including [`Add`], [`Sub`].
/// It implements [`Default`] by returning a zero-length `FfmpegTimeDuration`.
///
/// # Examples
///
/// ```rust
/// use ffmpeg_sidecar::ffmpeg_time_duration::FfmpegTimeDuration;
/// use ffmpeg_sidecar::command::FfmpegCommand;
///
/// let second = FfmpegTimeDuration::from_str("00:00:01").unwrap();
/// let hundred_milliseconds = FfmpegTimeDuration::from_str("100ms").unwrap();
///
/// assert_eq!(second.as_seconds(), 1.0);
/// assert_eq!(hundred_milliseconds.as_seconds(), 0.1);
///
/// FfmpegCommand::new()
///    .arg("-ss")
///    .arg(second.to_string());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct FfmpegTimeDuration(i64);

impl FfmpegTimeDuration {
    #[must_use]
    #[inline]
    pub fn new(microseconds: i64) -> Self {
        Self::from_micros(microseconds)
    }

    #[must_use]
    #[inline]
    pub fn as_micros(&self) -> i64 {
        self.0
    }

    #[must_use]
    #[inline]
    pub fn from_micros(microseconds: i64) -> Self {
        Self(microseconds)
    }

    #[must_use]
    #[inline]
    pub fn as_seconds(self) -> f64 {
        self.0 as f64 / MICROS_PER_SEC
    }

    #[must_use]
    #[inline]
    pub fn from_seconds(seconds: f64) -> Self {
        Self::from_micros((seconds * MICROS_PER_SEC) as i64)
    }

    #[must_use]
    pub fn from_str(str: &str) -> Option<Self> {
        let str = str.trim();

        // Handle negative values
        let (is_negative, str) = if str.starts_with('-') {
            (true, &str[1..])
        } else {
            (false, str)
        };

        let mut micros: i64;

        // Check for microseconds suffix
        if str.ends_with("us") {
            let value = str.trim_end_matches("us").trim().parse::<f64>().ok()?;
            micros = value as i64;
        }
        // Check for milliseconds suffix
        else if str.ends_with("ms") {
            let value = str.trim_end_matches("ms").trim().parse::<f64>().ok()?;
            micros = (value * MILLIS_PER_SEC) as i64;
        }
        // Check for HH:MM:SS format
        else if str.contains(':') {
            let mut seconds = 0.0;
            let mut smh = str.split(':').rev();
            if let Some(sec) = smh.next() {
                seconds += sec.parse::<f64>().ok()?;
            }

            if let Some(min) = smh.next() {
                seconds += min.parse::<f64>().ok()? * 60.0;
            }

            if let Some(hrs) = smh.next() {
                seconds += hrs.parse::<f64>().ok()? * 60.0 * 60.0;
            }
            micros = (seconds * MICROS_PER_SEC) as i64;
        }
        // Plain numeric value (seconds)
        else {
            let seconds = str.parse::<f64>().ok()?;
            micros = (seconds * MICROS_PER_SEC) as i64;
        }

        if is_negative {
            micros = -micros;
        }

        Some(Self::from_micros(micros))
    }

    #[must_use]
    #[inline]
    pub fn as_duration(&self) -> Duration {
        Duration::from_micros(self.0.unsigned_abs())
    }

    #[must_use]
    #[inline]
    pub fn from_duration(duration: Duration) -> Self {
        Self::from_micros(duration.as_micros() as i64)
    }

    /// Returns a string representation of the duration in microseconds with "us" suffix.
    #[must_use]
    #[inline]
    pub fn to_alt_string(&self) -> String {
        format!("{:#}", self)
    }
}

impl fmt::Display for FfmpegTimeDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            let seconds = self.as_seconds();
            let is_negative = seconds < 0.0;
            let abs_seconds = seconds.abs();

            let hours = (abs_seconds / 3600.0).floor() as i64;
            let minutes = ((abs_seconds / 60.0) % 60.0).floor() as i64;
            let secs = abs_seconds % 60.0;

            if is_negative {
                write!(f, "-{:02}:{:02}:{:06.3}", hours, minutes, secs)
            } else {
                write!(f, "{:02}:{:02}:{:06.3}", hours, minutes, secs)
            }
        } else {
            write!(f, "{}us", self.as_micros())
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseFfmpegTimeStrError;

impl fmt::Display for ParseFfmpegTimeStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse FFmpeg time string")
    }
}

impl std::error::Error for ParseFfmpegTimeStrError {}

impl TryFrom<String> for FfmpegTimeDuration {
    type Error = ParseFfmpegTimeStrError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        FfmpegTimeDuration::from_str(&value).ok_or(ParseFfmpegTimeStrError)
    }
}

impl TryFrom<&str> for FfmpegTimeDuration {
    type Error = ParseFfmpegTimeStrError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        FfmpegTimeDuration::from_str(value).ok_or(ParseFfmpegTimeStrError)
    }
}

macro_rules! impl_from_to_numeric {
    ($($t:ty),*) => {
        $(
            impl From<$t> for FfmpegTimeDuration {
                fn from(value: $t) -> Self {
                    Self::from_seconds(value as f64)
                }
            }

            impl From<FfmpegTimeDuration> for $t {
                fn from(value: FfmpegTimeDuration) -> Self {
                    value.as_seconds() as $t
                }
            }

            impl Add<$t> for FfmpegTimeDuration {
                type Output = Self;

                fn add(self, rhs: $t) -> Self::Output {
                    FfmpegTimeDuration::from_micros(self.as_micros() + (rhs * MICROS_PER_SEC as $t) as i64)
                }
            }
        )*
    };
}

impl_from_to_numeric!(f64, f32, i64, i32, i16, i8, u64, u32, u16, u8, isize, usize);

impl Add for FfmpegTimeDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        FfmpegTimeDuration::from_micros(self.as_micros() + rhs.as_micros())
    }
}

impl Sub for FfmpegTimeDuration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        FfmpegTimeDuration::from_micros(self.as_micros() - rhs.as_micros())
    }
}

impl From<FfmpegTimeDuration> for String {
    fn from(value: FfmpegTimeDuration) -> Self {
        value.to_string()
    }
}

impl From<FfmpegTimeDuration> for Cow<'static, str> {
    fn from(value: FfmpegTimeDuration) -> Self {
        Cow::Owned(value.to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        assert_eq!(FfmpegTimeDuration::from_str("00:00:00.00"), Some(FfmpegTimeDuration::from_seconds(0.0)));
        assert_eq!(FfmpegTimeDuration::from_str("5"),           Some(FfmpegTimeDuration::from_seconds(5.0)));
        assert_eq!(FfmpegTimeDuration::from_str("2.5"),         Some(FfmpegTimeDuration::from_seconds(2.5)));
        assert_eq!(FfmpegTimeDuration::from_str("0.123"),       Some(FfmpegTimeDuration::from_seconds(0.123)));
        assert_eq!(FfmpegTimeDuration::from_str("1:00.0"),      Some(FfmpegTimeDuration::from_seconds(60.0)));
        assert_eq!(FfmpegTimeDuration::from_str("1:01.0"),      Some(FfmpegTimeDuration::from_seconds(61.0)));
        assert_eq!(FfmpegTimeDuration::from_str("1:01:01.123"), Some(FfmpegTimeDuration::from_seconds(3661.123)));
        assert_eq!(FfmpegTimeDuration::from_str("N/A"),         None);
    }

    #[test]
    fn test_parse_negative_value() {
        assert_eq!(FfmpegTimeDuration::from_str("-00:00:01.00"), Some(FfmpegTimeDuration::from_seconds(-1.0)));
        assert_eq!(FfmpegTimeDuration::from_str("-00:01.00"),    Some(FfmpegTimeDuration::from_seconds(-1.0)));
        assert_eq!(FfmpegTimeDuration::from_str("-01.00"),       Some(FfmpegTimeDuration::from_seconds(-1.0)));
        assert_eq!(FfmpegTimeDuration::from_str("-01"),          Some(FfmpegTimeDuration::from_seconds(-1.0)));
        assert_eq!(FfmpegTimeDuration::from_str("-1"),           Some(FfmpegTimeDuration::from_seconds(-1.0)));
        assert_eq!(FfmpegTimeDuration::from_str("-1000ms"),      Some(FfmpegTimeDuration::from_seconds(-1.0)));
    }

    #[test]
    fn test_parse_string_with_suffix() {
        assert_eq!(FfmpegTimeDuration::from_str("400ms"),  Some(FfmpegTimeDuration::from_seconds(0.4)));
        assert_eq!(FfmpegTimeDuration::from_str("3000us"), Some(FfmpegTimeDuration::from_seconds(0.003)));
    }

    #[test]
    fn test_format() {
        assert_eq!(format!("{}", FfmpegTimeDuration::from_seconds(0.0)),      "00:00:00.000");
        assert_eq!(format!("{}", FfmpegTimeDuration::from_seconds(-1.0)),     "-00:00:01.000");
        assert_eq!(format!("{}", FfmpegTimeDuration::from_seconds(3661.123)), "01:01:01.123");
        assert_eq!(format!("{}", FfmpegTimeDuration::from_seconds(0.547)),    "00:00:00.547");
    }

    #[test]
    fn test_alternative_format() {
        assert_eq!(format!("{:#}", FfmpegTimeDuration::from_seconds(0.0)),       "0us");
        assert_eq!(format!("{:#}", FfmpegTimeDuration::from_seconds(-0.000001)), "-1us");
        assert_eq!(format!("{:#}", FfmpegTimeDuration::from_seconds(3661.123)),  "3661123000us");
        assert_eq!(format!("{:#}", FfmpegTimeDuration::from_seconds(0.547)),     "547000us");
    }
}