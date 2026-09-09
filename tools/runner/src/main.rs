mod assertion_target;
mod broker;
mod broker_stack;
mod cli;
mod command_line;
mod command_line_token;
mod container_image;
mod container_network;
mod context_broker;
mod data_model_name;
mod dataset;
mod descriptor;
mod docs_metadata;
mod drift;
mod drift_report;
mod entity_count;
mod entity_id;
mod example;
mod example_index;
mod example_name;
mod example_selector;
mod execution;
mod format_badge;
mod license_identifier;
mod listing;
mod output;
mod page;
mod poll_interval;
mod poll_schedule;
mod preparation;
mod program_name;
mod publisher_name;
mod requirements;
mod run;
mod runtime;
mod scheduled_run;
mod schema_cache;
mod shell_command;
mod source_format;
mod verification;
mod volatility;

use crate::{
    cli::{Cli, Task},
    docs_metadata::render as render_metadata,
    drift::DriftLog,
    drift_report::{DriftReportError, write as write_drift},
    example_index::{ExampleIndexError, render as render_index, update},
    execution::{DatasetPolicy, ExecutionError, OutputPolicy, fetch},
    listing::{ExampleEntry, ListingError, examples, names, repository_root, resolve, summary},
    page::{PageError, check},
    verification::{VerificationError, require_assertions},
};
use clap::Parser;
use std::{path::Path, process::ExitCode};
use thiserror::Error;

fn main() -> ExitCode {
    match dispatch(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("error: {failure}");
            ExitCode::FAILURE
        }
    }
}

/// Carries out one task.
fn dispatch(cli: Cli) -> Result<(), RunnerError> {
    let root = repository_root()?;

    match cli.command {
        Task::List { matrix } => {
            let entries = examples(&root)?;

            if matrix {
                let encoded = serde_json::to_string(&names(&entries)).map_err(RunnerError::Encoding)?;
                println!("{encoded}");
            } else {
                print!("{}", summary(&entries));
            }

            Ok(())
        }

        Task::Fetch { example } => fetch(&resolve(&root, &example)?).map_err(RunnerError::Execution),

        Task::Run {
            example,
            runtime,
            image,
            skip_fetch,
            skip_verify,
            drift_report,
        } => {
            let entry = resolve(&root, &example)?;
            let datasets = if skip_fetch { DatasetPolicy::Reuse } else { DatasetPolicy::Download };
            let outputs = if skip_verify { OutputPolicy::Trust } else { OutputPolicy::Verify };
            let drift = execution::run(&entry, runtime, &image, datasets, outputs)?;

            report(&drift, drift_report.as_deref())
        }

        Task::Verify { example } => {
            let entry = resolve(&root, &example)?;
            let drift = execution::verify(&entry)?;

            report(&drift, None)
        }

        Task::Check { example } => match example {
            Some(wanted) => inspect(&resolve(&root, &wanted)?),
            None => inspect_all(&examples(&root)?),
        },

        Task::Index { write } => {
            let entries = examples(&root)?;

            if write {
                update(&root.join("README.md"), &entries).map_err(RunnerError::ExampleIndex)
            } else {
                print!("{}", render_index(&entries));
                Ok(())
            }
        }

        Task::Metadata => {
            let entries = examples(&root)?;
            let encoded = serde_json::to_string_pretty(&render_metadata(&entries)).map_err(RunnerError::Encoding)?;
            println!("{encoded}");

            Ok(())
        }
    }
}

/// Says what drifted, and writes it where continuous integration can collect it.
///
/// Drift never changes the exit code. It is the outcome for an example that still works against a
/// source that has moved, and a run that fails on it is a run nobody reads.
fn report(drift: &DriftLog, path: Option<&Path>) -> Result<(), RunnerError> {
    if drift.is_empty() {
        println!("==> nothing has drifted");
    } else {
        println!("==> the run passed, and a live source has moved under it:");
        println!("{drift}");
    }

    match path {
        Some(path) => write_drift(path, drift).map_err(RunnerError::DriftReport),
        None => Ok(()),
    }
}

/// Checks every example, and reports everything that is wrong rather than only the first thing.
fn inspect_all(entries: &[ExampleEntry]) -> Result<(), RunnerError> {
    let failures: Vec<RunnerError> = entries.iter().filter_map(|entry| inspect(entry).err()).collect();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(RunnerError::SeveralExamples { failures })
    }
}

/// Checks one example's page and the coverage of its assertions.
fn inspect(entry: &ExampleEntry) -> Result<(), RunnerError> {
    check(&entry.directory, &entry.descriptor)?;

    require_assertions(&entry.name, &entry.descriptor.outputs).map_err(RunnerError::Verification)
}

/// Anything that can stop the runner.
#[derive(Debug, Error)]
enum RunnerError {
    /// The examples could not be found or read.
    #[error(transparent)]
    Listing(#[from] ListingError),

    /// An example could not be run.
    #[error(transparent)]
    Execution(#[from] ExecutionError),

    /// The output did not match the descriptor.
    #[error(transparent)]
    Verification(#[from] VerificationError),

    /// A page no longer documents its run.
    #[error(transparent)]
    Page(#[from] PageError),

    /// The README index could not be regenerated.
    #[error(transparent)]
    ExampleIndex(#[from] ExampleIndexError),

    /// The drift a run found could not be written.
    #[error(transparent)]
    DriftReport(#[from] DriftReportError),

    /// Several examples did not check out.
    #[error("{} examples did not check out:\n{}", failures.len(), failures.iter().map(RunnerError::to_string).collect::<Vec<String>>().join("\n"))]
    SeveralExamples {
        /// What each one failed with.
        failures: Vec<RunnerError>,
    },

    /// A JSON document the runner emits could not be encoded.
    #[error("cannot encode JSON output: {0}")]
    Encoding(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use crate::{
        RunnerError,
        cli::{Cli, Task},
        dispatch,
        example_selector::ExampleSelector,
        inspect,
        listing::{examples, repository_root},
    };
    use std::str::FromStr;

    fn checking(wanted: &str) -> Option<Result<(), RunnerError>> {
        let example = ExampleSelector::from_str(wanted).ok()?;

        Some(dispatch(Cli {
            command: Task::Check { example: Some(example) },
        }))
    }

    #[test]
    fn every_page_documents_the_run_its_descriptor_declares() {
        let listed = repository_root().and_then(|root| examples(&root));

        assert!(listed.is_ok_and(|entries| entries.iter().all(|entry| inspect(entry).is_ok())));
    }

    #[test]
    fn checking_one_example_reports_nothing_when_its_page_matches() {
        assert!(checking("01").is_some_and(|outcome| outcome.is_ok()));
    }

    #[test]
    fn an_example_that_does_not_exist_is_reported() {
        assert!(checking("99").is_some_and(|outcome| matches!(outcome, Err(RunnerError::Listing(_)))));
    }

    #[test]
    fn checking_everything_reports_every_failure_rather_than_the_first() {
        let checked = dispatch(Cli {
            command: Task::Check { example: None },
        });

        assert!(match checked {
            Ok(()) => true,
            Err(RunnerError::SeveralExamples { failures }) => !failures.is_empty(),
            Err(
                RunnerError::Listing(_)
                | RunnerError::Execution(_)
                | RunnerError::Verification(_)
                | RunnerError::Page(_)
                | RunnerError::ExampleIndex(_)
                | RunnerError::DriftReport(_)
                | RunnerError::Encoding(_),
            ) => false,
        });
    }
}
