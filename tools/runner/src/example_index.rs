use crate::{format_badge::FormatBadge, listing::ExampleEntry};
use std::{
    fmt::Write,
    fs,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Where the generated index starts in the README.
const START_MARKER: &str = "<!-- examples:start -->";

/// Where it ends.
const END_MARKER: &str = "<!-- examples:end -->";

/// Renders the README's index of examples.
///
/// The index is generated rather than hand-written so that an example's entry cannot say something
/// its descriptor does not. Entries keep the reading order the directory prefixes set, which is why
/// the prefix itself is left out of the link text: the order carries it.
pub fn render(entries: &[ExampleEntry]) -> String {
    let blocks: Vec<String> = entries.iter().map(block).collect();

    blocks.join("\n\n")
}

/// One example's entry: what it reads, what it is, what it produces, and what it shows.
fn block(entry: &ExampleEntry) -> String {
    // Floated badges stack from the right edge inwards, so the declared order is reversed here to
    // land them on the page in the order the descriptor writes them.
    let badges: Vec<String> = entry
        .descriptor
        .example
        .formats
        .iter()
        .rev()
        .map(|format| FormatBadge::from(*format).to_string())
        .collect();
    let models: Vec<String> = entry.descriptor.example.models.iter().map(|model| format!("`{model}`")).collect();

    let mut block = badges.join(" ");
    // Writing into a String cannot fail, and the result is discarded rather than propagated.
    let _ = write!(
        block,
        " **[{label}](examples/{name}/example.md)** → {models}\n\n{summary}",
        label = entry.descriptor.example.label,
        name = entry.name,
        models = models.join(" "),
        summary = entry.descriptor.example.summary,
    );

    block
}

/// Replaces the index between the markers in the README, leaving the rest of the file alone.
pub fn update(readme: &Path, entries: &[ExampleEntry]) -> Result<(), ExampleIndexError> {
    let text = match fs::read_to_string(readme) {
        Ok(text) => text,
        Err(source) => {
            return Err(ExampleIndexError::Unreadable {
                path: readme.to_path_buf(),
                source,
            });
        }
    };

    let region = match (text.find(START_MARKER), text.find(END_MARKER)) {
        (Some(start), Some(end)) if start < end => (start, end),
        (Some(_) | None, Some(_) | None) => return Err(ExampleIndexError::NoMarkers { path: readme.to_path_buf() }),
    };

    let (start, end) = region;
    let updated = format!("{}{START_MARKER}\n\n{}\n\n{}", &text[..start], render(entries), &text[end..]);

    fs::write(readme, updated).map_err(|source| ExampleIndexError::Unwritable {
        path: readme.to_path_buf(),
        source,
    })
}

/// Why the README index could not be regenerated.
#[derive(Debug, Error)]
pub enum ExampleIndexError {
    /// The README is missing or unreadable.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The README.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The README has no generated region.
    #[error("{path} has no {START_MARKER} and {END_MARKER} pair")]
    NoMarkers {
        /// The README.
        path: PathBuf,
    },

    /// The README could not be written back.
    #[error("cannot write {path}: {source}")]
    Unwritable {
        /// The README.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        example_index::{ExampleIndexError, render, update},
        listing::{ExampleEntry, examples, repository_root},
    };
    use std::{fs, path::PathBuf};
    use tempfile::TempDir;

    fn entries() -> Option<Vec<ExampleEntry>> {
        repository_root().and_then(|root| examples(&root)).ok()
    }

    fn readme(directory: &TempDir, text: &str) -> Option<PathBuf> {
        let path = directory.path().join("README.md");

        fs::write(&path, text).ok().map(|()| path)
    }

    #[test]
    fn the_index_carries_one_entry_for_each_example() {
        let rendered = entries().map(|entries| (render(&entries), entries.len()));

        assert!(rendered.is_some_and(|(index, count)| index.split("\n\n").count() == count * 2));
    }

    #[test]
    fn each_entry_links_to_the_page_it_names_under_its_label() {
        let rendered = entries().map(|entries| {
            let index = render(&entries);

            entries.iter().all(|entry| {
                index.contains(&format!(
                    "**[{label}](examples/{name}/example.md)**",
                    label = entry.descriptor.example.label,
                    name = entry.name
                ))
            })
        });

        assert_eq!(rendered, Some(true));
    }

    #[test]
    fn each_entry_opens_with_a_badge_for_every_format_it_reads() {
        let rendered = entries().map(|entries| {
            let index = render(&entries);

            entries.iter().all(|entry| {
                entry
                    .descriptor
                    .example
                    .formats
                    .iter()
                    .all(|format| index.contains(&format!(r#"src="https://img.shields.io/badge/format-{format}-"#)))
            })
        });

        assert_eq!(rendered, Some(true));
    }

    #[test]
    fn each_entry_closes_with_the_summary_its_descriptor_declares() {
        let rendered = entries().map(|entries| {
            let index = render(&entries);

            entries.iter().all(|entry| index.contains(&entry.descriptor.example.summary))
        });

        assert_eq!(rendered, Some(true));
    }

    #[test]
    fn the_generated_region_is_replaced_and_the_rest_of_the_readme_is_left_alone() {
        let directory = TempDir::new().ok();
        let outcome = directory.as_ref().zip(entries()).and_then(|(directory, entries)| {
            let path = readme(
                directory,
                "# Title\n\nBefore.\n\n<!-- examples:start -->\n\nstale\n<!-- examples:end -->\n\nAfter.\n",
            )?;
            update(&path, &entries).ok()?;

            fs::read_to_string(&path).ok()
        });

        assert!(outcome.is_some_and(|text| {
            text.starts_with("# Title\n\nBefore.\n\n<!-- examples:start -->\n\n<img ")
                && text.ends_with("<!-- examples:end -->\n\nAfter.\n")
                && !text.contains("stale")
        }));
    }

    #[test]
    fn a_readme_without_markers_is_reported() {
        let directory = TempDir::new().ok();
        let outcome = directory.as_ref().zip(entries()).and_then(|(directory, entries)| {
            let path = readme(directory, "# Title\n\nNo generated region here.\n")?;

            Some(update(&path, &entries))
        });

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ExampleIndexError::NoMarkers { .. }))));
    }

    #[test]
    fn markers_in_the_wrong_order_are_reported() {
        let directory = TempDir::new().ok();
        let outcome = directory.as_ref().zip(entries()).and_then(|(directory, entries)| {
            let path = readme(directory, "<!-- examples:end -->\n\n<!-- examples:start -->\n")?;

            Some(update(&path, &entries))
        });

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ExampleIndexError::NoMarkers { .. }))));
    }

    #[test]
    fn a_missing_readme_is_reported() {
        let directory = TempDir::new().ok();
        let outcome = directory
            .as_ref()
            .zip(entries())
            .map(|(directory, entries)| update(&directory.path().join("absent.md"), &entries));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ExampleIndexError::Unreadable { .. }))));
    }
}
