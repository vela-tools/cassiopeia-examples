use crate::{data_model_name::DataModelName, entity_id::EntityId};
use serde_json::Value;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use thiserror::Error;
use ureq::{Agent, http::StatusCode};
use url::Url;

/// The path every NGSI-LD request goes under (ETSI GS CIM 009 v1.9.1, clause 6.2.1).
const API_ROOT: [&str; 2] = ["ngsi-ld", "v1"];

/// The media type a broker answers in when the entity's terms travel with it, which is what the
/// broker examples deliver under (clause 6.3.5).
const LINKED_JSON: &str = "application/ld+json";

/// How long to wait between readiness probes.
const PROBE_INTERVAL: Duration = Duration::from_secs(2);

/// The most entities one query asks the broker for.
///
/// Brokers page their answers, and an assertion only ever needs enough entities to count them and
/// find the identifiers a page quotes. A floor above this would need paging, and no example
/// declares one.
const QUERY_LIMIT: usize = 1000;

/// Waits until the broker answers, or the deadline passes.
///
/// Compose reports a service as up once its container is running, and only `PostGIS` declares a
/// healthcheck, so neither says anything about whether Scorpio has finished starting its runtime
/// and serving the API. This poll is what actually gates a run.
pub fn await_readiness(base: &Url, deadline: Instant) -> Result<(), BrokerError> {
    let endpoint = endpoint(base, &["types"])?;
    let agent = agent();

    loop {
        match agent.get(endpoint.as_str()).header("Accept", LINKED_JSON).call() {
            // A broker that is still starting refuses the connection or answers with a status of
            // its own; either way the only question left is whether there is still time to ask
            // again, so neither outcome is reported here.
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => (),
        }

        if Instant::now() >= deadline {
            return Err(BrokerError::NeverReady { url: endpoint });
        }

        sleep(PROBE_INTERVAL);
    }
}

/// Every entity of one type the broker holds.
///
/// The context matters here rather than being decoration: a broker expands the type in a query
/// against whichever context the query carries, so a published model's short name asked for without
/// one expands against the core context and matches no entity written under the model's own
/// context (ETSI GS CIM 009 v1.9.1, clause 5.5.7).
pub fn entities(base: &Url, model: &DataModelName, context: Option<&Url>) -> Result<Vec<Value>, BrokerError> {
    let mut endpoint = endpoint(base, &["entities"])?;
    endpoint
        .query_pairs_mut()
        .append_pair("type", model.as_str())
        .append_pair("limit", &QUERY_LIMIT.to_string());

    match fetch(&endpoint, context)? {
        Some(Value::Array(entities)) => Ok(entities),
        Some(Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Object(_)) => Err(BrokerError::NotAnEntityArray { url: endpoint }),
        // A broker that holds nothing of a type answers with an empty array rather than a 404, so
        // this arm only fires for a path the broker does not serve at all.
        None => Ok(Vec::new()),
    }
}

/// The folded `EntityTemporal` the broker holds under one identifier, as a one-entity array so that
/// the same assertions apply to it as to any other target.
///
/// The temporal endpoint is where a series representation belongs (ETSI GS CIM 009 v1.9.1, clause
/// 5.2.20), and an identifier it holds nothing for is a 404 rather than an empty answer.
pub fn temporal_entity(base: &Url, id: &EntityId) -> Result<Vec<Value>, BrokerError> {
    let endpoint = endpoint(base, &["temporal", "entities", id.as_str()])?;

    // An identifier is asked for as it was written, so nothing here has to be expanded.
    match fetch(&endpoint, None)? {
        Some(entity @ Value::Object(_)) => Ok(vec![entity]),
        Some(Value::Array(entities)) => Ok(entities),
        Some(Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => Err(BrokerError::NotAnEntityArray { url: endpoint }),
        None => Ok(Vec::new()),
    }
}

/// A client that reports a status rather than raising it, so a 404 from the temporal endpoint reads
/// as "the broker holds nothing under this identifier" instead of a transport failure.
fn agent() -> Agent {
    Agent::config_builder().http_status_as_error(false).build().into()
}

/// One NGSI-LD endpoint under a broker's base URL.
///
/// Built segment by segment rather than by joining text, because an entity identifier is a URN and
/// carries the colons that would otherwise be read as a scheme.
fn endpoint(base: &Url, segments: &[&str]) -> Result<Url, BrokerError> {
    let mut endpoint = base.clone();

    match endpoint.path_segments_mut() {
        Ok(mut path) => {
            path.pop_if_empty().extend(API_ROOT).extend(segments);
        }
        Err(()) => return Err(BrokerError::NotABase { url: base.clone() }),
    }

    Ok(endpoint)
}

/// One GET against the broker, with a 404 read as "nothing here" rather than a failure.
fn fetch(endpoint: &Url, context: Option<&Url>) -> Result<Option<Value>, BrokerError> {
    let request = agent().get(endpoint.as_str()).header("Accept", LINKED_JSON);
    let request = match context {
        Some(context) => request.header("Link", &context_link(context)),
        None => request,
    };

    // Every failure below outlives the borrowed endpoint, so each one owns the URL it names.
    let mut response = request.call().map_err(|source| BrokerError::Unreachable {
        url: endpoint.clone(),
        source: Box::new(source),
    })?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        return Err(BrokerError::Refused { url: endpoint.clone(), status });
    }

    let body = response.body_mut().read_to_string().map_err(|source| BrokerError::Unreachable {
        url: endpoint.clone(),
        source: Box::new(source),
    })?;

    serde_json::from_str(&body)
        .map(Some)
        .map_err(|source| BrokerError::Malformed { url: endpoint.clone(), source })
}

