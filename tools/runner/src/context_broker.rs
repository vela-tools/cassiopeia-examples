use serde::Deserialize;
use std::path::PathBuf;
use url::Url;

/// The broker an example delivers to, and the compose stack that provides it.
///
/// A block of its own rather than a flag in `[requirements]`, because a broker is not a yes-or-no
/// question: the runner has to know which compose file starts it and where it answers, and a flag
/// could say neither.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ContextBroker {
    /// The compose file that starts the broker, relative to the example directory. Examples that
    /// need the same broker point at one stack rather than each shipping a copy of it.
    pub compose: PathBuf,

    /// Where the broker answers, matching the URL the example's manifest delivers to.
    pub url: Url,

    /// The `@context` the run's entities were written under, for an example that asserts on a
    /// whole entity type.
    ///
    /// A broker expands the type in a query against whichever context the query carries, and
    /// against the core context a published model's short name expands to something no entity
    /// written under that model's own context matches (ETSI GS CIM 009 v1.9.1, clause 5.5.7). An
    /// assertion that names one entity by identifier needs no context, which is why this is
    /// optional rather than required.
    #[serde(default)]
    pub context: Option<Url>,
}

#[cfg(test)]
mod tests {
    use crate::context_broker::ContextBroker;
    use std::path::PathBuf;

    fn broker(text: &str) -> Option<ContextBroker> {
        toml::from_str(text).ok()
    }

    #[test]
    fn a_block_carries_the_compose_file_and_the_url() {
        let declared = broker("compose = \"docker-compose.yml\"\nurl = \"http://localhost:9090/\"\n");

        assert_eq!(
            declared.map(|declared| (declared.compose, declared.url.to_string())),
            Some((PathBuf::from("docker-compose.yml"), "http://localhost:9090/".to_owned()))
        );
    }

    #[test]
    fn a_stack_shared_with_another_example_is_named_by_a_relative_path() {
        let declared = broker("compose = \"../22-csv-broker-temporal/docker-compose.yml\"\nurl = \"http://localhost:9090/\"\n");

        assert_eq!(
            declared.map(|declared| declared.compose),
            Some(PathBuf::from("../22-csv-broker-temporal/docker-compose.yml"))
        );
    }

    #[test]
    fn a_block_naming_the_context_its_entities_were_written_under_reads_it() {
        let declared = broker("compose = \"docker-compose.yml\"\nurl = \"http://localhost:9090/\"\ncontext = \"https://example.invalid/context.jsonld\"\n");

        assert_eq!(
            declared.and_then(|declared| declared.context).map(|context| context.to_string()),
            Some("https://example.invalid/context.jsonld".to_owned())
        );
    }

    #[test]
    fn a_block_without_a_context_reads_as_having_none() {
        let declared = broker("compose = \"docker-compose.yml\"\nurl = \"http://localhost:9090/\"\n");

        assert!(declared.is_some_and(|declared| declared.context.is_none()));
    }

    #[test]
    fn a_block_without_a_url_is_rejected() {
        assert!(broker("compose = \"docker-compose.yml\"\n").is_none());
    }

    #[test]
    fn a_url_without_a_scheme_is_rejected() {
        assert!(broker("compose = \"docker-compose.yml\"\nurl = \"localhost/ngsi-ld\"\n").is_none());
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        assert!(broker("compose = \"docker-compose.yml\"\nurl = \"http://localhost:9090/\"\ntoken = \"secret\"\n").is_none());
    }
}
