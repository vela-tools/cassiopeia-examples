use derive_more::Display;
use serde::Deserialize;
use thiserror::Error;

/// The name of an environment variable a run reads, such as the API key a manifest sends as a
/// request header.
///
/// The rule is the one Cassiopeia applies to a `${NAME}` placeholder in a manifest: ASCII letters,
/// digits, and underscores, not starting with a digit. A name the manifest could never read is
/// refused when the descriptor loads rather than passed to a run that cannot use it.
#[derive(Debug, Clone, PartialEq, Eq, Display, Deserialize)]
#[serde(try_from = "String")]
pub struct EnvironmentVariableName(String);

impl EnvironmentVariableName {
    /// The name as the shell and the container engine read it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EnvironmentVariableName {
    type Error = EnvironmentVariableNameError;

    fn try_from(value: String) -> Result<EnvironmentVariableName, EnvironmentVariableNameError> {
        let mut characters = value.chars();

        match characters.next() {
            None => Err(EnvironmentVariableNameError::Empty),
            Some(first) if first.is_ascii_digit() => Err(EnvironmentVariableNameError::LeadingDigit { name: value }),
            Some(_) if !value.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') => {
                Err(EnvironmentVariableNameError::IllegalCharacter { name: value })
            }
            Some(_) => Ok(EnvironmentVariableName(value)),
        }
    }
}

/// Why an environment variable name was refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvironmentVariableNameError {
    /// An empty name names no variable.
    #[error("an environment variable name cannot be empty")]
    Empty,

    /// A shell reads a name that starts with a digit as a positional parameter.
    #[error("the environment variable name `{name}` starts with a digit")]
    LeadingDigit {
        /// The refused name.
        name: String,
    },

    /// A manifest placeholder ends at the first character outside letters, digits, and underscores.
    #[error("the environment variable name `{name}` holds a character other than a letter, a digit, or an underscore")]
    IllegalCharacter {
        /// The refused name.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use crate::environment_variable_name::{EnvironmentVariableName, EnvironmentVariableNameError};

    #[test]
    fn a_name_of_capitals_digits_and_underscores_is_accepted() {
        let name = EnvironmentVariableName::try_from("BRUSSELS_ODS_APIKEY".to_owned());

        assert_eq!(name.map(|name| name.as_str().to_owned()), Ok("BRUSSELS_ODS_APIKEY".to_owned()));
    }

    #[test]
    fn a_name_starting_with_an_underscore_is_accepted() {
        assert!(EnvironmentVariableName::try_from("_TOKEN2".to_owned()).is_ok());
    }

    #[test]
    fn an_empty_name_is_refused() {
        assert_eq!(EnvironmentVariableName::try_from(String::new()), Err(EnvironmentVariableNameError::Empty));
    }

    #[test]
    fn a_name_starting_with_a_digit_is_refused() {
        assert_eq!(
            EnvironmentVariableName::try_from("1KEY".to_owned()),
            Err(EnvironmentVariableNameError::LeadingDigit { name: "1KEY".to_owned() })
        );
    }

    #[test]
    fn a_name_with_a_hyphen_is_refused() {
        assert_eq!(
            EnvironmentVariableName::try_from("API-KEY".to_owned()),
            Err(EnvironmentVariableNameError::IllegalCharacter { name: "API-KEY".to_owned() })
        );
    }

    #[test]
    fn a_name_written_as_a_placeholder_is_refused() {
        assert_eq!(
            EnvironmentVariableName::try_from("${KEY}".to_owned()),
            Err(EnvironmentVariableNameError::IllegalCharacter { name: "${KEY}".to_owned() })
        );
    }
}
