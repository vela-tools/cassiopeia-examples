use crate::{cassiopeia_release::CassiopeiaRelease, program_name::ProgramName};
use semver::Version;
use std::{io, process::Command};
use thiserror::Error;

/// Checks that the Cassiopeia on the host's `PATH` is the release the examples are pinned to.
///
/// A container run names its release in the image tag, so only a native run can pick up a different
/// one, and it does so silently: an older binary still maps, just not the way the page says it does.
pub fn require_installed(pinned: &CassiopeiaRelease) -> Result<(), InstalledReleaseError> {
    let output = Command::new(ProgramName::Cassiopeia.to_string())
        .arg("--version")
        .output()
        .map_err(|source| InstalledReleaseError::NotExecutable { source })?;

    if !output.status.success() {
        return Err(InstalledReleaseError::VersionFailed { code: output.status.code() });
    }

    let installed = release_in_banner(&String::from_utf8_lossy(&output.stdout))?;

    if &installed == pinned {
        Ok(())
    } else {
        Err(InstalledReleaseError::Mismatch {
            // The error outlives the pin it is compared against, so it owns a copy.
            pinned: pinned.clone(),
            installed,
        })
    }
}

/// The release `cassiopeia --version` reports, from a first line such as
/// `cassiopeia 1.0.3 · foss edition`.
fn release_in_banner(banner: &str) -> Result<CassiopeiaRelease, InstalledReleaseError> {
    let unrecognised = || InstalledReleaseError::Unrecognised { banner: banner.to_owned() };
    let first_line: Option<Vec<&str>> = banner.lines().next().map(|line| line.split_whitespace().take(2).collect());

    match first_line.as_deref() {
        Some(["cassiopeia", version]) => Version::parse(version).map(CassiopeiaRelease::from).map_err(|_unparsed| unrecognised()),
        Some(_other) => Err(unrecognised()),
        None => Err(unrecognised()),
    }
}

/// Why the installed Cassiopeia cannot run the examples.
#[derive(Debug, Error)]
pub enum InstalledReleaseError {
    /// Cassiopeia is not on the `PATH`.
    #[error("cannot execute cassiopeia: {source}")]
    NotExecutable {
        /// The underlying failure.
        source: io::Error,
    },

    /// `cassiopeia --version` failed.
    #[error("cassiopeia --version exited with {}", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    VersionFailed {
        /// Its exit code, when it had one.
        code: Option<i32>,
    },

    /// `cassiopeia --version` printed something other than a release.
    #[error("cannot read a release out of cassiopeia --version:\n{banner}")]
    Unrecognised {
        /// What it printed.
        banner: String,
    },

    /// The installed release is not the pinned one.
    #[error("the cassiopeia on PATH is {installed}, but cassiopeia-version pins {pinned}; install {pinned}, or run in a container with --runtime docker")]
    Mismatch {
        /// The release the examples are written for.
        pinned: CassiopeiaRelease,

        /// The release on the `PATH`.
        installed: CassiopeiaRelease,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        cassiopeia_release::CassiopeiaRelease,
        installed_release::{InstalledReleaseError, release_in_banner},
    };
    use semver::Version;

    #[test]
    fn a_release_banner_names_its_release() {
        let banner = "cassiopeia 1.0.0 · foss edition\n7ad85e93 · clean · EUPL-1.2\nfeatures +grib1 · +grib2-full\n";

        assert_eq!(release_in_banner(banner).ok(), Some(CassiopeiaRelease::from(Version::new(1, 0, 0))));
    }

    #[test]
    fn a_pre_release_banner_names_its_pre_release() {
        let banner = "cassiopeia 1.1.0-alpha1 · foss edition\n";

        assert_eq!(
            release_in_banner(banner).map(|release| release.to_string()).ok(),
            Some("v1.1.0-alpha1".to_owned())
        );
    }

    #[test]
    fn a_banner_from_another_program_is_unrecognised() {
        assert!(matches!(release_in_banner("docker 27.1.1\n"), Err(InstalledReleaseError::Unrecognised { .. })));
    }

    #[test]
    fn a_banner_without_a_version_is_unrecognised() {
        assert!(matches!(release_in_banner("cassiopeia\n"), Err(InstalledReleaseError::Unrecognised { .. })));
    }

    #[test]
    fn empty_output_is_unrecognised() {
        assert!(matches!(release_in_banner(""), Err(InstalledReleaseError::Unrecognised { .. })));
    }
}
