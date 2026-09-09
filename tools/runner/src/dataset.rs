use crate::{
    example_name::ExampleName,
    license_identifier::LicenseIdentifier,
    program_name::ProgramName,
    publisher_name::PublisherName,
    shell_command::ShellCommand,
    volatility::Volatility,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
    time::Duration,
};
use thiserror::Error;
use ureq::{Agent, http::StatusCode};
use url::Url;

/// How many times a download is attempted before the publisher is treated as gone.
const ATTEMPTS: u32 = 4;

/// How long the first retry waits. Each later one waits twice as long.
const FIRST_BACKOFF: Duration = Duration::from_secs(2);

/// The extensions whose contents a later stage parses as JSON, and which therefore have to parse
/// here: a publisher's holding page written into `data/products.json` has to fail as a download
/// rather than several stages later as a mapping.
const JSON_EXTENSIONS: [&str; 2] = ["json", "geojson"];

/// One file an example downloads before it runs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Dataset {
    /// Where the file is written, relative to the example directory.
    pub file: PathBuf,

    /// Where it is fetched from.
    pub url: Url,

    /// The publishing organisation or repository.
    pub source: PublisherName,

    /// The licence the publisher releases it under.
    pub license: LicenseIdentifier,

    /// Whether the publisher has finished with the file. Required: only the author of an example
    /// knows whether a source is a closed range or a feed, and getting it wrong turns either
    /// upstream churn into a red build or a broken mapping into a note nobody reads.
    pub volatility: Volatility,

    /// How many records the file holds, when the number is stable enough to state.
    #[serde(default)]
    pub records: Option<usize>,

    /// The shell command that produces the file, for a source a plain GET cannot fetch: a query
    /// the publisher expects as form parameters, a response that needs reshaping, or a set of byte
    /// ranges. The `url` stays as the provenance record even then.
    #[serde(default)]
    pub command: Option<ShellCommand>,
}

/// Downloads every dataset an example declares into the example directory.
///
/// A dataset already on disk is fetched again: publishers change files in place, and an example
/// that silently keeps a stale copy stops testing anything.
pub fn download_all(name: &ExampleName, directory: &Path, datasets: &[Dataset]) -> Result<(), DatasetError> {
    for dataset in datasets {
        download(name, directory, dataset)?;
    }

    Ok(())
}

/// Downloads one dataset, creating the directory it is written into.
///
/// A dataset with a `command` is produced by running it, because the publisher wants form
/// parameters, byte ranges, or a reshaping step that a plain GET cannot express. Either way the
/// file that comes out is checked before the run is allowed to go on.
pub fn download(name: &ExampleName, directory: &Path, dataset: &Dataset) -> Result<(), DatasetError> {
    let destination = directory.join(&dataset.file);

    if let Some(parent) = destination.parent()
        && let Err(source) = fs::create_dir_all(parent)
    {
        return Err(DatasetError::Unwritable {
            path: parent.to_path_buf(),
            source,
        });
    }

    match &dataset.command {
        Some(command) => produce(directory, dataset, command)?,
        None => fetch(&destination, name, dataset)?,
    }

    confirm(&destination, name, dataset)
}

/// Runs the command that produces a dataset a plain GET cannot fetch.
fn produce(directory: &Path, dataset: &Dataset, command: &ShellCommand) -> Result<(), DatasetError> {
    let status = Command::new(ProgramName::Sh.to_string())
        .arg("-c")
        .arg(command.as_str())
        .current_dir(directory)
        .status()
        .map_err(|source| DatasetError::NoShell { source })?;

    if status.success() {
        Ok(())
    } else {
        Err(DatasetError::CommandFailed {
            // The failure outlives the borrowed dataset, so it owns the path it names.
            file: dataset.file.clone(),
            code: status.code(),
        })
    }
}

/// Fetches a dataset the publisher serves over HTTP, riding out a publisher under load.
///
/// A throttled or briefly unavailable publisher is asked again; one that answers that the file is
/// not there is not, because that is a move rather than a blip and repeating cannot help.
fn fetch(destination: &Path, name: &ExampleName, dataset: &Dataset) -> Result<(), DatasetError> {
    // A client that reports a status rather than raising it, so the status itself decides whether
    // the request is worth repeating.
    let agent: Agent = Agent::config_builder().http_status_as_error(false).build().into();
    let mut backoff = FIRST_BACKOFF;
    let mut reason = String::new();

    for remaining in (0..ATTEMPTS).rev() {
        match attempt(&agent, &dataset.url) {
            Attempt::Body(body) => {
                return fs::write(destination, body).map_err(|source| DatasetError::Unwritable {
                    path: destination.to_path_buf(),
                    source,
                });
            }
            Attempt::Settled(stated) => {
                reason = stated;
                break;
            }
            Attempt::Transient(stated) => {
                reason = stated;

                if remaining > 0 {
                    println!("    {} did not answer ({reason}); asking again in {}s", dataset.source, backoff.as_secs());
                    sleep(backoff);
                    backoff = backoff.saturating_mul(2);
                }
            }
        }
    }

    Err(unreachable(name, dataset, reason))
}

