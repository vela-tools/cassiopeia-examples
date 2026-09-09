use crate::{example_name::ExampleName, listing::ExampleEntry};
use serde::Serialize;

/// One example page, as the documentation site needs it to place the page.
///
/// The site reads this instead of keeping its own list: order comes from the directory prefix, the
/// page title from `title`, and the sidebar entry from `label`, so an example is described once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageMetadata<'entry> {
    /// The example's directory, which the page is served under.
    directory: &'entry ExampleName,

    /// The page title, matching the H1 on the page.
    title: &'entry str,

    /// The sidebar entry.
    label: &'entry str,

    /// One sentence for an index of pages.
    summary: &'entry str,
}

/// Renders the metadata the documentation site needs to place the example pages.
pub fn render(entries: &[ExampleEntry]) -> Vec<PageMetadata<'_>> {
    entries
        .iter()
        .map(|entry| PageMetadata {
            directory: &entry.name,
            title: &entry.descriptor.example.title,
            label: &entry.descriptor.example.label,
            summary: &entry.descriptor.example.summary,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        docs_metadata::render,
        listing::{examples, repository_root},
    };

    #[test]
    fn every_example_is_described_once_in_reading_order() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| {
            let rendered = render(&entries);

            rendered.len() == entries.len() && rendered.first().map(|page| page.directory) == entries.first().map(|entry| &entry.name)
        }));
    }

    #[test]
    fn a_page_carries_the_title_and_label_its_descriptor_declares() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| {
            let rendered = render(&entries);

            rendered
                .iter()
                .zip(entries.iter())
                .all(|(page, entry)| page.title == entry.descriptor.example.title && page.label == entry.descriptor.example.label)
        }));
    }

    #[test]
    fn the_metadata_serialises_with_the_keys_the_site_reads() {
        let listed = repository_root().and_then(|root| examples(&root));
        let encoded = listed.ok().and_then(|entries| serde_json::to_string(&render(&entries)).ok());

        assert!(encoded.is_some_and(|encoded| {
            encoded.starts_with("[{\"directory\":") && encoded.contains("\"title\":") && encoded.contains("\"label\":") && encoded.contains("\"summary\":")
        }));
    }
}
