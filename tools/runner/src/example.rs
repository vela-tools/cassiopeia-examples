use crate::{data_model_name::DataModelName, source_format::SourceFormat};
use serde::Deserialize;

/// The `[example]` block: what an example teaches, and where it is placed.
///
/// These are the keys the documentation site and the README index read. They say nothing about how
/// the example runs, which is why they sit apart from the blocks that do.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Example {
    /// The page title, matching the H1 in `example.md`.
    pub title: String,

    /// Two or three words for the documentation sidebar.
    pub label: String,

    /// One sentence for the README index, phrased as what the example shows.
    pub summary: String,

    /// The source formats the example ingests.
    pub formats: Vec<SourceFormat>,

    /// The NGSI-LD models the run produces.
    pub models: Vec<DataModelName>,
}

#[cfg(test)]
mod tests {
    use crate::{example::Example, source_format::SourceFormat};

    const BLOCK: &str = r#"
title = "Mapping JSON fields into entities"
label = "JSON field mapping"
summary = "Map a JSON array to ChemicalElement entities."
formats = ["json"]
models = ["ChemicalElement"]
"#;

    #[test]
    fn a_block_is_read_into_its_typed_parts() {
        let example: Result<Example, _> = toml::from_str(BLOCK);

        assert_eq!(
            example
                .map(|example| (example.title, example.label, example.formats, example.models.len()))
                .ok(),
            Some((
                "Mapping JSON fields into entities".to_owned(),
                "JSON field mapping".to_owned(),
                vec![SourceFormat::Json],
                1
            ))
        );
    }

    #[test]
    fn no_example_can_declare_itself_out_of_continuous_integration() {
        let text = "title = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\nci = \"skip\"\n";
        let example: Result<Example, _> = toml::from_str(text);

        assert!(example.is_err());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        let text = "title = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\nauthor = \"someone\"\n";
        let example: Result<Example, _> = toml::from_str(text);

        assert!(example.is_err());
    }

    #[test]
    fn a_block_missing_its_title_is_rejected() {
        let text = "label = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\n";
        let example: Result<Example, _> = toml::from_str(text);

        assert!(example.is_err());
    }
}