/// What one attempt at a download came to.
enum Attempt {
    /// The body arrived.
    Body(Vec<u8>),

    /// The publisher is busy or briefly broken, and is worth asking again.
    Transient(String),

    /// The publisher answered, and the answer will not change.
    Settled(String),
}

/// One request, classified by whether asking again could help.
fn attempt(agent: &Agent, url: &Url) -> Attempt {
    let mut response = match agent.get(url.as_str()).call() {
        Ok(response) => response,
        // A refused connection, a name that did not resolve, or a timeout is worth another
        // attempt: the failures actually seen here are publishers under load, not moved files.
        Err(source) => return Attempt::Transient(source.to_string()),
    };

    let status = response.status();

    // Open Food Facts answers an unthrottled client with 503 and Overpass answers a busy one with
    // 429 or 504, so those are the statuses a download rides out rather than reports.
    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
        return Attempt::Transient(format!("the publisher answered {status}"));
    }

    if !status.is_success() {
        return Attempt::Settled(format!("the publisher answered {status}"));
    }

    match response.body_mut().read_to_vec() {
        Ok(body) => Attempt::Body(body),
        Err(source) => Attempt::Transient(source.to_string()),
    }
}

/// Checks that a download actually produced something a mapping can read.
///
/// A publisher that is down often answers with a holding page rather than a failure, and a command
/// that writes that page into the target leaves a file the shell was perfectly happy with. This is
/// what turns "the publisher is down" into a message that says so.
fn confirm(destination: &Path, name: &ExampleName, dataset: &Dataset) -> Result<(), DatasetError> {
    let content = match fs::read(destination) {
        Ok(content) => content,
        Err(source) => {
            return Err(unreachable(
                name,
                dataset,
                format!("nothing was written to {}: {source}", dataset.file.display()),
            ));
        }
    };

    if content.is_empty() {
        return Err(unreachable(name, dataset, format!("{} was written empty", dataset.file.display())));
    }

    let json = destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| JSON_EXTENSIONS.contains(&extension));

    if json && let Err(source) = serde_json::from_slice::<Value>(&content) {
        return Err(unreachable(
            name,
            dataset,
            format!("{} is not JSON ({source}), and it begins: {}", dataset.file.display(), opening(&content)),
        ));
    }

    Ok(())
}

/// The first line or so of a file that was supposed to be data, so a holding page is recognisable
/// in the failure it causes.
fn opening(content: &[u8]) -> String {
    const SHOWN: usize = 120;

    String::from_utf8_lossy(&content[..content.len().min(SHOWN)]).replace('\n', " ")
}

/// The failure a reader sees when a publisher stops serving a dataset.
fn unreachable(name: &ExampleName, dataset: &Dataset, reason: String) -> DatasetError {
    DatasetError::Unreachable {
        // The failure outlives the borrowed example and dataset, so it owns everything it names.
        example: name.clone(),
        url: Box::new(dataset.url.clone()),
        publisher: dataset.source.clone(),
        reason,
    }
}

