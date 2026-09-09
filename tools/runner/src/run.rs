use crate::command_line_token::CommandLineToken;
use serde::Deserialize;

/// The Cassiopeia invocation an example documents.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Run {
    /// Arguments passed to `cassiopeia`, starting with the subcommand. Paths are relative to the
    /// example directory, because the container runtime mounts that directory and nothing above it.
    pub args: Vec<CommandLineToken>,
}

#[cfg(test)]
mod tests {
    use crate::run::Run;

    #[test]
    fn arguments_are_read_as_command_line_tokens() {
        let run: Result<Run, _> = toml::from_str("args = [\"map\", \"--input\", \"data/element.json\"]\n");

        assert_eq!(
            run.map(|run| run.args.iter().map(|token| token.as_str().to_owned()).collect::<Vec<String>>())
                .ok(),
            Some(vec!["map".to_owned(), "--input".to_owned(), "data/element.json".to_owned()])
        );
    }

    #[test]
    fn an_empty_argument_is_rejected() {
        let run: Result<Run, _> = toml::from_str("args = [\"map\", \"\"]\n");

        assert!(run.is_err());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        let run: Result<Run, _> = toml::from_str("args = [\"map\"]\nenv = \"production\"\n");

        assert!(run.is_err());
    }
}
