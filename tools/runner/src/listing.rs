use crate::{
    descriptor::{DESCRIPTOR_FILE, Descriptor, DescriptorError},
    example_name::{ExampleName, ExampleNameError},
    example_selector::ExampleSelector,
};
use std::{
    env,
    fmt::Write,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// The directory every example lives under, relative to the repository root.
const EXAMPLES_DIRECTORY: &str = "examples";

/// One example on disk: where it is, what it is called, and what it declares.
#[derive(Debug, Clone)]
pub struct ExampleEntry {
    /// The directory name, such as `01-json-field-mapping`.
    pub name: ExampleName,

    /// The absolute path to the example directory.
    pub directory: PathBuf,

    /// The parsed descriptor.
    pub descriptor: Descriptor,
}

/// Finds the repository root by walking up from the working directory until an `examples` directory
/// appears, so the runner works from anywhere in the tree.
pub fn repository_root() -> Result<PathBuf, ListingError> {
    let working_directory = env::current_dir().map_err(|source| ListingError::WorkingDirectory { source })?;

    working_directory
        .ancestors()
        .find(|candidate| candidate.join(EXAMPLES_DIRECTORY).is_dir())
        .map(Path::to_path_buf)
        .ok_or(ListingError::NoRepositoryRoot { start: working_directory })
}

/// Every example in the repository, in reading order.
pub fn examples(root: &Path) -> Result<Vec<ExampleEntry>, ListingError> {
    let directory = root.join(EXAMPLES_DIRECTORY);

    let listing = match directory.read_dir() {
        Ok(listing) => listing,
        Err(source) => return Err(ListingError::UnreadableDirectory { path: directory, source }),
    };

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in listing {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(source) => return Err(ListingError::UnreadableDirectory { path: directory, source }),
        };

        if path.join(DESCRIPTOR_FILE).is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    let paths = paths;

    paths.into_iter().map(entry_at).collect()
}

/// Resolves what the user typed to one example: a directory name, a path, or just its ordinal.
pub fn resolve(root: &Path, wanted: &ExampleSelector) -> Result<ExampleEntry, ListingError> {
    let direct = Path::new(wanted.as_str());
    if direct.join(DESCRIPTOR_FILE).is_file() {
        return entry_at(direct.to_path_buf());
    }

    let ordinal = format!("{wanted}-");
    let mut matches: Vec<ExampleEntry> = examples(root)?
        .into_iter()
        .filter(|entry| entry.name.as_str() == wanted.as_str() || entry.name.as_str().starts_with(&ordinal))
        .collect();

    // The errors below outlive the borrowed selector, so each one owns a copy of what was typed.
    match matches.len() {
        1 => matches.pop().ok_or_else(|| ListingError::NoSuchExample { wanted: wanted.clone() }),
        0 => Err(ListingError::NoSuchExample { wanted: wanted.clone() }),
        _several => Err(ListingError::AmbiguousExample {
            wanted: wanted.clone(),
            candidates: matches.into_iter().map(|entry| entry.name).collect(),
        }),
    }
}

/// The names of every example, as a continuous integration matrix reads them.
///
/// Nothing is left out. An example that cannot be checked unattended is an example nothing checks,
/// and the descriptor has no way to say so.
pub fn names(entries: &[ExampleEntry]) -> Vec<&ExampleName> {
    entries.iter().map(|entry| &entry.name).collect()
}

/// The listing a reader sees: one example per line, with what it shows.
pub fn summary(entries: &[ExampleEntry]) -> String {
    entries.iter().fold(String::new(), |mut listing, entry| {
        // Writing into a String cannot fail, and the result is discarded rather than propagated.
        let _ = writeln!(listing, "{}  {}", entry.name, entry.descriptor.example.summary);
        listing
    })
}

/// Loads the example rooted at a directory.
fn entry_at(directory: PathBuf) -> Result<ExampleEntry, ListingError> {
    let descriptor = Descriptor::load(&directory)?;

    let name = match directory.file_name().and_then(|name| name.to_str()) {
        Some(name) => ExampleName::try_from(name.to_owned())?,
        None => return Err(ListingError::UnnamedDirectory { path: directory }),
    };

    Ok(ExampleEntry { name, directory, descriptor })
}

/// Why the examples could not be listed.
#[derive(Debug, Error)]
pub enum ListingError {
    /// The working directory could not be read.
    #[error("cannot determine the working directory: {source}")]
    WorkingDirectory {
        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// No ancestor of the working directory holds an `examples` directory.
    #[error("no examples directory in {start} or any directory above it")]
    NoRepositoryRoot {
        /// Where the search started.
        start: PathBuf,
    },

    /// A directory could not be listed.
    #[error("cannot read {path}: {source}")]
    UnreadableDirectory {
        /// The directory that was listed.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// A directory name is not usable text.
    #[error("{path} has no usable directory name")]
    UnnamedDirectory {
        /// The directory in question.
        path: PathBuf,
    },

    /// A directory sits among the examples under a name no example may carry.
    #[error(transparent)]
    Name(#[from] ExampleNameError),

    /// Nothing matched what the user typed.
    #[error("no example matches {wanted}")]
    NoSuchExample {
        /// What the user typed.
        wanted: ExampleSelector,
    },

    /// Several examples matched what the user typed.
    #[error("{wanted} matches several examples: {}", candidates.iter().map(ExampleName::as_str).collect::<Vec<&str>>().join(", "))]
    AmbiguousExample {
        /// What the user typed.
        wanted: ExampleSelector,
        /// The examples it could have meant.
        candidates: Vec<ExampleName>,
    },

    /// The matched example's descriptor could not be read.
    #[error(transparent)]
    Descriptor(#[from] DescriptorError),
}

#[cfg(test)]
mod tests {
    use crate::{
        example_selector::ExampleSelector,
        listing::{ListingError, examples, names, repository_root, resolve, summary},
    };
    use std::str::FromStr;

    fn selector(wanted: &str) -> Option<ExampleSelector> {
        ExampleSelector::from_str(wanted).ok()
    }

    #[test]
    fn the_repository_root_holds_the_examples_directory() {
        let root = repository_root();

        assert!(root.is_ok_and(|root| root.join("examples").is_dir()));
    }

    #[test]
    fn every_example_in_the_repository_is_listed_in_reading_order() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| {
            let ordered: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
            let mut sorted: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
            sorted.sort_unstable();

            !ordered.is_empty() && ordered == sorted
        }));
    }

    #[test]
    fn an_ordinal_resolves_to_the_example_it_prefixes() {
        let entry = repository_root().and_then(|root| match selector("01") {
            Some(wanted) => resolve(&root, &wanted),
            None => Err(ListingError::NoRepositoryRoot { start: root }),
        });

        assert!(entry.is_ok_and(|entry| entry.name.as_str().starts_with("01-")));
    }

    #[test]
    fn an_ordinal_that_matches_nothing_is_reported() {
        let entry = repository_root().and_then(|root| match selector("99") {
            Some(wanted) => resolve(&root, &wanted),
            None => Err(ListingError::NoRepositoryRoot { start: root }),
        });

        assert!(matches!(entry, Err(ListingError::NoSuchExample { .. })));
    }

    #[test]
    fn every_example_is_in_the_continuous_integration_matrix() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| {
            let matrix = names(&entries);

            !matrix.is_empty() && matrix.len() == entries.len()
        }));
    }

    #[test]
    fn the_readable_listing_carries_one_line_for_each_example() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| summary(&entries).lines().count() == entries.len()));
    }
}
