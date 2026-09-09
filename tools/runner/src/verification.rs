use crate::{
    assertion_target::AssertionTarget,
    broker::{BrokerError, entities, temporal_entity},
    context_broker::ContextBroker,
    drift::{Drift, DriftKind, DriftLog},
    entity_count::{EntityCount, Plausibility},
    entity_id::EntityId,
    example_name::ExampleName,
    output::Output,
    volatility::{Mismatch, Volatility},
};
use serde_json::Value;
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Checks that an example claims something about what it produces.
///
/// Every example runs unattended, so every example has to assert what the run produces; without an
/// `[[outputs]]` block the run could produce nothing at all and still pass.
pub fn require_assertions(name: &ExampleName, outputs: &[Output]) -> Result<(), VerificationError> {
    if outputs.is_empty() {
        return Err(VerificationError::NoAssertions {
            // The failure outlives the listing it is raised from, so it owns the name.
            name: name.clone(),
        });
    }

    Ok(())
}

/// Checks everything a run was supposed to produce against what the example claims about it.
///
/// Returns whatever drifted rather than failing on it: a live source that grew under a working
/// example is worth reporting, and is not worth a red matrix.
pub fn verify_all(
    directory: &Path,
    name: &ExampleName,
    outputs: &[Output],
    volatility: Volatility,
    broker: Option<&ContextBroker>,
) -> Result<DriftLog, VerificationError> {
    let mut log = DriftLog::default();

    for output in outputs {
        log.absorb(verify(directory, name, output, volatility, broker)?);
    }

    Ok(log)
}

/// Checks one output: that it holds the stated number of entities, and that the entities the page
/// quotes are among them.
pub fn verify(
    directory: &Path,
    name: &ExampleName,
    output: &Output,
    volatility: Volatility,
    broker: Option<&ContextBroker>,
) -> Result<DriftLog, VerificationError> {
    let found = collect(directory, &output.target, broker)?;

    assess(name, &found, output, volatility)
}

/// The entities one target holds.
fn collect(directory: &Path, target: &AssertionTarget, broker: Option<&ContextBroker>) -> Result<Vec<Value>, VerificationError> {
    match target {
        AssertionTarget::File(file) => read_entities(&directory.join(file)),
        AssertionTarget::BrokerEntities(model) => {
            let broker = address(target, broker)?;

            entities(&broker.url, model, broker.context.as_ref()).map_err(VerificationError::Broker)
        }
        AssertionTarget::BrokerTemporal(id) => temporal_entity(&address(target, broker)?.url, id).map_err(VerificationError::Broker),
    }
}

/// Where a broker target is queried, or a failure naming the target that has nowhere to go.
fn address<'broker>(target: &AssertionTarget, broker: Option<&'broker ContextBroker>) -> Result<&'broker ContextBroker, VerificationError> {
    broker.ok_or_else(|| VerificationError::NoBroker {
        // The failure outlives the borrowed descriptor, so it owns what it names.
        target: target.clone(),
    })
}

