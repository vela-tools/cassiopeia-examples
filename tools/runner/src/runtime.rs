use crate::{
    command_line::{group_arguments, render},
    command_line_token::CommandLineToken,
    container_image::ContainerImage,
    container_network::ContainerNetwork,
    program_name::ProgramName,
    requirements::Requirements,
    run::Run,
};
use clap::ValueEnum;
#[cfg(unix)]
use rustix::process::{getgid, getuid};
use std::{ffi::OsString, path::Path};
use strum::Display;

/// Where the container runs read and write the Smart Data Models catalog.
const CONTAINER_SCHEMAS: &str = "/var/lib/cassiopeia/schemas";

/// Where the example directory is mounted, and the working directory inside the container, so that
/// a relative path means the same thing in a container as it does on the host.
const CONTAINER_WORKDIR: &str = "/data";

/// Where a page tells the reader to keep the catalog, matching what [`crate::schema_cache`] resolves
/// for a reader whose `XDG_CACHE_HOME` is unset.
const DOCUMENTED_SCHEMA_CACHE: &str = "$HOME/.cache/cassiopeia-examples/schemas";

/// How Cassiopeia is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Runtime {
    /// A binary on the host's `PATH`.
    #[default]
    Native,

    /// The published image, under Docker.
    Docker,

    /// The published image, under Podman.
    Podman,
}

impl Runtime {
    /// The program a run of this kind invokes.
    pub const fn program(self) -> ProgramName {
        match self {
            Runtime::Native => ProgramName::Cassiopeia,
            Runtime::Docker => ProgramName::Docker,
            Runtime::Podman => ProgramName::Podman,
        }
    }

    /// The engine that brings a broker's compose stack up for a run of this kind.
    ///
    /// A native run has no engine of its own, and the compose file and the pages that use it both
    /// name Docker, so that is what a native run's broker comes up under.
    pub const fn compose_engine(self) -> ProgramName {
        match self {
            Runtime::Native | Runtime::Docker => ProgramName::Docker,
            Runtime::Podman => ProgramName::Podman,
        }
    }
}

/// A program and the arguments to start it with.
///
/// Arguments are `OsString` rather than text because they carry filesystem paths, which no platform
/// guarantees to be valid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The program to start.
    pub program: ProgramName,

    /// What to pass it.
    pub arguments: Vec<OsString>,
}

/// The command an example page documents for a host binary.
pub fn documented_native_command(run: &Run) -> String {
    render(&native_lines(run))
}

/// The command an example page documents for Docker.
///
/// Podman differs only in the engine name and in dropping `--user`, which the page states in prose
/// rather than repeating the whole command.
pub fn documented_container_command(run: &Run, requirements: Requirements, network: ContainerNetwork, image: &ContainerImage) -> String {
    let mut lines = vec!["docker run --rm".to_owned(), "--user \"$(id -u):$(id -g)\"".to_owned()];

    match network {
        ContainerNetwork::Host => lines.push(format!("--network {network}")),
        ContainerNetwork::Isolated => (),
    }

    lines.push(format!("--volume \"$PWD:{CONTAINER_WORKDIR}\""));

    if requirements.smart_data_models {
        lines.push(format!("--volume \"{DOCUMENTED_SCHEMA_CACHE}:{CONTAINER_SCHEMAS}\""));
    }

    lines.push(format!("--workdir {CONTAINER_WORKDIR}"));
    lines.push(image.as_str().to_owned());
    lines.extend(group_arguments(&run.args));

    render(&lines)
}

/// The program and arguments to execute for a run, with real values where the documented form
/// carries shell expansions.
///
/// A schema cache is mounted only when one is given, which is how a run that needs the Smart Data
/// Models catalog differs from one that does not.
pub fn invocation(
    runtime: Runtime,
    directory: &Path,
    arguments: &[CommandLineToken],
    schema_cache: Option<&Path>,
    network: ContainerNetwork,
    image: &ContainerImage,
) -> Invocation {
    let program = runtime.program();

    match runtime {
        Runtime::Native => Invocation {
            program,
            arguments: arguments.iter().map(|token| OsString::from(token.as_str())).collect(),
        },
        Runtime::Docker | Runtime::Podman => {
            let mut engine_arguments = vec![OsString::from("run"), OsString::from("--rm")];

            // Rootless Podman already maps the container's root to the invoking user, so forcing an
            // identity there writes files nobody on the host owns. Docker needs it, otherwise the
            // output directory comes back owned by root.
            if runtime == Runtime::Docker {
                engine_arguments.extend(user_arguments());
            }

            // A run that delivers to a broker reaches it at the same localhost URL its manifest
            // and its page name, which only holds when the container shares the host's network.
            match network {
                ContainerNetwork::Host => {
                    engine_arguments.push(OsString::from("--network"));
                    engine_arguments.push(OsString::from(network.to_string()));
                }
                ContainerNetwork::Isolated => (),
            }

            engine_arguments.push(OsString::from("--volume"));
            engine_arguments.push(mount(directory, CONTAINER_WORKDIR));

            if let Some(cache) = schema_cache {
                engine_arguments.push(OsString::from("--volume"));
                engine_arguments.push(mount(cache, CONTAINER_SCHEMAS));
            }

            engine_arguments.push(OsString::from("--workdir"));
            engine_arguments.push(OsString::from(CONTAINER_WORKDIR));
            engine_arguments.push(OsString::from(image.as_str()));
            engine_arguments.extend(arguments.iter().map(|token| OsString::from(token.as_str())));

            Invocation {
                program,
                arguments: engine_arguments,
            }
        }
    }
}

