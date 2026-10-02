use crate::cassiopeia_release::{CassiopeiaRelease, CassiopeiaReleaseError};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

/// The file at the repository root that names the Cassiopeia release the examples are written for.
pub const PIN_FILE: &str = "cassiopeia-version";

/// The release this checkout of the examples is written and checked against.
///
/// One file for the whole repository rather than a key per descriptor: every example in a checkout
/// targets the same release, and a tag of this repository is named after the release it pins. A
/// plain file rather than TOML, so a workflow step reads it with `cat`.
pub fn pinned_release(root: &Path) -> Result<CassiopeiaRelease, ReleasePinError> {
    let path = root.join(PIN_FILE);

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) => return Err(ReleasePinError::Unreadable { path, source }),
    };

    CassiopeiaRelease::from_str(text.trim()).map_err(|source| ReleasePinError::Malformed { path, source })
}

/// Why the pinned release could not be read.
#[derive(Debug, Error)]
pub enum ReleasePinError {
    /// The pin file is missing or unreadable.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The file that was looked for.
        path: PathBuf,

        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The pin file does not hold a release tag.
    #[error("{path} does not pin a release: {source}")]
    Malformed {
        /// The file in question.
        path: PathBuf,

        /// Why its content is not a release tag.
        source: CassiopeiaReleaseError,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        cassiopeia_release::CassiopeiaRelease,
        release_pin::{PIN_FILE, ReleasePinError, pinned_release},
    };
    use semver::Version;
    use std::fs;
    use tempfile::TempDir;

    fn pinning(content: &str) -> Option<Result<CassiopeiaRelease, ReleasePinError>> {
        let root = TempDir::new().ok()?;
        fs::write(root.path().join(PIN_FILE), content).ok()?;

        Some(pinned_release(root.path()))
    }

    #[test]
    fn a_pin_with_a_trailing_newline_names_its_release() {
        assert!(pinning("v1.0.0\n").is_some_and(|pinned| pinned.ok() == Some(CassiopeiaRelease::from(Version::new(1, 0, 0)))));
    }

    #[test]
    fn a_pin_that_is_not_a_release_tag_is_rejected() {
        assert!(pinning("latest\n").is_some_and(|pinned| matches!(pinned, Err(ReleasePinError::Malformed { .. }))));
    }

    #[test]
    fn a_missing_pin_is_reported() {
        let root = TempDir::new().ok();

        assert!(root.is_some_and(|root| matches!(pinned_release(root.path()), Err(ReleasePinError::Unreadable { .. }))));
    }
}