/// The entities a file holds, which is the JSON array a file run writes.
fn read_entities(path: &Path) -> Result<Vec<Value>, VerificationError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) => {
            return Err(VerificationError::Unreadable {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let parsed: Value = match serde_json::from_str(&text) {
        Ok(parsed) => parsed,
        Err(source) => {
            return Err(VerificationError::Malformed {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    match parsed {
        Value::Array(entities) => Ok(entities),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Object(_) => Err(VerificationError::NotAnEntityArray {
            target: AssertionTarget::File(path.to_path_buf()),
        }),
    }
}

/// Checks entities against what the example claims about them.
///
/// Separate from [`verify`] so the assertions can be tested without touching a filesystem or a
/// broker.
fn assess(name: &ExampleName, found: &[Value], output: &Output, volatility: Volatility) -> Result<DriftLog, VerificationError> {
    let mut log = DriftLog::default();

    if !output.count.satisfied_by(found.len()) {
        match volatility.classify(output.count.plausibility(found.len())) {
            Mismatch::Broken => return Err(wrong_count(&output.target, output.count, found.len())),
            Mismatch::Churn => log.record(Drift {
                // Every drift outlives the borrowed descriptor, so each one owns what it names.
                example: name.clone(),
                target: output.target.clone(),
                kind: DriftKind::EntityCount {
                    declared: output.count.declared(),
                    found: found.len(),
                },
            }),
        }
    }

    let present: Vec<&str> = found.iter().filter_map(|entity| entity.get("id")).filter_map(Value::as_str).collect();

    for declared in &output.contains {
        if present.iter().any(|id| *id == declared.as_str()) {
            continue;
        }

        // An identifier is either there or not, so there is no near miss to rule out: whether its
        // absence is churn is entirely the source's volatility.
        match volatility.classify(Plausibility::Churnable) {
            Mismatch::Broken => {
                return Err(VerificationError::MissingEntity {
                    target: output.target.clone(),
                    declared: declared.clone(),
                });
            }
            Mismatch::Churn => log.record(Drift {
                example: name.clone(),
                target: output.target.clone(),
                kind: DriftKind::MissingEntity { declared: declared.clone() },
            }),
        }
    }

    Ok(log)
}

/// The failure a count that is not satisfied raises, in the terms the count was declared in.
fn wrong_count(target: &AssertionTarget, count: EntityCount, found: usize) -> VerificationError {
    match count {
        EntityCount::Exactly(expected) => VerificationError::WrongEntityCount {
            target: target.clone(),
            expected,
            found,
        },
        EntityCount::AtLeast(fewest) => VerificationError::TooFewEntities {
            target: target.clone(),
            fewest,
            found,
        },
    }
}

/// Why a run's output did not match what its example claims.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// The file the run was supposed to write is missing or unreadable.
    #[error("cannot read {path}: {source}")]
    Unreadable {
        /// The output file.
        path: PathBuf,

        /// The underlying filesystem failure.
        source: io::Error,
    },

    /// The file is not JSON.
    #[error("{path} is not valid JSON: {source}")]
    Malformed {
        /// The output file.
        path: PathBuf,

        /// The parse failure, with its position in the file.
        source: serde_json::Error,
    },

    /// The output is JSON, but not the array of entities a run produces.
    #[error("{target} does not hold a JSON array of entities")]
    NotAnEntityArray {
        /// What was checked.
        target: AssertionTarget,
    },

    /// An assertion is about a broker, and the example declares no `[context-broker]` block.
    #[error("{target} cannot be checked: the example declares no [context-broker] block to query")]
    NoBroker {
        /// What was checked.
        target: AssertionTarget,
    },

    /// The broker could not be asked what it holds.
    #[error(transparent)]
    Broker(#[from] BrokerError),

    /// The run produced a different number of entities than the example states.
    #[error("{target} holds {found} entities, expected {expected}")]
    WrongEntityCount {
        /// What was checked.
        target: AssertionTarget,

        /// What the descriptor states.
        expected: usize,

        /// What the run produced.
        found: usize,
    },

    /// An example asserts nothing about what it produces.
    #[error("{name} declares no [[outputs]] to check")]
    NoAssertions {
        /// The example's directory name.
        name: ExampleName,
    },

    /// A source produced fewer entities than the example says it must.
    #[error("{target} holds {found} entities, expected at least {fewest}")]
    TooFewEntities {
        /// What was checked.
        target: AssertionTarget,

        /// The floor the descriptor states.
        fewest: usize,

        /// What the run produced.
        found: usize,
    },

    /// An entity the page quotes is not in the output.
    #[error("{target} does not hold {declared}")]
    MissingEntity {
        /// What was checked.
        target: AssertionTarget,

        /// The identifier that was expected.
        declared: EntityId,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        example_name::ExampleName,
        output::Output,
        verification::{VerificationError, assess, require_assertions},
        volatility::Volatility,
    };
    use serde_json::{Value, json};

    fn name() -> Option<ExampleName> {
        ExampleName::try_from("09-csv-manifest".to_owned()).ok()
    }

    fn output(text: &str) -> Option<Output> {
        toml::from_str(text).ok()
    }

    fn exact(entities: usize, contains: &[&str]) -> Option<Output> {
        let quoted: Vec<String> = contains.iter().map(|value| format!("\"{value}\"")).collect();

        output(&format!(
            "file = \"out/ChemicalElement.json\"\nentities = {entities}\ncontains = [{}]\n",
            quoted.join(", ")
        ))
    }

    fn entities(count: usize) -> Vec<Value> {
        (0..count)
            .map(|ordinal| json!({ "id": format!("urn:ngsi-ld:ChemicalElement:{ordinal}") }))
            .collect()
    }

    fn identified(ids: &[&str]) -> Vec<Value> {
        ids.iter().map(|id| json!({ "id": id })).collect()
    }

    fn checked(found: &[Value], declared: Option<Output>, volatility: Volatility) -> Option<Result<bool, VerificationError>> {
        let name = name()?;

        declared.map(|declared| assess(&name, found, &declared, volatility).map(|log| log.is_empty()))
    }

    #[test]
    fn a_matching_document_passes_with_nothing_to_report() {
        let found = identified(&["urn:ngsi-ld:ChemicalElement:H", "urn:ngsi-ld:ChemicalElement:He"]);

        assert_eq!(
            checked(&found, exact(2, &["urn:ngsi-ld:ChemicalElement:H"]), Volatility::Frozen).and_then(Result::ok),
            Some(true)
        );
    }

    #[test]
    fn a_frozen_source_that_counts_differently_is_a_failure() {
        let outcome = checked(&entities(1), exact(118, &[]), Volatility::Frozen);

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(VerificationError::WrongEntityCount { expected: 118, found: 1, .. }))));
    }

    #[test]
    fn a_live_source_that_counts_a_little_differently_is_reported_and_passes() {
        let outcome = checked(&entities(120), exact(118, &[]), Volatility::Live);

        assert_eq!(outcome.and_then(Result::ok), Some(false));
    }

    #[test]
    fn a_live_source_that_counts_far_differently_is_a_failure() {
        let outcome = checked(&entities(1), exact(118, &[]), Volatility::Live);

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(VerificationError::WrongEntityCount { .. }))));
    }

    #[test]
    fn a_quoted_entity_that_is_absent_from_a_frozen_source_is_a_failure() {
        let found = identified(&["urn:ngsi-ld:ChemicalElement:He"]);
        let outcome = checked(&found, exact(1, &["urn:ngsi-ld:ChemicalElement:H"]), Volatility::Frozen);

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(VerificationError::MissingEntity { .. }))));
    }

    #[test]
    fn a_quoted_entity_that_is_absent_from_a_live_source_is_reported_and_passes() {
        let found = identified(&["urn:ngsi-ld:ChemicalElement:He"]);
        let outcome = checked(&found, exact(1, &["urn:ngsi-ld:ChemicalElement:H"]), Volatility::Live);

        assert_eq!(outcome.and_then(Result::ok), Some(false));
    }

    #[test]
    fn a_live_feed_passes_when_it_clears_its_floor() {
        let declared = output("file = \"out/ChemicalElement.json\"\nat-least = 1\n");

        assert_eq!(checked(&entities(2), declared, Volatility::Live).and_then(Result::ok), Some(true));
    }

    #[test]
    fn a_live_feed_a_little_below_its_floor_is_reported_and_passes() {
        let declared = output("file = \"out/ChemicalElement.json\"\nat-least = 114\n");

        assert_eq!(checked(&entities(100), declared, Volatility::Live).and_then(Result::ok), Some(false));
    }

    #[test]
    fn a_live_feed_far_below_its_floor_is_a_failure() {
        let declared = output("file = \"out/ChemicalElement.json\"\nat-least = 114\n");
        let outcome = checked(&entities(10), declared, Volatility::Live);

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(VerificationError::TooFewEntities { .. }))));
    }

    #[test]
    fn a_frozen_feed_below_its_floor_is_a_failure() {
        let declared = output("file = \"out/ChemicalElement.json\"\nat-least = 1\n");
        let outcome = checked(&[], declared, Volatility::Frozen);

        assert!(outcome.is_some_and(|outcome| matches!(outcome, Err(VerificationError::TooFewEntities { .. }))));
    }

    #[test]
    fn an_example_without_assertions_is_reported() {
        let checked = name().map(|name| require_assertions(&name, &[]));

        assert!(checked.is_some_and(|outcome| matches!(outcome, Err(VerificationError::NoAssertions { .. }))));
    }

    #[test]
    fn an_example_with_assertions_passes() {
        let checked = name().zip(exact(1, &[])).map(|(name, declared)| {
            let outputs = [declared];

            require_assertions(&name, &outputs)
        });

        assert!(checked.is_some_and(|outcome| outcome.is_ok()));
    }
}
