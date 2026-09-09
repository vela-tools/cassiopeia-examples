use crate::{assertion_target::AssertionTarget, data_model_name::DataModelName, entity_count::EntityCount, entity_id::EntityId};
use serde::Deserialize;
use std::path::PathBuf;
use thiserror::Error;

/// One thing a run produces, and the assertions that prove the run did what its page says.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "OutputBlock")]
pub struct Output {
    /// Where the entities are looked for.
    pub target: AssertionTarget,

    /// How many of them there have to be.
    pub count: EntityCount,

    /// Identifiers that have to be present, chosen for the entities the page quotes.
    pub contains: Vec<EntityId>,
}

/// An `[[outputs]]` block as it is written in a descriptor.
///
/// Deserialised into [`Output`] rather than used directly, because a block names one target out of
/// three and states its count as one of two keys, and only the conversion can hold either rule.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct OutputBlock {
    /// A file the run writes, relative to the example directory.
    #[serde(default)]
    file: Option<PathBuf>,

    /// A model whose entities the broker has to hold after the run.
    #[serde(default)]
    broker_entities: Option<DataModelName>,

    /// One folded `EntityTemporal` the broker has to hold after the run.
    #[serde(default)]
    broker_temporal: Option<EntityId>,

    /// Exactly how many entities the target holds.
    #[serde(default)]
    entities: Option<usize>,

    /// The fewest entities it may hold.
    #[serde(default)]
    at_least: Option<usize>,

    /// Identifiers that have to be present.
    #[serde(default)]
    contains: Vec<EntityId>,
}

impl TryFrom<OutputBlock> for Output {
    type Error = OutputError;

    fn try_from(block: OutputBlock) -> Result<Output, OutputError> {
        let target = match (block.file, block.broker_entities, block.broker_temporal) {
            (Some(file), None, None) => AssertionTarget::File(file),
            (None, Some(model), None) => AssertionTarget::BrokerEntities(model),
            (None, None, Some(id)) => AssertionTarget::BrokerTemporal(id),
            (None, None, None) => return Err(OutputError::NoTarget),
            (Some(_), Some(_), None | Some(_)) | (Some(_), None, Some(_)) | (None, Some(_), Some(_)) => return Err(OutputError::SeveralTargets),
        };

        let count = match (block.entities, block.at_least) {
            (Some(exact), None) => EntityCount::Exactly(exact),
            (None, Some(fewest)) => EntityCount::AtLeast(fewest),
            (Some(_), Some(_)) => return Err(OutputError::BothCounts { target }),
            (None, None) => return Err(OutputError::NoCount { target }),
        };

        Ok(Output {
            target,
            count,
            contains: block.contains,
        })
    }
}

/// Why an output block does not describe something checkable.
#[derive(Debug, Error)]
pub enum OutputError {
    /// The block says nothing about where to look.
    #[error("an [[outputs]] block names nothing to check: give it a `file`, a `broker-entities`, or a `broker-temporal`")]
    NoTarget,

    /// The block names more than one place, which leaves it unclear which one the counts are about.
    #[error("an [[outputs]] block names more than one of `file`, `broker-entities`, and `broker-temporal`: give each target a block of its own")]
    SeveralTargets,

    /// The block states neither an exact count nor a floor, so a run producing nothing would pass.
    #[error("the output {target} has no entity count to check: give it an `entities` or an `at-least`")]
    NoCount {
        /// What the block describes.
        target: AssertionTarget,
    },

    /// The block states both, which leaves it unclear which one a run has to meet.
    #[error("the output {target} states both `entities` and `at-least`: keep the one the source warrants")]
    BothCounts {
        /// What the block describes.
        target: AssertionTarget,
    },
}

#[cfg(test)]
mod tests {
    use crate::{assertion_target::AssertionTarget, entity_count::EntityCount, output::Output};
    use std::path::PathBuf;
    use toml::de::Error as TomlError;

    fn output(text: &str) -> Result<Output, TomlError> {
        toml::from_str(text)
    }

    #[test]
    fn an_exact_count_is_read_as_an_exact_count() {
        let declared = output("file = \"out/ChemicalElement.json\"\nentities = 118\n");

        assert_eq!(declared.map(|declared| declared.count).ok(), Some(EntityCount::Exactly(118)));
    }

    #[test]
    fn a_floor_is_read_as_a_floor() {
        let declared = output("file = \"out/WeatherObserved.json\"\nat-least = 500\n");

        assert_eq!(declared.map(|declared| declared.count).ok(), Some(EntityCount::AtLeast(500)));
    }

    #[test]
    fn a_file_block_is_read_as_a_file_target() {
        let declared = output("file = \"out/ChemicalElement.json\"\nentities = 118\n");

        assert_eq!(
            declared.map(|declared| declared.target).ok(),
            Some(AssertionTarget::File(PathBuf::from("out/ChemicalElement.json")))
        );
    }

    #[test]
    fn a_broker_entities_block_is_read_as_a_broker_target() {
        let declared = output("broker-entities = \"AirQualityObserved\"\nat-least = 5\n");

        assert_eq!(
            declared.map(|declared| declared.target.to_string()).ok(),
            Some("the broker's AirQualityObserved entities".to_owned())
        );
    }

    #[test]
    fn a_broker_temporal_block_is_read_as_a_temporal_target() {
        let declared = output("broker-temporal = \"urn:ngsi-ld:TropicalCyclone:AL122005\"\nentities = 1\n");

        assert_eq!(
            declared.map(|declared| declared.target.to_string()).ok(),
            Some("urn:ngsi-ld:TropicalCyclone:AL122005 at the broker's temporal endpoint".to_owned())
        );
    }

    #[test]
    fn quoted_identifiers_are_read_as_entity_identifiers() {
        let declared = output("file = \"out/ChemicalElement.json\"\nentities = 118\ncontains = [\"urn:ngsi-ld:ChemicalElement:H\"]\n");

        assert_eq!(declared.map(|declared| declared.contains.len()).ok(), Some(1));
    }

    #[test]
    fn a_block_without_a_target_is_rejected_when_it_is_read() {
        assert!(output("entities = 118\n").is_err());
    }

    #[test]
    fn a_block_naming_two_targets_is_rejected_when_it_is_read() {
        assert!(output("file = \"out/City.json\"\nbroker-entities = \"City\"\nentities = 1\n").is_err());
    }

    #[test]
    fn a_block_without_a_count_is_rejected_when_it_is_read() {
        assert!(output("file = \"out/ChemicalElement.json\"\n").is_err());
    }

    #[test]
    fn a_block_stating_both_counts_is_rejected_when_it_is_read() {
        assert!(output("file = \"out/ChemicalElement.json\"\nentities = 118\nat-least = 100\n").is_err());
    }

    #[test]
    fn an_unknown_key_is_rejected_when_it_is_read() {
        assert!(output("file = \"out/ChemicalElement.json\"\nentities = 118\nentites = 118\n").is_err());
    }
}
