use derive_more::{Display, From};
use semver::Version;
use std::str::FromStr;
use thiserror::Error;

/// A Cassiopeia release, named the way its git tag and its container image are: `v1.0.3`, or
/// `v1.1.0-alpha1` for a pre-release.
#[derive(Debug, Clone, PartialEq, Eq, Display, From)]
#[display("v{_0}")]
pub struct CassiopeiaRelease(Version);

// Written by hand because the tag carries a `v` the version does not, and a derived `FromStr` would
// hand the whole tag to the semver parser, which rejects it.
impl FromStr for CassiopeiaRelease {
    type Err = CassiopeiaReleaseError;

    fn from_str(tag: &str) -> Result<CassiopeiaRelease, CassiopeiaReleaseError> {
        let Some(version) = tag.strip_prefix('v') else {
            return Err(CassiopeiaReleaseError::NoPrefix { tag: tag.to_owned() });
        };

        Version::parse(version)
            .map(CassiopeiaRelease)
            .map_err(|source| CassiopeiaReleaseError::NotSemver { tag: tag.to_owned(), source })
    }
}

/// Why a release tag could not be read.
#[derive(Debug, Error)]
pub enum CassiopeiaReleaseError {
    /// The tag lacks the `v` every Cassiopeia release tag starts with.
    #[error("\"{tag}\" is not a release tag: Cassiopeia's tags start with a \"v\", as in v1.0.3")]
    NoPrefix {
        /// What was read.
        tag: String,
    },

    /// What follows the `v` is not a semantic version.
    #[error("\"{tag}\" is not a release tag: {source}")]
    NotSemver {
        /// What was read.
        tag: String,

        /// What the semver parser objected to.
        source: semver::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::cassiopeia_release::{CassiopeiaRelease, CassiopeiaReleaseError};
    use semver::Version;
    use std::str::FromStr;

    #[test]
    fn a_release_tag_reads_back_as_written() {
        let release = CassiopeiaRelease::from_str("v1.0.3");

        assert_eq!(release.map(|release| release.to_string()).ok(), Some("v1.0.3".to_owned()));
    }

    #[test]
    fn a_pre_release_tag_reads_back_as_written() {
        let release = CassiopeiaRelease::from_str("v1.1.0-alpha1");

        assert_eq!(release.map(|release| release.to_string()).ok(), Some("v1.1.0-alpha1".to_owned()));
    }

    #[test]
    fn a_tag_names_the_same_release_as_its_bare_version() {
        let release = CassiopeiaRelease::from_str("v1.0.0");

        assert_eq!(release.ok(), Some(CassiopeiaRelease::from(Version::new(1, 0, 0))));
    }

    #[test]
    fn a_version_without_the_v_is_rejected() {
        assert!(matches!(CassiopeiaRelease::from_str("1.0.3"), Err(CassiopeiaReleaseError::NoPrefix { .. })));
    }

    #[test]
    fn a_tag_that_is_not_a_semantic_version_is_rejected() {
        assert!(matches!(CassiopeiaRelease::from_str("v1.0"), Err(CassiopeiaReleaseError::NotSemver { .. })));
    }

    #[test]
    fn a_floating_tag_is_rejected() {
        assert!(CassiopeiaRelease::from_str("latest").is_err());
    }
}
