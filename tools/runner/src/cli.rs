use crate::{container_image::ContainerImage, example_selector::ExampleSelector, runtime::Runtime};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Runs and checks the Cassiopeia examples.
///
/// Every example declares its datasets, its Cassiopeia invocation, and what the run must produce in
/// its `example.toml`. This tool is the only consumer of that file that executes anything; the pages
/// stay runnable by hand without it.
#[derive(Debug, Parser)]
#[command(name = "cassiopeia-example-runner", version, about)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Task,
}

/// The things the runner can be asked to do.
#[derive(Debug, Subcommand)]
pub enum Task {
    /// List the examples.
    List {
        /// Print the example names as a JSON array, for a workflow matrix.
        #[arg(long)]
        matrix: bool,
    },

    /// Download an example's datasets.
    Fetch {
        /// The example, by ordinal, directory name, or path.
        example: ExampleSelector,
    },

    /// Fetch, run, and verify an example.
    Run {
        /// The example, by ordinal, directory name, or path.
        example: ExampleSelector,

        /// How to invoke Cassiopeia.
        #[arg(short, long, value_enum, default_value_t = Runtime::Native)]
        runtime: Runtime,

        /// The container image, when the runtime is docker or podman.
        #[arg(short, long, default_value_t = ContainerImage::published())]
        image: ContainerImage,

        /// Keep the datasets already in the example's data directory.
        #[arg(long)]
        skip_fetch: bool,

        /// Run the example without checking its output.
        #[arg(long)]
        skip_verify: bool,

        /// Write what a live source drifted to as JSON, for continuous integration to collect.
        #[arg(long, value_name = "PATH")]
        drift_report: Option<PathBuf>,
    },

    /// Check an example's output against what its descriptor claims.
    Verify {
        /// The example, by ordinal, directory name, or path.
        example: ExampleSelector,
    },

    /// Check that the pages document the commands their descriptors declare.
    Check {
        /// One example. Every example is checked when this is omitted.
        example: Option<ExampleSelector>,
    },

    /// Render the README's index of examples.
    Index {
        /// Write the table into the README instead of printing it.
        #[arg(long)]
        write: bool,
    },

    /// Print the metadata the documentation site places the example pages with.
    Metadata,
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Task};
    use clap::{CommandFactory, Parser};
    use std::path::PathBuf;

    #[test]
    fn the_interface_is_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn a_run_defaults_to_the_host_binary_and_the_published_image() {
        let parsed = Cli::try_parse_from(["cassiopeia-example-runner", "run", "01"]);

        assert!(parsed.is_ok_and(|parsed| match parsed.command {
            Task::Run {
                example,
                runtime,
                image,
                skip_fetch,
                skip_verify,
                drift_report,
            } => {
                example.as_str() == "01"
                    && runtime.to_string() == "native"
                    && image.as_str() == "ghcr.io/vela-tools/cassiopeia:latest"
                    && !skip_fetch
                    && !skip_verify
                    && drift_report.is_none()
            }
            Task::List { .. } | Task::Fetch { .. } | Task::Verify { .. } | Task::Check { .. } | Task::Index { .. } | Task::Metadata => false,
        }));
    }

    #[test]
    fn a_container_run_takes_its_engine_and_image() {
        let parsed = Cli::try_parse_from([
            "cassiopeia-example-runner",
            "run",
            "03",
            "--runtime",
            "podman",
            "--image",
            "ghcr.io/vela-tools/cassiopeia:0.4.1",
            "--skip-fetch",
        ]);

        assert!(parsed.is_ok_and(|parsed| match parsed.command {
            Task::Run {
                runtime, image, skip_fetch, ..
            } => runtime.to_string() == "podman" && image.as_str() == "ghcr.io/vela-tools/cassiopeia:0.4.1" && skip_fetch,
            Task::List { .. } | Task::Fetch { .. } | Task::Verify { .. } | Task::Check { .. } | Task::Index { .. } | Task::Metadata => false,
        }));
    }

    #[test]
    fn a_run_writes_its_drift_where_it_is_told_to() {
        let parsed = Cli::try_parse_from(["cassiopeia-example-runner", "run", "09", "--drift-report", "drift.json"]);

        assert!(parsed.is_ok_and(|parsed| match parsed.command {
            Task::Run { drift_report, .. } => drift_report == Some(PathBuf::from("drift.json")),
            Task::List { .. } | Task::Fetch { .. } | Task::Verify { .. } | Task::Check { .. } | Task::Index { .. } | Task::Metadata => false,
        }));
    }

    #[test]
    fn checking_every_page_takes_no_example() {
        let parsed = Cli::try_parse_from(["cassiopeia-example-runner", "check"]);

        assert!(parsed.is_ok_and(|parsed| match parsed.command {
            Task::Check { example } => example.is_none(),
            Task::List { .. } | Task::Fetch { .. } | Task::Run { .. } | Task::Verify { .. } | Task::Index { .. } | Task::Metadata => false,
        }));
    }

    #[test]
    fn an_unknown_runtime_is_rejected() {
        let parsed = Cli::try_parse_from(["cassiopeia-example-runner", "run", "01", "--runtime", "containerd"]);

        assert!(parsed.is_err());
    }

    #[test]
    fn an_unknown_task_is_rejected() {
        let parsed = Cli::try_parse_from(["cassiopeia-example-runner", "publish"]);

        assert!(parsed.is_err());
    }
}
