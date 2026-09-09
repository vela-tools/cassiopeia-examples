use crate::{
    broker_stack::{BrokerStack, BrokerStackError},
    command_line_token::{CommandLineToken, CommandLineTokenError},
    container_image::ContainerImage,
    dataset::{DatasetError, download_all},
    drift::DriftLog,
    listing::ExampleEntry,
    poll_schedule::PollSchedule,
    preparation::Preparation,
    program_name::ProgramName,
    runtime::{Runtime, invocation},
    scheduled_run::{ScheduledRunError, run_bounded},
    schema_cache::{SchemaCacheError, prepare},
    verification::{VerificationError, verify_all},
    volatility::combined,
};
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

/// The invocation that downloads the Smart Data Models catalog.
const CATALOG_DOWNLOAD: [&str; 2] = ["sdm", "download"];

/// Whether a run downloads its datasets first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetPolicy {
    /// Download every dataset the example declares.
    Download,

    /// Use whatever is already on disk.
    Reuse,
}

/// Whether a run checks its output afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPolicy {
    /// Check the output against the descriptor.
    Verify,

    /// Leave the output unchecked.
    Trust,
}

/// Downloads an example's datasets, naming each publisher and licence as it goes.
pub fn fetch(entry: &ExampleEntry) -> Result<(), ExecutionError> {
    println!("==> {}: fetching the dataset", entry.name);

    for dataset in &entry.descriptor.datasets {
        let records = match dataset.records {
            Some(records) => format!(", {records} records"),
            None => String::new(),
        };
        println!(
            "    {} from {} ({}, {}){records}",
            dataset.file.display(),
            dataset.source,
            dataset.license,
            dataset.volatility
        );
    }

    download_all(&entry.name, &entry.directory, &entry.descriptor.datasets).map_err(ExecutionError::Dataset)
}

/// Fetches, runs, and verifies one example.
///
/// Answers with whatever drifted rather than taking a sink for it, so the caller decides what a
/// live source having moved is worth.
pub fn run(entry: &ExampleEntry, runtime: Runtime, image: &ContainerImage, datasets: DatasetPolicy, outputs: OutputPolicy) -> Result<DriftLog, ExecutionError> {
    match datasets {
        DatasetPolicy::Download => fetch(entry)?,
        DatasetPolicy::Reuse => (),
    }

    for step in &entry.descriptor.preparation {
        prepare_dataset(entry, step)?;
    }

    let cache = schema_cache(entry, runtime)?;

    // The stack is bound before the run so that it is dropped after it, taking the broker down
    // whether the mapping succeeded or failed.
    let _stack = start_broker(entry, runtime)?;

    if entry.descriptor.requirements.smart_data_models {
        println!("==> {}: downloading the Smart Data Models catalog", entry.name);
        execute(entry, runtime, &catalog_arguments()?, cache.as_deref(), image)?;
    }

    println!("==> {}: running ({runtime})", entry.name);
    match entry.descriptor.schedule {
        Some(schedule) => execute_scheduled(entry, runtime, cache.as_deref(), image, schedule)?,
        None => execute(entry, runtime, &entry.descriptor.run.args, cache.as_deref(), image)?,
    }

    let drift = match outputs {
        OutputPolicy::Verify => {
            println!("==> {}: verifying the output", entry.name);
            verify(entry)?
        }
        OutputPolicy::Trust => DriftLog::default(),
    };

    println!("==> {}: done", entry.name);

    Ok(drift)
}

/// Brings up the broker an example delivers to, when it declares one.
fn start_broker(entry: &ExampleEntry, runtime: Runtime) -> Result<Option<BrokerStack>, ExecutionError> {
    match &entry.descriptor.context_broker {
        Some(broker) => {
            let stack = BrokerStack::up(runtime.compose_engine(), &entry.directory, &entry.directory.join(&broker.compose), &broker.url)?;

            Ok(Some(stack))
        }
        None => Ok(None),
    }
}

/// Checks what the run produced against what the example claims about it.
///
/// Public so that checking an existing `out/` reads the descriptor the same way a whole run does,
/// rather than working out an example's volatility and broker a second time.
pub fn verify(entry: &ExampleEntry) -> Result<DriftLog, ExecutionError> {
    let volatility = combined(entry.descriptor.datasets.iter().map(|dataset| &dataset.volatility));

    verify_all(
        &entry.directory,
        &entry.name,
        &entry.descriptor.outputs,
        volatility,
        entry.descriptor.context_broker.as_ref(),
    )
    .map_err(ExecutionError::Verification)
}

