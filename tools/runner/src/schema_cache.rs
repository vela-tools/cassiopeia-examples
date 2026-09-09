use std::{env, ffi::OsString, fs, io, path::PathBuf};
use thiserror::Error;

/// Where the Smart Data Models catalog is kept between container runs, since `--rm` would otherwise
/// discard it with the container.
///
/// The path follows the XDG base directory specification: `XDG_CACHE_HOME` when it is set, and
/// `$HOME/.cache` otherwise.
pub fn location() -> Result<PathBuf, SchemaCacheError> {
    resolve(env::var_os("XDG_CACHE_HOME"), env::var_os("HOME"))
}

/// The cache directory, created if it is not there yet.
///
/// A container mounts this path, and an engine asked to mount a path that does not exist creates it
/// as a root-owned directory the run cannot then write to.
pub fn prepare() -> Result<PathBuf, SchemaCacheError> {
    let path = location()?;

    match fs::create_dir_all(&path) {
        Ok(()) => Ok(path),
        Err(source) => Err(SchemaCacheError::Uncreatable { path, source }),
    }
}

/// The cache path for one environment.
///
/// Separate from [`location`] so the two variables can be tested without setting process-wide state.
fn resolve(cache_home: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, SchemaCacheError> {
    let base = match cache_home {
        Some(configured) => PathBuf::from(configured),
        None => PathBuf::from(home.ok_or(SchemaCacheError::Homeless)?).join(".cache"),
    };

    Ok(base.join("cassiopeia-examples").join("schemas"))
}

/// Why the schema cache could not be placed.
#[derive(Debug, Error)]
pub enum SchemaCacheError {
    /// Neither variable that could say where the cache belongs is set.
    #[error("neither XDG_CACHE_HOME nor HOME is set, so the Smart Data Models cache has nowhere to live")]
    Homeless,

    /// The directory could not be created.
    #[error("cannot create the schema cache at {path}: {source}")]
    Uncreatable {
        /// The cache directory.
        path: PathBuf,
        /// The underlying filesystem failure.
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use crate::schema_cache::{SchemaCacheError, resolve};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn the_configured_cache_directory_is_used_when_it_is_set() {
        let path = resolve(Some(OsString::from("/var/cache")), Some(OsString::from("/home/reader")));

        assert_eq!(path.ok(), Some(PathBuf::from("/var/cache/cassiopeia-examples/schemas")));
    }

    #[test]
    fn the_home_directory_carries_the_cache_when_nothing_is_configured() {
        let path = resolve(None, Some(OsString::from("/home/reader")));

        assert_eq!(path.ok(), Some(PathBuf::from("/home/reader/.cache/cassiopeia-examples/schemas")));
    }

    #[test]
    fn an_environment_with_neither_variable_is_reported_rather_than_guessed() {
        assert!(matches!(resolve(None, None), Err(SchemaCacheError::Homeless)));
    }
}
