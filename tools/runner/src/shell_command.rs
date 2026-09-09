use derive_more::Display;
use serde::Deserialize;
use thiserror::Error;

/// A command the shell runs in an example directory.
///
/// Preparation steps and the datasets a plain GET cannot fetch are both written this way, so that
/// the page, the runner, and continuous integration all name the same command.
#[derive(Debug, Clone, PartialEq, Eq, Display, Deserialize)]
#[serde(try_from = "String")]
pub struct ShellCommand(String);

impl ShellCommand {
    /// The command as the shell receives it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ShellCommand {
    type Error = ShellCommandError;

    fn try_from(value: String) -> Result<ShellCommand, ShellCommandError> {
        if value.trim().is_empty() {
            Err(ShellCommandError::Empty)
        } else {
            Ok(ShellCommand(value))
        }
    }
}

/// Why a shell command could not be read.
#[derive(Debug, Error)]
pub enum ShellCommandError {
    /// A step that runs nothing is a step that should not be declared.
    #[error("a shell command cannot be blank")]
    Empty,
}

#[cfg(test)]
mod tests {
    use crate::shell_command::{ShellCommand, ShellCommandError};

    #[test]
    fn a_pipeline_is_kept_verbatim() {
        let command = ShellCommand::try_from("awk -f flatten.awk data/hurdat2.txt > data/cyclones.csv".to_owned());

        assert_eq!(
            command.map(|command| command.as_str().to_owned()).ok(),
            Some("awk -f flatten.awk data/hurdat2.txt > data/cyclones.csv".to_owned())
        );
    }

    #[test]
    fn a_multi_line_command_is_kept_verbatim() {
        let command = ShellCommand::try_from("curl --silent \\\n  --output data/movies.json \\\n  https://example.invalid/movies".to_owned());

        assert!(command.is_ok_and(|command| command.as_str().lines().count() == 3));
    }

    #[test]
    fn a_blank_command_is_rejected() {
        assert!(matches!(ShellCommand::try_from("  \n ".to_owned()), Err(ShellCommandError::Empty)));
    }
}
