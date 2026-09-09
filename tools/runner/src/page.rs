use crate::{
    container_image::ContainerImage,
    descriptor::Descriptor,
    runtime::{documented_container_command, documented_native_command},
};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// The walkthrough every example directory holds.
pub const PAGE_FILE: &str = "example.md";

/// The fence language the runnable commands are written in.
const SHELL_FENCE: &str = "```bash";

/// The fence that closes any block.
const FENCE: &str = "```";

/// Checks that a page still documents the run the descriptor declares.
///
/// The page is the path for a reader who has neither the runner nor a Rust toolchain, so it has to
/// carry both forms of the command verbatim. Comparing them here turns drift into a failed check
/// instead of a reader running something that no longer works.
pub fn check(directory: &Path, descriptor: &Descriptor) -> Result<(), PageError> {
    let path = directory.join(PAGE_FILE);

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) => return Err(PageError::Unreadable { path, source }),
    };

    let heading = match text.lines().find_map(|line| line.strip_prefix("# ")) {
        Some(heading) => heading.trim(),
        None => return Err(PageError::NoHeading { path }),
    };

    if heading != descriptor.example.title {
        return Err(PageError::TitleMismatch {
            path,
            heading: heading.to_owned(),
            // The error outlives the descriptor it is raised from, so it owns the title.
            title: descriptor.example.title.clone(),
        });
    }

    let blocks = shell_blocks(&text);
    let image = ContainerImage::published();

    for expected in [
        documented_native_command(&descriptor.run),
        documented_container_command(&descriptor.run, descriptor.requirements, descriptor.network(), &image),
    ] {
        if !blocks.iter().any(|block| block.trim() == expected) {
            return Err(PageError::MissingCommand { path, expected });
        }
    }

    Ok(())
}

/// What a line does to the block being collected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delimiter {
    /// The line opens a shell block.
    OpensShell,

    /// The line closes whatever block is open.
    Closes,

    /// The line is not a fence.
    None,
}

/// The content of every shell code fence on a page.
fn shell_blocks(text: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Option<Vec<&str>> = None;

    for line in text.lines() {
        let delimiter = match line.trim_end() {
            SHELL_FENCE => Delimiter::OpensShell,
            FENCE => Delimiter::Closes,
            _text => Delimiter::None,
        };

        match (delimiter, current.take()) {
            (Delimiter::OpensShell, None) => current = Some(Vec::new()),
            (Delimiter::Closes, Some(collected)) => blocks.push(collected.join("\n")),
            // A fence line inside an open block is content: the pages quote fenced markdown.
            (Delimiter::OpensShell | Delimiter::None, Some(mut collected)) => {
                collected.push(line);
                current = Some(collected);
            }
            (Delimiter::Closes | Delimiter::None, None) => (),
        }
    }

    blocks
}

/// Why a page and its descriptor disagree.
#[derive(Debug, Error)]
pub enum PageError {
    /// The page is missing or unreadable.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The page that was looked for.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The page has no H1.
    #[error("{path} has no H1 heading")]
    NoHeading {
        /// The page in question.
        path: PathBuf,
    },

    /// The page's H1 is not the title the descriptor declares.
    #[error("{path} is titled \"{heading}\", but its descriptor says \"{title}\"")]
    TitleMismatch {
        /// The page in question.
        path: PathBuf,
        /// What the page says.
        heading: String,
        /// What the descriptor says.
        title: String,
    },

    /// The page does not carry a command the descriptor declares.
    #[error("{path} does not document this command:\n\n{expected}\n")]
    MissingCommand {
        /// The page in question.
        path: PathBuf,
        /// The command it should have carried.
        expected: String,
    },
}

#[cfg(test)]
mod tests {
    use crate::page::shell_blocks;

    #[test]
    fn a_shell_fence_is_collected_without_its_delimiters() {
        let text = "text\n\n```bash\ncassiopeia map \\\n    --output out\n```\n\nmore text\n";

        assert_eq!(shell_blocks(text), vec!["cassiopeia map \\\n    --output out".to_owned()]);
    }

    #[test]
    fn a_fence_in_another_language_is_ignored() {
        let text = "```json5\n{ version: \"v4\" }\n```\n";

        assert!(shell_blocks(text).is_empty());
    }

    #[test]
    fn several_shell_fences_are_collected_in_order() {
        let text = "```bash\nfirst\n```\n\n```bash\nsecond\n```\n";

        assert_eq!(shell_blocks(text), vec!["first".to_owned(), "second".to_owned()]);
    }

    #[test]
    fn a_page_without_any_fence_yields_no_blocks() {
        assert!(shell_blocks("# A page\n\nProse only.\n").is_empty());
    }

    #[test]
    fn an_unclosed_fence_yields_no_block() {
        assert!(shell_blocks("```bash\ncassiopeia map\n").is_empty());
    }
}
