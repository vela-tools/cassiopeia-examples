use derive_more::Display;
use humantime::{DurationError, format_duration, parse_duration};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

/// A span of time a descriptor states in the same words a manifest's schedule states it, such as
/// `5m` or `20s`.
///
/// A newtype rather than a bare number of seconds so that a descriptor and the manifest beside it
/// cannot disagree about the unit, and so that the runner never has to be told which one was meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Deserialize)]
#[display("{}", format_duration(*_0))]
#[serde(try_from = "String")]
pub struct PollInterval(Duration);

impl PollInterval {
    /// The span as the standard library measures it.
    pub const fn as_duration(self) -> Duration {
        self.0
    }
}

impl TryFrom<String> for PollInterval {
    type Error = PollIntervalError;

    fn try_from(value: String) -> Result<PollInterval, PollIntervalError> {
        match parse_duration(&value) {
            Ok(duration) => Ok(PollInterval(duration)),
            // The failure outlives the borrowed text, so it owns what was written.
            Err(source) => Err(PollIntervalError::Unparsable { value, source }),
        }
    }
}

/// Why a span of time could not be read.
#[derive(Debug, Error)]
pub enum PollIntervalError {
    /// The value is not a duration.
    #[error("{value} is not a duration such as 5m or 20s: {source}")]
    Unparsable {
        /// The value as it was written.
        value: String,

        /// What the parser objected to.
        source: DurationError,
    },
}

#[cfg(test)]
mod tests {
    use crate::poll_interval::{PollInterval, PollIntervalError};
    use std::time::Duration;

    fn interval(value: &str) -> Result<PollInterval, PollIntervalError> {
        PollInterval::try_from(value.to_owned())
    }

    #[test]
    fn minutes_and_seconds_are_read_the_way_a_manifest_writes_them() {
        assert_eq!(interval("5m").map(PollInterval::as_duration).ok(), Some(Duration::from_secs(300)));
        assert_eq!(interval("20s").map(PollInterval::as_duration).ok(), Some(Duration::from_secs(20)));
    }

    #[test]
    fn a_compound_span_is_read_as_its_total() {
        assert_eq!(interval("1h 30m").map(PollInterval::as_duration).ok(), Some(Duration::from_mins(90)));
    }

    #[test]
    fn a_span_prints_the_way_it_was_written() {
        assert_eq!(interval("5m").map(|interval| interval.to_string()).ok(), Some("5m".to_owned()));
    }

    #[test]
    fn a_bare_number_is_rejected() {
        assert!(matches!(interval("300"), Err(PollIntervalError::Unparsable { .. })));
    }

    #[test]
    fn a_span_in_words_is_rejected() {
        assert!(matches!(interval("five minutes"), Err(PollIntervalError::Unparsable { .. })));
    }
}
