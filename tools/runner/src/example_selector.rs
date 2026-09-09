use derive_more::{Display, FromStr};

/// What the user typed to pick an example: its ordinal, its directory name, or a path to it.
///
/// The three forms are resolved against the repository rather than parsed apart here, because an
/// ordinal only means something next to the directories it could match.
#[derive(Debug, Clone, PartialEq, Eq, Display, FromStr)]
pub struct ExampleSelector(String);

impl ExampleSelector {
    /// The selector as the user wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::example_selector::ExampleSelector;
    use std::str::FromStr;

    #[test]
    fn an_ordinal_is_kept_as_written() {
        let selector = ExampleSelector::from_str("01");

        assert_eq!(selector.map(|selector| selector.as_str().to_owned()).ok(), Some("01".to_owned()));
    }

    #[test]
    fn a_path_is_kept_as_written() {
        let selector = ExampleSelector::from_str("examples/01-json-field-mapping");

        assert_eq!(
            selector.map(|selector| selector.as_str().to_owned()).ok(),
            Some("examples/01-json-field-mapping".to_owned())
        );
    }
}
