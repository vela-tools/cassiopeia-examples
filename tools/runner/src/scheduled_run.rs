use crate::{poll_schedule::PollSchedule, program_name::ProgramName, runtime::Invocation};
#[cfg(unix)]
use rustix::{
    io::Errno,
    process::{Pid, Signal, kill_process},
};
use std::{
    io,
    path::Path,
    process::{Child, Command, ExitStatus},
    thread::sleep,
    time::{Duration, Instant},
};
use thiserror::Error;

/// How often the run is looked in on while its window is open.
const SETTLE: Duration = Duration::from_secs(2);

/// How long the run is given to finish after it has been interrupted.
const SHUTDOWN: Duration = Duration::from_secs(60);

/// The code a program stopped by an interrupt reports: the shell convention of 128 plus the signal
/// number. A container engine's client answers with it too, having been interrupted itself.
const INTERRUPTED: i32 = 130;

/// Runs a scheduled mapping for the window its descriptor allows, stops it the way its page tells a
/// reader to, and waits for it to finish.
///
/// A scheduled run has no natural end: it polls until something interrupts it. Nothing in
/// continuous integration sits at a terminal to do that, so the window is what stands in for the
/// reader's Ctrl-C, and the same shutdown path a reader exercises is the one checked here.
pub fn run_bounded(invocation: &Invocation, directory: &Path, schedule: PollSchedule) -> Result<(), ScheduledRunError> {
    let program = invocation.program;
    let started = Instant::now();

    let mut child = Command::new(program.to_string())
        .args(&invocation.arguments)
        .current_dir(directory)
        .spawn()
        .map_err(|source| ScheduledRunError::NotExecutable { program, source })?;

    if let Some(status) = settle(&mut child, program, schedule.deadline(started))? {
        return Err(ScheduledRunError::StoppedEarly {
            program,
            code: status.code(),
            cycles: schedule.cycles.get(),
        });
    }

    println!("==> interrupting the run after {} cycles", schedule.cycles);
    interrupt(&child)?;

    if let Some(status) = settle(&mut child, program, Instant::now() + SHUTDOWN)? {
        return classify(program, status);
    }

    // Nothing else can be learned from a run that ignores an interrupt, and leaving it behind would
    // hold the broker's port for whatever runs next, so the outcome of the kill is not what decides
    // the run: ignoring the interrupt already did.
    match child.kill().and_then(|()| child.wait()) {
        Ok(_) | Err(_) => Err(ScheduledRunError::Unresponsive { program }),
    }
}

/// Waits for the run to finish of its own accord, up to a deadline.
///
/// Answers with the status when it finished, and with nothing when the deadline came first.
fn settle(child: &mut Child, program: ProgramName, deadline: Instant) -> Result<Option<ExitStatus>, ScheduledRunError> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => (),
            Err(source) => return Err(ScheduledRunError::Unreapable { program, source }),
        }

        if Instant::now() >= deadline {
            return Ok(None);
        }

        sleep(SETTLE);
    }
}

/// How a run that stopped after being interrupted is read.
///
/// A mapping that handles the interrupt exits cleanly, one that lets the signal terminate it has no
/// exit code at all, and one that reports the interrupt reports 130. All three are the shutdown
/// working. Any other code is the run failing.
fn classify(program: ProgramName, status: ExitStatus) -> Result<(), ScheduledRunError> {
    match status.code() {
        None | Some(0 | INTERRUPTED) => Ok(()),
        Some(code) => Err(ScheduledRunError::StoppedBadly { program, code }),
    }
}

/// Asks the run to stop the way Ctrl-C does.
///
/// Under a container runtime the signal reaches the engine's client, which proxies it to the
/// container, so the mapping inside sees the same interrupt either way.
#[cfg(unix)]
fn interrupt(child: &Child) -> Result<(), ScheduledRunError> {
    kill_process(Pid::from_child(child), Signal::INT).map_err(|source| ScheduledRunError::Unsignalable { source })
}

/// Windows has no signal a scheduled run could be asked to stop with.
#[cfg(not(unix))]
fn interrupt(_child: &Child) -> Result<(), ScheduledRunError> {
    Err(ScheduledRunError::Unsupported)
}

/// Why a scheduled run could not be bounded.
#[derive(Debug, Error)]
pub enum ScheduledRunError {
    /// Cassiopeia or the container engine is not installed.
    #[error("cannot execute {program}: {source}")]
    NotExecutable {
        /// The program that was invoked.
        program: ProgramName,

        /// The underlying failure.
        source: io::Error,
    },

    /// The run stopped before its cycles had time to happen, which means the schedule stopped
    /// scheduling.
    #[error("{program} stopped with {} before its {cycles} cycles were done", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    StoppedEarly {
        /// The program that was invoked.
        program: ProgramName,

        /// Its exit code, when it had one.
        code: Option<i32>,

