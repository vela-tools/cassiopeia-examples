use derive_more::Display;
use serde::Serialize;
use thiserror::Error;

/// The directory name an example lives under, such as `01-json-field-mapping`.
///
/// The leading ordinal fixes the reading order the README index and the documentation sidebar are
/// built in, so a directory without one is not an example this runner can place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Serialize)]
pub struct ExampleName(String);

impl ExampleName {
    /// The name as it appears on disk.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ExampleName {
    type Error = ExampleNameError;

    fn try_from(value: String) -> Result<ExampleName, ExampleNameError> {
        let segments: Vec<&str> = value.split('-').collect();
        let kebab_case = segments
            .iter()
            .all(|segment| !segment.is_empty() && segment.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()));

        if !kebab_case {
            return Err(ExampleNameError::NotKebabCase { value });
        }

        let ordinal = segments
            .first()
            .is_some_and(|segment| segment.chars().all(|character| character.is_ascii_digit()));
        if !ordinal || segments.len() < 2 {
            return Err(ExampleNameError::NoOrdinal { value });
        }

        Ok(ExampleName(value))
    }
}

/// Why a directory name could not be read as an example name.
#[derive(Debug, Error)]
pub enum ExampleNameError {
    /// The name carries something other than lowercase letters, digits, and single hyphens.
    #[error("{value} is not a kebab-case example directory name")]
    NotKebabCase {
        /// The name as it appears on disk.
        value: String,
    },

    /// The name does not open with the ordinal that places the example.
    #[error("{value} does not start with an ordinal and a name, such as 01-json-field-mapping")]
    NoOrdinal {
        /// The name as it appears on disk.
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use crate::example_name::{ExampleName, ExampleNameError};

    #[test]
    fn an_ordinal_and_a_kebab_case_name_is_accepted() {
        let name = ExampleName::try_from("01-json-field-mapping".to_owned());

        assert_eq!(name.map(|name| name.to_string()).ok(), Some("01-json-field-mapping".to_owned()));
    }

    #[test]
    fn a_name_without_an_ordinal_is_rejected() {
        let name = ExampleName::try_from("json-field-mapping".to_owned());

        assert!(matches!(name, Err(ExampleNameError::NoOrdinal { .. })));
    }

    #[test]
    fn an_ordinal_on_its_own_is_rejected() {
        let name = ExampleName::try_from("01".to_owned());

        assert!(matches!(name, Err(ExampleNameError::NoOrdinal { .. })));
    }

    #[test]
    fn an_uppercase_name_is_rejected() {
        let name = ExampleName::try_from("01-JsonFieldMapping".to_owned());

        assert!(matches!(name, Err(ExampleNameError::NotKebabCase { .. })));
    }

    #[test]
    fn a_doubled_hyphen_is_rejected() {
        let name = ExampleName::try_from("01--json".to_owned());

        assert!(matches!(name, Err(ExampleNameError::NotKebabCase { .. })));
    }

    #[test]
    fn names_order_by_their_ordinal() {
        let mut names = [
            ExampleName::try_from("10-json-relationships".to_owned()).ok(),
            ExampleName::try_from("02-json-id-collision".to_owned()).ok(),
        ];
        names.sort();

        assert_eq!(
            names.first().and_then(|name| name.as_ref()).map(ExampleName::as_str),
            Some("02-json-id-collision")
        );
    }
}
