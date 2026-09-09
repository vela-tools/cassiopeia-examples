use crate::{
    container_network::ContainerNetwork,
    context_broker::ContextBroker,
    dataset::Dataset,
    example::Example,
    output::Output,
    poll_schedule::PollSchedule,
    preparation::Preparation,
    requirements::Requirements,
    run::Run,
};
use serde::Deserialize;
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use toml::de::Error as TomlError;

/// The file every example directory declares itself in.
pub const DESCRIPTOR_FILE: &str = "example.toml";

/// Everything an example declares about itself: what it teaches, what it reads, what it runs, and
/// what the run has to produce.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Descriptor {
    /// What the example teaches, and where the documentation site places it.
    pub example: Example,

    /// What the run needs beyond Cassiopeia itself.
    #[serde(default)]
    pub requirements: Requirements,

    /// The broker the run delivers to, for an example that writes to one instead of to files.
    #[serde(default)]
    pub context_broker: Option<ContextBroker>,

    /// The bound a scheduled run is given, for an example whose mapping repeats until it is
    /// interrupted.
    #[serde(default)]
    pub schedule: Option<PollSchedule>,

    /// Shell steps the dataset needs before Cassiopeia can read it, such as unzipping an archive
    /// or flattening an irregular source into a plain CSV.
    #[serde(default)]
    pub preparation: Vec<Preparation>,

    /// The Cassiopeia invocation, which both the runner and the page are derived from.
    pub run: Run,

    /// The files the example downloads before it runs.
    #[serde(default)]
    pub datasets: Vec<Dataset>,

    /// The files the run writes, and what has to be in them.
    #[serde(default)]
    pub outputs: Vec<Output>,
}

impl Descriptor {
    /// The network a container run of this example needs.
    ///
    /// A run that delivers to a broker published on a host port only reaches it when the container
    /// shares the host's network, because the URL its manifest names is `localhost`.
    pub const fn network(&self) -> ContainerNetwork {
        if self.context_broker.is_some() {
            ContainerNetwork::Host
        } else {
            ContainerNetwork::Isolated
        }
    }

    /// Reads the descriptor out of an example directory.
    pub fn load(directory: &Path) -> Result<Descriptor, DescriptorError> {
        let path = directory.join(DESCRIPTOR_FILE);

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) => return Err(DescriptorError::Unreadable { path, source }),
        };

        toml::from_str(&text).map_err(|source| DescriptorError::Malformed { path, source })
    }
}

/// Why an example's descriptor could not be read.
#[derive(Debug, Error)]
pub enum DescriptorError {
    /// The file is missing or unreadable.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The descriptor that was looked for.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The file is not a valid descriptor.
    #[error("{path} is not a valid example descriptor: {source}")]
    Malformed {
        /// The descriptor that was parsed.
        path: PathBuf,
        /// The parse failure, with its position in the file.
        source: TomlError,
    },
}

#[cfg(test)]
mod tests {
    use crate::{descriptor::Descriptor, entity_count::EntityCount, source_format::SourceFormat};

    const DESCRIPTOR: &str = r#"
[example]
title = "Mapping JSON fields into entities"
label = "JSON field mapping"
summary = "Map a JSON array to ChemicalElement entities."
formats = ["json"]
models = ["ChemicalElement"]

[requirements]
smart-data-models = false

[run]
args = ["map", "--input", "data/element.json"]

[[datasets]]
file = "data/element.json"
url = "https://example.invalid/data.json"
source = "andrejewski/periodic-table"
license = "ISC"
volatility = "frozen"
records = 118

[[outputs]]
file = "out/ChemicalElement.json"
entities = 118
contains = ["urn:ngsi-ld:ChemicalElement:H"]
"#;

    #[test]
    fn a_descriptor_is_read_into_its_typed_parts() {
        let descriptor: Result<Descriptor, _> = toml::from_str(DESCRIPTOR);
        let parts = descriptor.map(|descriptor| {
            (
                descriptor.example.formats,
                descriptor.example.models.len(),
                descriptor.run.args.len(),
                descriptor.outputs.first().map(|output| output.count),
                descriptor.datasets.first().and_then(|dataset| dataset.records),
            )
        });

        assert_eq!(parts.ok(), Some((vec![SourceFormat::Json], 1, 3, Some(EntityCount::Exactly(118)), Some(118))));
    }

    #[test]
    fn a_descriptor_without_the_optional_blocks_is_read() {
        let text = "[example]\ntitle = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\n\n[run]\nargs = [\"map\"]\n";
        let descriptor: Result<Descriptor, _> = toml::from_str(text);

        assert_eq!(
            descriptor
                .map(|descriptor| (descriptor.datasets.len(), descriptor.outputs.len(), descriptor.preparation.len()))
                .ok(),
            Some((0, 0, 0))
        );
    }

    #[test]
    fn an_unknown_block_is_rejected() {
        let text = "[example]\ntitle = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\n\n[author]\nname = \"someone\"\n\n[run]\nargs = [\"map\"]\n";
        let descriptor: Result<Descriptor, _> = toml::from_str(text);

        assert!(descriptor.is_err());
    }

    #[test]
    fn a_descriptor_declaring_its_keys_at_the_root_is_rejected() {
        let text = "title = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\n\n[run]\nargs = [\"map\"]\n";
        let descriptor: Result<Descriptor, _> = toml::from_str(text);

        assert!(descriptor.is_err());
    }

    #[test]
    fn a_descriptor_missing_its_example_block_is_rejected() {
        let text = "[run]\nargs = [\"map\"]\n";
        let descriptor: Result<Descriptor, _> = toml::from_str(text);

        assert!(descriptor.is_err());
    }

    #[test]
    fn a_descriptor_missing_its_run_is_rejected() {
        let text = "[example]\ntitle = \"t\"\nlabel = \"l\"\nsummary = \"s\"\nformats = [\"csv\"]\nmodels = [\"City\"]\n";
        let descriptor: Result<Descriptor, _> = toml::from_str(text);

        assert!(descriptor.is_err());
    }
}
