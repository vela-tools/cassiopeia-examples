use crate::shell_command::ShellCommand;
use serde::Deserialize;

/// A shell step that turns a downloaded file into something Cassiopeia can read.
///
/// Kept in the descriptor rather than in a script beside the mapping so that the page, the runner,
/// and continuous integration all name the same step.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Preparation {
    /// What the step does, printed while it runs.
    pub description: String,

    /// The command, run by the shell in the example directory.
    pub command: ShellCommand,
}

#[cfg(test)]
mod tests {
    use crate::preparation::Preparation;

    #[test]
    fn a_step_carries_its_description_and_its_command() {
        let step: Result<Preparation, _> =
            toml::from_str("description = \"packaging the KML as a KMZ archive\"\ncommand = \"cd data && zip -q poi.kmz poi.kml\"\n");

        assert_eq!(
            step.map(|step| (step.description, step.command.as_str().to_owned())).ok(),
            Some(("packaging the KML as a KMZ archive".to_owned(), "cd data && zip -q poi.kmz poi.kml".to_owned()))
        );
    }

    #[test]
    fn a_step_with_a_blank_command_is_rejected() {
        let step: Result<Preparation, _> = toml::from_str("description = \"doing nothing\"\ncommand = \"\"\n");

        assert!(step.is_err());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        let step: Result<Preparation, _> = toml::from_str("description = \"x\"\ncommand = \"true\"\nshell = \"zsh\"\n");

        assert!(step.is_err());
    }
}
