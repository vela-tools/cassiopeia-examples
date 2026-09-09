use derive_more::Display;
use serde::Deserialize;
use thiserror::Error;

/// The terms a dataset is published under, written as an SPDX identifier where one exists and as
/// the publisher's own wording where none does.
///
/// Recorded per dataset because the datasets are fetched from their publishers and are not covered
/// by this repository's own licence.
#[derive(Debug, Clone, PartialEq, Eq, Display, Deserialize)]
#[serde(try_from = "String")]
pub struct LicenseIdentifier(String);

impl TryFrom<String> for LicenseIdentifier {
    type Error = LicenseIdentifierError;

    fn try_from(value: String) -> Result<LicenseIdentifier, LicenseIdentifierError> {
        if value.trim().is_empty() {
            Err(LicenseIdentifierError::Empty)
        } else {
            Ok(LicenseIdentifier(value))
        }
    }
}

/// Why a dataset's terms could not be read.
#[derive(Debug, Error)]
pub enum LicenseIdentifierError {
    /// A dataset whose terms are unknown says so with `Unspecified`, never with nothing.
    #[error("a dataset's licence cannot be blank: state the terms, or Unspecified")]
    Empty,
}

#[cfg(test)]
mod tests {
    use crate::license_identifier::{LicenseIdentifier, LicenseIdentifierError};

    #[test]
    fn an_spdx_identifier_is_accepted() {
        assert_eq!(
            LicenseIdentifier::try_from("CC-BY-4.0".to_owned()).map(|license| license.to_string()).ok(),
            Some("CC-BY-4.0".to_owned())
        );
    }

    #[test]
    fn a_publishers_own_wording_is_accepted() {
        assert!(LicenseIdentifier::try_from("Free reuse with attribution to ARSO".to_owned()).is_ok());
    }

    #[test]
    fn blank_terms_are_rejected() {
        assert!(matches!(LicenseIdentifier::try_from(" ".to_owned()), Err(LicenseIdentifierError::Empty)));
    }
}
