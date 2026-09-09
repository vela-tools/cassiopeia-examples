use crate::{
    broker::{BrokerError, await_readiness},
    program_name::ProgramName,
};
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use thiserror::Error;
use url::Url;

/// How long the broker is given to answer once its containers are running.
///
/// Only the database declares a healthcheck in the compose file, so compose reports the stack up
/// while the broker is still starting. The readiness poll is what gates the run, and this is how
/// long it is worth waiting for.
const READINESS_TIMEOUT: Duration = Duration::from_secs(300);

/// Bringing the stack up in the background, waiting for what does declare a healthcheck.
const UP: [&str; 3] = ["up", "-d", "--wait"];

/// Taking it down, discarding the database volume so the next run starts from nothing.
const DOWN: [&str; 2] = ["down", "-v"];

/// A compose stack brought up for one run and taken down with it.
///
/// The stack is torn down by [`Drop`] rather than at the end of the run, because a mapping that
/// fails has to leave as little behind as one that succeeds.
#[derive(Debug)]
pub struct BrokerStack {
    /// The engine that runs compose.
    engine: ProgramName,

    /// The example directory the compose file is named relative to.
    directory: PathBuf,

    /// The compose file the stack came up from. Declared last so that it is still there for the
    /// shutdown, which names it.
    compose: PathBuf,
}

impl BrokerStack {
    /// Brings the stack up and waits for the broker to answer.
    ///
    /// The value exists before anything is started, so a stack that comes up and then fails its
    /// readiness poll is still taken down.
    pub fn up(engine: ProgramName, directory: &Path, compose: &Path, url: &Url) -> Result<BrokerStack, BrokerStackError> {
        let stack = BrokerStack {
            engine,
            directory: directory.to_path_buf(),
            compose: compose.to_path_buf(),
        };

        println!("==> starting the broker with {engine} compose");
        invoke(&stack, &UP)?;

        println!("==> waiting for the broker at {url}");
        await_readiness(url, Instant::now() + READINESS_TIMEOUT)?;

        Ok(stack)
    }
}

impl Drop for BrokerStack {
    fn drop(&mut self) {
        println!("==> stopping the broker");

        // Nothing above can act on a stack that will not stop, and a Drop cannot hand a failure
        // back, so the outcome is reported where a reader will see it rather than propagated.
        if let Err(failure) = invoke(self, &DOWN) {
            eprintln!("warning: {failure}");
        }
    }
}

/// Runs one compose subcommand against the stack's file, from the example directory.
fn invoke(stack: &BrokerStack, arguments: &[&str]) -> Result<(), BrokerStackError> {
    let status = Command::new(stack.engine.to_string())
        .arg("compose")
        .arg("--file")
        .arg(&stack.compose)
        .args(arguments)
        .current_dir(&stack.directory)
        .status()
        .map_err(|source| BrokerStackError::NotExecutable { program: stack.engine, source })?;

    if status.success() {
        Ok(())
    } else {
        Err(BrokerStackError::ComposeFailed {
            // The failure outlives the borrowed stack, so it owns the file it names.
            compose: stack.compose.clone(),
            code: status.code(),
        })
    }
}

/// Why a broker stack could not be run.
#[derive(Debug, Error)]
pub enum BrokerStackError {
    /// The container engine is not installed.
    #[error("cannot execute {program}: {source}")]
    NotExecutable {
        /// The engine that was invoked.
        program: ProgramName,

        /// The underlying failure.
        source: io::Error,
    },

    /// Compose itself failed.
    #[error("compose exited with {} for {compose}", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    ComposeFailed {
        /// The compose file it was given.
        compose: PathBuf,

        /// Its exit code, when it had one.
        code: Option<i32>,
    },

    /// The stack came up, and the broker never started serving.
    #[error(transparent)]
    Unready(#[from] BrokerError),
}

#[cfg(test)]
mod tests {
    use crate::{
        broker_stack::{BrokerStackError, DOWN, UP},
        program_name::ProgramName,
    };
    use std::{
        io::{Error, ErrorKind},
        path::PathBuf,
    };

    #[test]
    fn the_stack_comes_up_detached_and_waits_for_what_declares_a_healthcheck() {
        assert_eq!(UP, ["up", "-d", "--wait"]);
    }

    #[test]
    fn the_stack_goes_down_with_its_volumes() {
        assert_eq!(DOWN, ["down", "-v"]);
    }

    #[test]
    fn a_missing_engine_is_reported_by_the_name_it_is_invoked_by() {
        let failure = BrokerStackError::NotExecutable {
            program: ProgramName::Docker,
            source: Error::from(ErrorKind::NotFound),
        };

        assert!(failure.to_string().starts_with("cannot execute docker: "));
    }

    #[test]
    fn a_compose_failure_names_the_file_and_the_exit_code() {
        let failure = BrokerStackError::ComposeFailed {
            compose: PathBuf::from("docker-compose.yml"),
            code: Some(1),
        };

        assert_eq!(failure.to_string(), "compose exited with 1 for docker-compose.yml");
    }

    #[test]
    fn a_stack_stopped_by_a_signal_is_reported_as_such() {
        let failure = BrokerStackError::ComposeFailed {
            compose: PathBuf::from("docker-compose.yml"),
            code: None,
        };

        assert_eq!(failure.to_string(), "compose exited with a signal for docker-compose.yml");
    }
}
