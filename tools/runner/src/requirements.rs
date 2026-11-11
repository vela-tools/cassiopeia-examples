use crate::environment_variable_name::EnvironmentVariableName;
use serde::Deserialize;

/// What an example needs from its environment beyond Cassiopeia itself.
///
/// A broker is not here: it is not a yes-or-no question, because the runner also has to know which
/// compose file starts it and where it answers, so it has a `[context-broker]` block of its own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct Requirements {
    /// The run validates against a published Smart Data Model, so it needs the schema catalog.
    pub smart_data_models: bool,

    /// The environment variables the run reads, such as an API key its manifest sends as a request
    /// header. A native run inherits them; a container run is handed each one by name, so its value
    /// never appears on a command line or on the page.
    pub environment: Vec<EnvironmentVariableName>,
}

#[cfg(test)]
mod tests {
    use crate::requirements::Requirements;

    #[test]
    fn an_absent_block_needs_nothing() {
        let requirements = Requirements::default();

        assert!(!requirements.smart_data_models);
        assert_eq!(requirements.environment.len(), 0);
    }

    #[test]
    fn a_block_is_read_with_kebab_case_keys() {
        let requirements: Result<Requirements, _> = toml::from_str("smart-data-models = true\n");

        assert_eq!(requirements.map(|requirements| requirements.smart_data_models).ok(), Some(true));
    }

    #[test]
    fn an_empty_block_defaults_its_keys() {
        let requirements: Result<Requirements, _> = toml::from_str("");

        assert_eq!(requirements.ok(), Some(Requirements::default()));
    }

    #[test]
    fn the_environment_variables_a_run_reads_are_listed_by_name() {
        let requirements: Result<Requirements, _> = toml::from_str("environment = [\"BRUSSELS_ODS_APIKEY\"]\n");

        assert_eq!(
            requirements
                .map(|requirements| requirements.environment.iter().map(|name| name.as_str().to_owned()).collect::<Vec<String>>())
                .ok(),
            Some(vec!["BRUSSELS_ODS_APIKEY".to_owned()])
        );
    }

    #[test]
    fn an_environment_variable_name_a_manifest_could_not_read_is_rejected() {
        let requirements: Result<Requirements, _> = toml::from_str("environment = [\"API-KEY\"]\n");

        assert!(requirements.is_err());
    }

    #[test]
    fn an_unknown_requirement_is_rejected() {
        let requirements: Result<Requirements, _> = toml::from_str("gdal = true\n");

        assert!(requirements.is_err());
    }

    #[test]
    fn a_broker_is_not_a_requirement_flag() {
        let requirements: Result<Requirements, _> = toml::from_str("context-broker = true\n");

        assert!(requirements.is_err());
    }
}