/// The `Link` header that tells a broker which context to expand a query's terms against
/// (ETSI GS CIM 009 v1.9.1, clause 6.3.5).
fn context_link(context: &Url) -> String {
    format!("<{context}>; rel=\"http://www.w3.org/ns/json-ld#context\"; type=\"{LINKED_JSON}\"")
}

/// Why a broker could not be asked what it holds.
#[derive(Debug, Error)]
pub enum BrokerError {
    /// The broker did not start, or did not finish starting, in the time it was given.
    #[error("the broker did not answer {url} in time: bring the stack up and wait for it before running the example")]
    NeverReady {
        /// The endpoint that was polled.
        url: Url,
    },

    /// The URL an example declares cannot carry a path.
    #[error("{url} is not a base URL a broker path can be built on")]
    NotABase {
        /// The URL the descriptor declares.
        url: Url,
    },

    /// The broker could not be reached at all.
    #[error("cannot reach the broker at {url}: {source}")]
    Unreachable {
        /// The endpoint that was requested.
        url: Url,

        /// The underlying transport failure.
        source: Box<ureq::Error>,
    },

    /// The broker answered, and refused.
    #[error("the broker answered {status} for {url}")]
    Refused {
        /// The endpoint that was requested.
        url: Url,

        /// The status it answered with.
        status: StatusCode,
    },

    /// The broker's answer is not JSON.
    #[error("the broker's answer for {url} is not valid JSON: {source}")]
    Malformed {
        /// The endpoint that was requested.
        url: Url,

        /// The parse failure, with its position in the answer.
        source: serde_json::Error,
    },

    /// The broker's answer is JSON, but not entities.
    #[error("the broker's answer for {url} does not hold NGSI-LD entities")]
    NotAnEntityArray {
        /// The endpoint that was requested.
        url: Url,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        broker::{context_link, endpoint},
        entity_id::EntityId,
    };
    use url::Url;

    fn base() -> Option<Url> {
        Url::parse("http://localhost:9090/").ok()
    }

    #[test]
    fn an_entity_query_is_built_under_the_ngsi_ld_api_root() {
        let built = base().and_then(|base| endpoint(&base, &["entities"]).ok());

        assert_eq!(
            built.map(|built| built.to_string()),
            Some("http://localhost:9090/ngsi-ld/v1/entities".to_owned())
        );
    }

    #[test]
    fn a_temporal_identifier_keeps_its_urn_colons_rather_than_becoming_a_scheme() {
        let built = EntityId::try_from("urn:ngsi-ld:TropicalCyclone:AL122005".to_owned())
            .ok()
            .zip(base())
            .and_then(|(id, base)| endpoint(&base, &["temporal", "entities", id.as_str()]).ok());

        assert_eq!(
            built.map(|built| built.to_string()),
            Some("http://localhost:9090/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:TropicalCyclone:AL122005".to_owned())
        );
    }

    #[test]
    fn a_context_is_offered_to_the_broker_as_a_json_ld_link() {
        let link = Url::parse("https://example.invalid/context.jsonld").ok().map(|context| context_link(&context));

        assert_eq!(
            link,
            Some("<https://example.invalid/context.jsonld>; rel=\"http://www.w3.org/ns/json-ld#context\"; type=\"application/ld+json\"".to_owned())
        );
    }

    #[test]
    fn a_base_written_without_its_trailing_slash_still_builds_the_same_path() {
        let built = Url::parse("http://localhost:9090").ok().and_then(|base| endpoint(&base, &["types"]).ok());

        assert_eq!(built.map(|built| built.to_string()), Some("http://localhost:9090/ngsi-ld/v1/types".to_owned()));
    }
}
