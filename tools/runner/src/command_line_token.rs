use derive_more::Display;
use serde::Deserialize;
use thiserror::Error;

/// One token of a Cassiopeia command line: a subcommand, a long flag, or a flag's value.
///
/// Descriptors declare the invocation token by token rather than as one string, because the runner
/// executes those tokens directly and the page renders the same tokens as text.
#[derive(Debug, Clone, PartialEq, Eq, Display, Deserialize)]
#[serde(try_from = "String")]
pub struct CommandLineToken(String);

impl CommandLineToken {
    /// The token as it reaches the program.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// What the token does to the line it is rendered on.
    pub fn role(&self) -> TokenRole {
        if self.0.starts_with("--") { TokenRole::Flag } else { TokenRole::Value }
    }
}

impl TryFrom<String> for CommandLineToken {
    type Error = CommandLineTokenError;

    fn try_from(value: String) -> Result<CommandLineToken, CommandLineTokenError> {
        if value.is_empty() {
            Err(CommandLineTokenError::Empty)
        } else {
            Ok(CommandLineToken(value))
        }
    }
}

/// The part a token plays in a rendered command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRole {
    /// A long flag, which opens a line of its own.
    Flag,

    /// A subcommand or a flag's value, which joins the flag before it or stands alone.
    Value,
}

/// Why a token could not be read.
#[derive(Debug, Error)]
pub enum CommandLineTokenError {
    /// An empty argument reaches the program as an empty string, which no Cassiopeia flag accepts.
    #[error("a command-line token cannot be empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use crate::command_line_token::{CommandLineToken, CommandLineTokenError, TokenRole};

    #[test]
    fn a_long_flag_opens_a_line() {
        let token = CommandLineToken::try_from("--input".to_owned());

        assert_eq!(token.map(|token| token.role()).ok(), Some(TokenRole::Flag));
    }

    #[test]
    fn a_subcommand_is_a_value() {
        let token = CommandLineToken::try_from("map".to_owned());

        assert_eq!(token.map(|token| token.role()).ok(), Some(TokenRole::Value));
    }

    #[test]
    fn a_path_that_starts_with_a_dash_is_still_a_value() {
        let token = CommandLineToken::try_from("-v".to_owned());

        assert_eq!(token.map(|token| token.role()).ok(), Some(TokenRole::Value));
    }

    #[test]
    fn an_empty_token_is_rejected() {
        assert!(matches!(CommandLineToken::try_from(String::new()), Err(CommandLineTokenError::Empty)));
    }
}