/// Runs one preparation step in the example directory.
fn prepare_dataset(entry: &ExampleEntry, step: &Preparation) -> Result<(), ExecutionError> {
    println!("==> {}: {}", entry.name, step.description);

    let status = Command::new(ProgramName::Sh.to_string())
        .arg("-c")
        .arg(step.command.as_str())
        .current_dir(&entry.directory)
        .status()
        .map_err(|source| ExecutionError::NotExecutable {
            program: ProgramName::Sh,
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(ExecutionError::PreparationFailed {
            // The failure outlives the descriptor it is raised from, so it owns the description.
            description: step.description.clone(),
            code: status.code(),
        })
    }
}

/// The host directory a container run mounts the Smart Data Models catalog from.
///
/// A native run reads the catalog through Cassiopeia's own cache, so it needs nothing here.
fn schema_cache(entry: &ExampleEntry, runtime: Runtime) -> Result<Option<PathBuf>, ExecutionError> {
    match (runtime, entry.descriptor.requirements.smart_data_models) {
        (Runtime::Docker | Runtime::Podman, true) => Ok(Some(prepare()?)),
        (Runtime::Docker | Runtime::Podman | Runtime::Native, false) | (Runtime::Native, true) => Ok(None),
    }
}

/// The catalog download as command-line tokens.
fn catalog_arguments() -> Result<Vec<CommandLineToken>, CommandLineTokenError> {
    CATALOG_DOWNLOAD.into_iter().map(|token| CommandLineToken::try_from(token.to_owned())).collect()
}

/// Invokes Cassiopeia in the example directory, under whichever runtime was asked for.
fn execute(
    entry: &ExampleEntry,
    runtime: Runtime,
    arguments: &[CommandLineToken],
    schema_cache: Option<&Path>,
    image: &ContainerImage,
) -> Result<(), ExecutionError> {
    let invocation = invocation(runtime, &entry.directory, arguments, schema_cache, entry.descriptor.network(), image);

    let status = Command::new(invocation.program.to_string())
        .args(&invocation.arguments)
        .current_dir(&entry.directory)
        .status()
        .map_err(|source| ExecutionError::NotExecutable {
            program: invocation.program,
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(ExecutionError::RunFailed {
            program: invocation.program,
            code: status.code(),
        })
    }
}

/// Invokes a mapping that keeps polling, and stops it once its cycles have had time to happen.
fn execute_scheduled(
    entry: &ExampleEntry,
    runtime: Runtime,
    schema_cache: Option<&Path>,
    image: &ContainerImage,
    schedule: PollSchedule,
) -> Result<(), ExecutionError> {
    let invocation = invocation(
        runtime,
        &entry.directory,
        &entry.descriptor.run.args,
        schema_cache,
        entry.descriptor.network(),
        image,
    );

    println!("    polling {} times every {}, then interrupting the run", schedule.cycles, schedule.interval);

    run_bounded(&invocation, &entry.directory, schedule).map_err(ExecutionError::Scheduled)
}

/// Why an example could not be run.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// A dataset could not be fetched.
    #[error(transparent)]
    Dataset(#[from] DatasetError),

    /// The output did not match the descriptor.
    #[error(transparent)]
    Verification(#[from] VerificationError),

    /// The schema cache could not be placed.
    #[error(transparent)]
    SchemaCache(#[from] SchemaCacheError),

    /// The broker the example delivers to could not be started.
    #[error(transparent)]
    Broker(#[from] BrokerStackError),

    /// The scheduled run could not be bounded.
    #[error(transparent)]
    Scheduled(#[from] ScheduledRunError),

    /// The catalog invocation could not be built.
    #[error(transparent)]
    Token(#[from] CommandLineTokenError),

    /// A preparation step failed.
    #[error("the preparation step \"{description}\" exited with {}", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    PreparationFailed {
        /// What the step was for.
        description: String,

        /// Its exit code, when it had one.
        code: Option<i32>,
    },

    /// Cassiopeia, Docker, Podman, or the shell is not installed.
    #[error("cannot execute {program}: {source}")]
    NotExecutable {
        /// The program that was invoked.
        program: ProgramName,

        /// The underlying failure.
        source: io::Error,
    },

    /// The run itself failed.
    #[error("{program} exited with {}", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    RunFailed {
        /// The program that was invoked.
        program: ProgramName,

        /// Its exit code, when it had one.
        code: Option<i32>,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        container_image::ContainerImage,
        descriptor::Descriptor,
        example_name::ExampleName,
        execution::{DatasetPolicy, ExecutionError, OutputPolicy, catalog_arguments, run},
        listing::ExampleEntry,
        runtime::Runtime,
    };
    use tempfile::TempDir;

    /// An example whose preparation step fails before anything is invoked.
    const FAILING_PREPARATION: &str = r#"
[example]
title = "t"
label = "l"
summary = "s"
formats = ["csv"]
models = ["City"]

[[preparation]]
description = "a step that cannot succeed"
command = "exit 4"

[run]
args = ["map"]
"#;

    fn entry(directory: &TempDir, descriptor: &str) -> Option<ExampleEntry> {
        let name = ExampleName::try_from("01-a-example".to_owned()).ok()?;
        let descriptor: Descriptor = toml::from_str(descriptor).ok()?;

        Some(ExampleEntry {
            name,
            directory: directory.path().to_path_buf(),
            descriptor,
        })
    }

    #[test]
    fn a_preparation_step_that_fails_stops_the_run_with_its_exit_code() {
        let directory = TempDir::new().ok();
        let outcome = directory
            .as_ref()
            .and_then(|directory| entry(directory, FAILING_PREPARATION))
            .map(|entry| run(&entry, Runtime::Native, &ContainerImage::published(), DatasetPolicy::Reuse, OutputPolicy::Trust));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ExecutionError::PreparationFailed { code: Some(4), .. }))));
    }

    #[test]
    fn the_catalog_is_downloaded_by_the_sdm_subcommand() {
        let arguments = catalog_arguments();

        assert_eq!(
            arguments
                .map(|arguments| arguments.iter().map(|token| token.as_str().to_owned()).collect::<Vec<String>>())
                .ok(),
            Some(vec!["sdm".to_owned(), "download".to_owned()])
        );
    }
}
