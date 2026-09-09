use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The name of an NGSI-LD data model an example produces, such as `ChemicalElement`.
///
/// The name is the entity `type` and the stem of the file the run writes, so it is a domain value
/// rather than free text. NGSI-LD entity types are written in `UpperCamelCase` (ETSI GS CIM 009
/// v1.9.1, clause 4.6.2), which is also what the Smart Data Models catalog publishes them as.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct DataModelName(String);

impl DataModelName {
    /// The name as it appears in an entity's `type` member.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DataModelName {
    type Error = DataModelNameError;

    fn try_from(value: String) -> Result<DataModelName, DataModelNameError> {
        let upper_camel_case =
            value.starts_with(|character: char| character.is_ascii_uppercase()) && value.chars().all(|character| character.is_ascii_alphanumeric());

        if upper_camel_case {
            Ok(DataModelName(value))
        } else {
            Err(DataModelNameError::NotUpperCamelCase { value })
        }
    }
}

/// Why a model name could not be read.
#[derive(Debug, Error)]
pub enum DataModelNameError {
    /// The name is not the `UpperCamelCase` an NGSI-LD entity type is written in.
    #[error("{value} is not an UpperCamelCase NGSI-LD entity type, such as ChemicalElement")]
    NotUpperCamelCase {
        /// The name as it was written.
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use crate::data_model_name::{DataModelName, DataModelNameError};

    #[test]
    fn an_upper_camel_case_name_is_accepted() {
        assert_eq!(
            DataModelName::try_from("AirQualityObserved".to_owned()).map(|name| name.to_string()).ok(),
            Some("AirQualityObserved".to_owned())
        );
    }

    #[test]
    fn a_lowercase_name_is_rejected() {
        assert!(matches!(
            DataModelName::try_from("airQualityObserved".to_owned()),
            Err(DataModelNameError::NotUpperCamelCase { .. })
        ));
    }

    #[test]
    fn a_hyphenated_name_is_rejected() {
        assert!(matches!(
            DataModelName::try_from("Air-Quality".to_owned()),
            Err(DataModelNameError::NotUpperCamelCase { .. })
        ));
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(matches!(
            DataModelName::try_from(String::new()),
            Err(DataModelNameError::NotUpperCamelCase { .. })
        ));
    }
}