/// One `--volume` argument, built without going through text so that a host path that is not valid
/// UTF-8 still mounts.
fn mount(host: &Path, container: &str) -> OsString {
    let mut specification = OsString::from(host);
    specification.push(":");
    specification.push(container);

    specification
}

/// The lines of the host-binary form of a run.
fn native_lines(run: &Run) -> Vec<String> {
    let mut lines = vec![ProgramName::Cassiopeia.to_string()];
    lines.extend(group_arguments(&run.args));

    // The subcommand belongs on the first line, next to the program.
    match lines.len() {
        0 | 1 => lines,
        _more => {
            let subcommand = lines.remove(1);
            lines[0] = format!("{} {subcommand}", ProgramName::Cassiopeia);
            lines
        }
    }
}

/// The `--user` pair that keeps Docker from writing output owned by root.
#[cfg(unix)]
fn user_arguments() -> Vec<OsString> {
    vec![OsString::from("--user"), OsString::from(format!("{}:{}", getuid().as_raw(), getgid().as_raw()))]
}

/// Windows and other non-unix hosts have no uid to map, and Docker Desktop does not need one.
#[cfg(not(unix))]
fn user_arguments() -> Vec<OsString> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use crate::{
        command_line_token::CommandLineToken,
        container_image::ContainerImage,
        container_network::ContainerNetwork,
        program_name::ProgramName,
        requirements::Requirements,
        run::Run,
        runtime::{Runtime, documented_container_command, documented_native_command, invocation},
    };
    use std::{ffi::OsString, path::Path};

    fn run() -> Run {
        Run {
            args: ["map", "--input", "data/element.json", "--output", "out"]
                .iter()
                .filter_map(|value| CommandLineToken::try_from((*value).to_owned()).ok())
                .collect(),
        }
    }

    #[test]
    fn the_native_command_puts_the_subcommand_beside_the_program() {
        assert_eq!(
            documented_native_command(&run()),
            "cassiopeia map \\\n    --input data/element.json \\\n    --output out"
        );
    }

    #[test]
    fn the_container_command_mounts_the_example_directory_as_the_working_directory() {
        let documented = documented_container_command(&run(), Requirements::default(), ContainerNetwork::Isolated, &ContainerImage::published());

        assert!(documented.contains("--volume \"$PWD:/data\""));
        assert!(documented.contains("--workdir /data"));
        assert!(documented.ends_with("--output out"));
    }

    #[test]
    fn a_run_without_the_catalog_mounts_no_schema_volume() {
        let documented = documented_container_command(&run(), Requirements::default(), ContainerNetwork::Isolated, &ContainerImage::published());

        assert!(!documented.contains("schemas"));
    }

    #[test]
    fn a_run_needing_the_catalog_mounts_the_schema_volume() {
        let requirements = Requirements { smart_data_models: true };
        let documented = documented_container_command(&run(), requirements, ContainerNetwork::Isolated, &ContainerImage::published());

        assert!(documented.contains("$HOME/.cache/cassiopeia-examples/schemas:/var/lib/cassiopeia/schemas"));
    }

    #[test]
    fn podman_is_invoked_without_a_user_mapping() {
        let built = invocation(
            Runtime::Podman,
            Path::new("/tmp/example"),
            &run().args,
            None,
            ContainerNetwork::Isolated,
            &ContainerImage::published(),
        );

        assert_eq!(built.program, ProgramName::Podman);
        assert!(!built.arguments.iter().any(|argument| argument == "--user"));
    }

    #[test]
    fn a_container_run_with_a_cache_mounts_it_beside_the_example_directory() {
        let built = invocation(
            Runtime::Podman,
            Path::new("/tmp/example"),
            &run().args,
            Some(Path::new("/tmp/cache")),
            ContainerNetwork::Isolated,
            &ContainerImage::published(),
        );

        assert!(built.arguments.contains(&OsString::from("/tmp/example:/data")));
        assert!(built.arguments.contains(&OsString::from("/tmp/cache:/var/lib/cassiopeia/schemas")));
    }

    #[test]
    fn a_broker_run_documents_and_invokes_host_networking() {
        let documented = documented_container_command(&run(), Requirements::default(), ContainerNetwork::Host, &ContainerImage::published());
        let built = invocation(
            Runtime::Docker,
            Path::new("/tmp/example"),
            &run().args,
            None,
            ContainerNetwork::Host,
            &ContainerImage::published(),
        );

        assert!(documented.contains("--network host"));
        assert!(built.arguments.contains(&OsString::from("--network")));
        assert!(built.arguments.contains(&OsString::from("host")));
    }

    #[test]
    fn a_run_that_needs_no_broker_names_no_network() {
        let documented = documented_container_command(&run(), Requirements::default(), ContainerNetwork::Isolated, &ContainerImage::published());

        assert!(!documented.contains("--network"));
    }

    #[test]
    fn a_native_invocation_passes_the_arguments_through_unchanged() {
        let built = invocation(
            Runtime::Native,
            Path::new("/tmp/example"),
            &run().args,
            None,
            ContainerNetwork::Isolated,
            &ContainerImage::published(),
        );

        assert_eq!(built.program, ProgramName::Cassiopeia);
        assert_eq!(
            built.arguments,
            run().args.iter().map(|token| OsString::from(token.as_str())).collect::<Vec<OsString>>()
        );
    }
}