        /// How many cycles the descriptor allows.
        cycles: u32,
    },

    /// The run was interrupted and then failed on its way out.
    #[error("{program} exited with {code} after it was interrupted")]
    StoppedBadly {
        /// The program that was invoked.
        program: ProgramName,

        /// The code it exited with.
        code: i32,
    },

    /// The run ignored the interrupt.
    #[error("{program} did not stop when it was interrupted, and had to be killed")]
    Unresponsive {
        /// The program that was invoked.
        program: ProgramName,
    },

    /// The run could not be waited for.
    #[error("cannot wait for {program}: {source}")]
    Unreapable {
        /// The program that was invoked.
        program: ProgramName,

        /// The underlying failure.
        source: io::Error,
    },

    /// The interrupt could not be sent.
    #[cfg(unix)]
    #[error("cannot interrupt the run: {source}")]
    Unsignalable {
        /// The failure the operating system reported.
        source: Errno,
    },

    /// The host has no interrupt to send.
    #[cfg(not(unix))]
    #[error("a scheduled example can only be bounded on a unix host")]
    Unsupported,
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use crate::scheduled_run::classify;
    use crate::{
        poll_schedule::PollSchedule,
        program_name::ProgramName,
        runtime::Invocation,
        scheduled_run::{ScheduledRunError, run_bounded},
    };
    use std::{ffi::OsString, path::Path};
    #[cfg(unix)]
    use std::{os::unix::process::ExitStatusExt, process::ExitStatus};
    use tempfile::TempDir;

    /// A wait status for a process the interrupt terminated: the signal number in the low bits, and
    /// no exit code at all.
    #[cfg(unix)]
    const SIGINT_STATUS: i32 = 2;

    fn schedule(text: &str) -> Option<PollSchedule> {
        toml::from_str(text).ok()
    }

    fn shell(script: &str) -> Invocation {
        Invocation {
            program: ProgramName::Sh,
            arguments: vec![OsString::from("-c"), OsString::from(script)],
        }
    }

    fn bounded(script: &str, declared: &str) -> Option<Result<(), ScheduledRunError>> {
        let directory = TempDir::new().ok()?;

        Some(run_bounded(&shell(script), directory.path(), schedule(declared)?))
    }

    #[test]
    fn a_run_that_polls_past_its_window_is_interrupted_and_reaped() {
        let outcome = bounded(
            "trap 'exit 0' INT; while true; do sleep 1; done",
            "cycles = 1\ninterval = \"1s\"\ngrace = \"1s\"\n",
        );

        assert!(outcome.is_some_and(|outcome| outcome.is_ok()));
    }

    #[test]
    fn a_run_reporting_the_interrupt_in_the_usual_way_is_a_clean_stop() {
        let outcome = bounded(
            "trap 'exit 130' INT; while true; do sleep 1; done",
            "cycles = 1\ninterval = \"1s\"\ngrace = \"1s\"\n",
        );

        assert!(outcome.is_some_and(|outcome| outcome.is_ok()));
    }

    #[test]
    #[cfg(unix)]
    fn a_run_the_interrupt_terminated_outright_is_a_clean_stop() {
        // Built rather than provoked: whether a shell lets an interrupt through to itself depends
        // on the shell, and what is being checked here is how the status is read, not that.
        assert!(classify(ProgramName::Cassiopeia, ExitStatus::from_raw(SIGINT_STATUS)).is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn a_run_that_fails_on_its_way_out_is_reported_with_its_code() {
        let outcome = classify(ProgramName::Cassiopeia, ExitStatus::from_raw(4 << 8));

        assert!(matches!(outcome, Err(ScheduledRunError::StoppedBadly { code: 4, .. })));
    }

    #[test]
    fn a_schedule_that_stops_scheduling_before_its_window_is_reported() {
        let outcome = bounded("exit 0", "cycles = 5\ninterval = \"1s\"\ngrace = \"5s\"\n");

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ScheduledRunError::StoppedEarly { cycles: 5, .. }))));
    }

    #[test]
    fn a_run_that_fails_inside_its_window_is_reported_with_its_code() {
        let outcome = bounded("exit 4", "cycles = 2\ninterval = \"1s\"\ngrace = \"3s\"\n");

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ScheduledRunError::StoppedEarly { code: Some(4), .. }))));
    }

    #[test]
    fn a_program_that_is_not_installed_is_reported_rather_than_waited_for() {
        let declared = schedule("cycles = 1\ninterval = \"1s\"\ngrace = \"1s\"\n");
        let invocation = Invocation {
            program: ProgramName::Cassiopeia,
            arguments: vec![OsString::from("--version")],
        };
        let outcome = declared.map(|declared| run_bounded(&invocation, Path::new("/nonexistent-example-directory"), declared));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(ScheduledRunError::NotExecutable { .. }))));
    }
}