/// Why a dataset could not be fetched.
#[derive(Debug, Error)]
pub enum DatasetError {
    /// The publisher did not serve the file. A moved dataset is a defect in the example, not
    /// something to work around locally.
    #[error(
        "{example} cannot obtain {url} from {publisher}: {reason}\n\
         the publisher may have moved or withdrawn the dataset, in which case the example needs a \
         different source rather than a local workaround"
    )]
    Unreachable {
        /// The example whose run was stopped.
        example: ExampleName,

        /// The URL that was requested. Boxed so that a failure a whole run is threaded through
        /// stays small enough to hand back by value.
        url: Box<Url>,

        /// Who publishes it.
        publisher: PublisherName,

        /// What went wrong, in the publisher's own terms where there are any.
        reason: String,
    },

    /// The shell that produces a dataset could not be started.
    #[error("cannot start {}: {source}", ProgramName::Sh)]
    NoShell {
        /// The underlying failure.
        source: io::Error,
    },

    /// The command that was supposed to produce the file failed.
    #[error("the command producing {file} exited with {}", match code { Some(code) => code.to_string(), None => "a signal".to_owned() })]
    CommandFailed {
        /// The file it was supposed to produce.
        file: PathBuf,

        /// Its exit code, when it had one.
        code: Option<i32>,
    },

    /// The downloaded file could not be written.
    #[error("cannot write {path}: {source}")]
    Unwritable {
        /// The path that was written to.
        path: PathBuf,

        /// The underlying filesystem failure.
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        dataset::{Dataset, DatasetError, download},
        example_name::ExampleName,
        volatility::Volatility,
    };
    use std::fs;
    use tempfile::TempDir;

    fn name() -> Option<ExampleName> {
        ExampleName::try_from("28-json-advanced-schema".to_owned()).ok()
    }

    fn dataset(text: &str) -> Option<Dataset> {
        toml::from_str(text).ok()
    }

    fn produced(file: &str, command: &str) -> Option<Dataset> {
        dataset(&format!(
            "file = \"{file}\"\nurl = \"https://example.invalid/source\"\nsource = \"a publisher\"\nlicense = \"CC0-1.0\"\nvolatility = \"live\"\ncommand = \"{command}\"\n"
        ))
    }

    fn attempted(declared: Option<Dataset>) -> Option<Result<(), DatasetError>> {
        let directory = TempDir::new().ok()?;

        Some(download(&name()?, directory.path(), &declared?))
    }

    #[test]
    fn a_dataset_with_a_command_is_produced_by_running_it() {
        let directory = TempDir::new().ok();
        let declared = produced("data/produced.txt", "printf hello > data/produced.txt");

        let written = directory.as_ref().zip(name()).zip(declared).and_then(|((directory, name), declared)| {
            download(&name, directory.path(), &declared).ok()?;

            fs::read_to_string(directory.path().join("data/produced.txt")).ok()
        });

        assert_eq!(written, Some("hello".to_owned()));
    }

    #[test]
    fn a_command_that_fails_is_reported_with_its_exit_code() {
        let outcome = attempted(produced("data/absent.txt", "exit 3"));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(DatasetError::CommandFailed { code: Some(3), .. }))));
    }

    #[test]
    fn a_command_that_writes_nothing_is_reported_as_the_publisher_failing() {
        let outcome = attempted(produced("data/produced.txt", "true"));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(DatasetError::Unreachable { .. }))));
    }

    #[test]
    fn a_command_that_writes_an_empty_file_is_reported_rather_than_passed_on() {
        let outcome = attempted(produced("data/produced.json", ": > data/produced.json"));

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(DatasetError::Unreachable { .. }))));
    }

    #[test]
    fn a_holding_page_written_into_a_json_target_is_reported_with_its_opening() {
        let outcome = attempted(produced(
            "data/products.json",
            "printf '<!DOCTYPE html><title>503</title>' > data/products.json",
        ));

        assert!(outcome.is_some_and(|outcome| match outcome {
            Err(DatasetError::Unreachable { reason, .. }) => reason.contains("<!DOCTYPE html>"),
            Err(DatasetError::NoShell { .. } | DatasetError::CommandFailed { .. } | DatasetError::Unwritable { .. }) | Ok(()) => false,
        }));
    }

    #[test]
    fn a_holding_page_written_into_a_target_nothing_parses_as_json_is_left_alone() {
        let outcome = attempted(produced("data/amenities.csv", "printf '<!DOCTYPE html>' > data/amenities.csv"));

        assert!(outcome.is_some_and(|outcome| outcome.is_ok()));
    }

    #[test]
    fn the_directory_a_dataset_is_written_into_is_created() {
        let directory = TempDir::new().ok();
        let declared = produced("data/nested/produced.txt", "printf hello > data/nested/produced.txt");

        let created = directory
            .as_ref()
            .zip(name())
            .zip(declared)
            .map(|((directory, name), declared)| download(&name, directory.path(), &declared).is_ok() && directory.path().join("data/nested").is_dir());

        assert_eq!(created, Some(true));
    }

    #[test]
    fn a_dataset_declares_its_provenance_and_its_volatility() {
        let declared = dataset(
            "file = \"data/element.json\"\nurl = \"https://example.invalid/data.json\"\nsource = \"andrejewski/periodic-table\"\nlicense = \"ISC\"\nvolatility = \"frozen\"\nrecords = 118\n",
        );

        assert_eq!(
            declared.map(|declared| (declared.source.to_string(), declared.license.to_string(), declared.volatility, declared.records)),
            Some(("andrejewski/periodic-table".to_owned(), "ISC".to_owned(), Volatility::Frozen, Some(118)))
        );
    }

    #[test]
    fn a_dataset_that_does_not_say_whether_it_moves_is_rejected() {
        let declared = dataset("file = \"data/x.json\"\nurl = \"https://example.invalid/x\"\nsource = \"s\"\nlicense = \"ISC\"\n");

        assert!(declared.is_none());
    }

    #[test]
    fn a_dataset_with_an_unknown_key_is_rejected() {
        let declared = dataset(
            "file = \"data/x.json\"\nurl = \"https://example.invalid/x\"\nsource = \"s\"\nlicense = \"ISC\"\nvolatility = \"live\"\nchecksum = \"abc\"\n",
        );

        assert!(declared.is_none());
    }
}
