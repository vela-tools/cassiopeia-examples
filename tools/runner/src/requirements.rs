use serde::Deserialize;

/// What an example needs from its environment beyond Cassiopeia itself.
///
/// A broker is not here: it is not a yes-or-no question, because the runner also has to know which
/// compose file starts it and where it answers, so it has a `[context-broker]` block of its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct Requirements {
    /// The run validates against a published Smart Data Model, so it needs the schema catalog.
    pub smart_data_models: bool,
}

#[cfg(test)]
mod tests {
    use crate::requirements::Requirements;

    #[test]
    fn an_absent_block_needs_nothing() {
        assert_eq!(Requirements::default(), Requirements { smart_data_models: false });
    }

    #[test]
    fn a_block_is_read_with_kebab_case_keys() {
        let requirements: Result<Requirements, _> = toml::from_str("smart-data-models = true\n");

        assert_eq!(requirements.map(|requirements| requirements.smart_data_models).ok(), Some(true));
    }

    #[test]
    fn an_empty_block_defaults_its_keys() {
        let requirements: Result<Requirements, _> = toml::from_str("");

        assert_eq!(requirements.ok(), Some(Requirements { smart_data_models: false }));
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
