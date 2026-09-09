use crate::drift::DriftLog;
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Writes the drift a run found to the file `--drift-report` names.
///
/// The file is written even when the run drifted nowhere: a continuous integration job that merges
/// one report per example has to be able to tell an example that reported nothing from an example
/// whose leg never got far enough to report at all.
pub fn write(path: &Path, log: &DriftLog) -> Result<(), DriftReportError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(source) = fs::create_dir_all(parent)
    {
        return Err(DriftReportError::Unwritable {
            path: parent.to_path_buf(),
            source,
        });
    }

    let encoded = match serde_json::to_string_pretty(log) {
        Ok(encoded) => encoded,
        Err(source) => return Err(DriftReportError::Encoding { source }),
    };

    fs::write(path, encoded).map_err(|source| DriftReportError::Unwritable {
        path: path.to_path_buf(),
        source,
    })
}

/// Why a drift report could not be written.
#[derive(Debug, Error)]
pub enum DriftReportError {
    /// The report, or the directory it belongs in, could not be written.
    #[error("cannot write the drift report to {path}: {source}")]
    Unwritable {
        /// The path that was written to.
        path: PathBuf,

        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The findings could not be encoded.
    #[error("cannot encode the drift report: {source}")]
    Encoding {
        /// The failure serde reported.
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        assertion_target::AssertionTarget,
        drift::{Drift, DriftKind, DriftLog},
        drift_report::write,
        example_name::ExampleName,
    };
    use std::{fs, path::PathBuf};
    use tempfile::TempDir;

    fn log() -> Option<DriftLog> {
        let mut log = DriftLog::default();
        log.record(Drift {
            example: ExampleName::try_from("09-csv-manifest".to_owned()).ok()?,
            target: AssertionTarget::File(PathBuf::from("out/Airport.json")),
            kind: DriftKind::EntityCount { declared: 7000, found: 7698 },
        });

        Some(log)
    }

    #[test]
    fn a_report_is_written_as_json_naming_both_figures() {
        let directory = TempDir::new().ok();
        let written = directory.as_ref().zip(log()).and_then(|(directory, log)| {
            let path = directory.path().join("drift.json");
            write(&path, &log).ok()?;

            fs::read_to_string(path).ok()
        });

        assert!(written.is_some_and(|written| written.contains("\"declared\": 7000") && written.contains("\"found\": 7698")));
    }

    #[test]
    fn a_run_that_drifted_nowhere_still_writes_an_empty_report() {
        let directory = TempDir::new().ok();
        let written = directory.as_ref().and_then(|directory| {
            let path = directory.path().join("drift.json");
            write(&path, &DriftLog::default()).ok()?;

            fs::read_to_string(path).ok()
        });

        assert_eq!(written, Some("[]".to_owned()));
    }

    #[test]
    fn the_directory_a_report_is_written_into_is_created() {
        let directory = TempDir::new().ok();
        let created = directory.as_ref().and_then(|directory| {
            let path = directory.path().join("reports/09/drift.json");
            write(&path, &DriftLog::default()).ok()?;

            Some(path.is_file())
        });

        assert_eq!(created, Some(true));
    }
}
