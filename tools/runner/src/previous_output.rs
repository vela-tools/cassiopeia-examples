use crate::{assertion_target::AssertionTarget, output::Output};
use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Removes the files an earlier run left where this run is about to write.
///
/// Verification reads the declared files once the run is over, and a file an earlier run wrote looks
/// exactly like one this run wrote. A run that writes nothing, because its source answered with an
/// empty file or the mapping produced no entities, would otherwise be checked against the last good
/// output and pass. A broker target keeps nothing on disk, so it has nothing to remove.
pub fn remove_previous_outputs(directory: &Path, outputs: &[Output]) -> Result<(), PreviousOutputError> {
    outputs
        .iter()
        .filter_map(|output| match &output.target {
            AssertionTarget::File(file) => Some(directory.join(file)),
            AssertionTarget::BrokerEntities(_) | AssertionTarget::BrokerTemporal(_) => None,
        })
        .try_for_each(|path| match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(source) => Err(PreviousOutputError::Unremovable { path, source }),
        })
}

/// Why an earlier run's output could not be cleared.
#[derive(Debug, Error)]
pub enum PreviousOutputError {
    /// The file is there but could not be removed.
    #[error("cannot remove the earlier output {path}: {source}")]
    Unremovable {
        /// The file in question.
        path: PathBuf,

        /// The underlying filesystem failure.
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::{output::Output, previous_output::remove_previous_outputs};
    use serde::Deserialize;
    use std::{fs, path::Path};
    use tempfile::TempDir;

    /// A file output and a broker output, as a descriptor declares them.
    const DECLARED: &str = "[[outputs]]\nfile = \"out/City.json\"\nentities = 1\n\n[[outputs]]\nbroker-entities = \"AirQualityObserved\"\nat-least = 5\n";

    #[derive(Deserialize)]
    struct Declared {
        outputs: Vec<Output>,
    }

    /// Writes `files` under a fresh example directory, clears the declared outputs, and answers which
    /// of `files` are still there.
    fn after_clearing(files: &[&str]) -> Option<Vec<String>> {
        let declared: Declared = toml::from_str(DECLARED).ok()?;
        let directory = TempDir::new().ok()?;

        for file in files {
            let path = directory.path().join(file);
            fs::create_dir_all(path.parent()?).ok()?;
            fs::write(&path, "[]").ok()?;
        }

        remove_previous_outputs(directory.path(), &declared.outputs).ok()?;

        Some(
            files
                .iter()
                .filter(|file| Path::exists(&directory.path().join(file)))
                .map(|file| (*file).to_owned())
                .collect(),
        )
    }

    #[test]
    fn a_file_an_earlier_run_wrote_is_removed() {
        assert_eq!(after_clearing(&["out/City.json"]), Some(Vec::new()));
    }

    #[test]
    fn a_file_that_is_not_there_yet_is_no_error() {
        assert_eq!(after_clearing(&[]), Some(Vec::new()));
    }

    #[test]
    fn a_file_no_output_declares_is_left_alone() {
        assert_eq!(after_clearing(&["out/City.json", "out/Notes.json"]), Some(vec!["out/Notes.json".to_owned()]));
    }
}
