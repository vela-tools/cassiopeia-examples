use derive_more::Display;
use serde::Deserialize;
use thiserror::Error;

/// The organisation, agency, or repository a dataset is published by.
///
/// Recorded per dataset because the licence a dataset carries is the publisher's, not this
/// repository's, and `NOTICE.md` has to be able to name them.
#[derive(Debug, Clone, PartialEq, Eq, Display, Deserialize)]
#[serde(try_from = "String")]
pub struct PublisherName(String);

impl TryFrom<String> for PublisherName {
    type Error = PublisherNameError;

    fn try_from(value: String) -> Result<PublisherName, PublisherNameError> {
        if value.trim().is_empty() {
            Err(PublisherNameError::Empty)
        } else {
            Ok(PublisherName(value))
        }
    }
}

/// Why a dataset's publisher could not be read.
#[derive(Debug, Error)]
pub enum PublisherNameError {
    /// A dataset has to name who published it.
    #[error("a dataset's publisher cannot be blank")]
    Empty,
}

#[cfg(test)]
mod tests {
    use crate::publisher_name::{PublisherName, PublisherNameError};
    use serde::Deserialize;

    /// The shape a dataset block reads its publisher in.
    #[derive(Debug, Deserialize)]
    struct Dataset {
        source: PublisherName,
    }

    #[test]
    fn an_organisation_is_accepted() {
        let publisher = PublisherName::try_from("Slovenian Environment Agency (ARSO)".to_owned());

        assert_eq!(
            publisher.map(|publisher| publisher.to_string()).ok(),
            Some("Slovenian Environment Agency (ARSO)".to_owned())
        );
    }

    #[test]
    fn a_repository_is_accepted() {
        assert!(PublisherName::try_from("andrejewski/periodic-table".to_owned()).is_ok());
    }

    #[test]
    fn a_blank_publisher_is_rejected() {
        assert!(matches!(PublisherName::try_from("   ".to_owned()), Err(PublisherNameError::Empty)));
    }

    #[test]
    fn a_publisher_is_read_from_a_descriptor_field() {
        let dataset: Result<Dataset, _> = toml::from_str("source = \"Eurostat GISCO\"");

        assert_eq!(dataset.map(|dataset| dataset.source.to_string()).ok(), Some("Eurostat GISCO".to_owned()));
    }
}
