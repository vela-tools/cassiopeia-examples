use strum::Display;

/// An executable the runner invokes.
///
/// The set is closed: Cassiopeia itself, the two container engines that can carry it, and the shell
/// that runs preparation steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ProgramName {
    /// The Cassiopeia binary on the host's `PATH`.
    Cassiopeia,

    /// The Docker CLI.
    Docker,

    /// The Podman CLI.
    Podman,

    /// The shell that runs preparation steps and dataset commands.
    Sh,
}

#[cfg(test)]
mod tests {
    use crate::program_name::ProgramName;

    #[test]
    fn each_program_prints_the_name_it_is_invoked_by() {
        assert_eq!(ProgramName::Cassiopeia.to_string(), "cassiopeia");
        assert_eq!(ProgramName::Docker.to_string(), "docker");
        assert_eq!(ProgramName::Podman.to_string(), "podman");
        assert_eq!(ProgramName::Sh.to_string(), "sh");
    }
}
