use crate::{assertion_target::AssertionTarget, entity_id::EntityId, example_name::ExampleName};
use derive_more::Display;
use serde::Serialize;

/// One difference a run found that did not stop it.
///
/// A drift is a third outcome beside passing and failing, not a kind of failure: the example still
/// works, and the publisher has moved under it. Keeping it out of
/// [`crate::verification::VerificationError`] is what stops a matrix from turning red for three new
/// airports and training a reader to ignore red.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize)]
#[display("{example}: {target} {kind}")]
#[serde(rename_all = "kebab-case")]
pub struct Drift {
    /// The example the run belongs to.
    pub example: ExampleName,

    /// What was checked.
    pub target: AssertionTarget,

    /// What no longer matches.
    pub kind: DriftKind,
}

/// What a drift is about, with the figure the example declares beside what the run found.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum DriftKind {
    /// The run produced a different number of entities than the example declares.
    #[display("holds {found} entities, and the example declares {declared}")]
    EntityCount {
        /// What the descriptor states.
        declared: usize,

        /// What the run produced.
        found: usize,
    },

    /// An identifier the page quotes is no longer among the entities.
    #[display("no longer holds {declared}")]
    MissingEntity {
        /// The identifier the descriptor names.
        declared: EntityId,
    },
}

/// Every drift one run found, in the order the assertions were checked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Display, Serialize)]
#[display("{}", _0.iter().map(Drift::to_string).collect::<Vec<String>>().join("\n"))]
#[serde(transparent)]
pub struct DriftLog(Vec<Drift>);

impl DriftLog {
    /// Adds one finding.
    pub fn record(&mut self, drift: Drift) {
        self.0.push(drift);
    }

    /// Adds everything another log holds, leaving that log consumed.
    pub fn absorb(&mut self, other: DriftLog) {
        self.0.extend(other.0);
    }

    /// Whether the run found nothing worth reporting.
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assertion_target::AssertionTarget,
        drift::{Drift, DriftKind, DriftLog},
        entity_id::EntityId,
        example_name::ExampleName,
    };
    use std::path::PathBuf;

    fn drift(kind: DriftKind) -> Option<Drift> {
        Some(Drift {
            example: ExampleName::try_from("09-csv-manifest".to_owned()).ok()?,
            target: AssertionTarget::File(PathBuf::from("out/Airport.json")),
            kind,
        })
    }

    fn count() -> Option<Drift> {
        drift(DriftKind::EntityCount { declared: 7000, found: 7698 })
    }

    #[test]
    fn a_count_drift_reads_as_the_example_the_file_and_both_figures() {
        assert_eq!(
            count().map(|drift| drift.to_string()),
            Some("09-csv-manifest: out/Airport.json holds 7698 entities, and the example declares 7000".to_owned())
        );
    }

    #[test]
    fn a_missing_entity_drift_names_the_identifier() {
        let absent = EntityId::try_from("urn:ngsi-ld:Airport:1".to_owned())
            .ok()
            .and_then(|declared| drift(DriftKind::MissingEntity { declared }));

        assert_eq!(
            absent.map(|drift| drift.to_string()),
            Some("09-csv-manifest: out/Airport.json no longer holds urn:ngsi-ld:Airport:1".to_owned())
        );
    }

    #[test]
    fn a_log_with_nothing_in_it_is_empty() {
        assert!(DriftLog::default().is_empty());
    }

    #[test]
    fn a_recorded_finding_makes_the_log_report_it() {
        let log = count().map(|drift| {
            let mut log = DriftLog::default();
            log.record(drift);

            log
        });

        assert!(log.is_some_and(|log| !log.is_empty() && log.to_string().contains("7698")));
    }

    #[test]
    fn absorbing_another_log_keeps_both_findings() {
        let log = count().zip(count()).map(|(first, second)| {
            let mut log = DriftLog::default();
            log.record(first);

            let mut absorbed = DriftLog::default();
            absorbed.record(second);
            log.absorb(absorbed);

            log
        });

        assert!(log.is_some_and(|log| log.to_string().lines().count() == 2));
    }

    #[test]
    fn a_log_is_encoded_as_a_plain_array_of_findings() {
        let encoded = count().and_then(|drift| {
            let mut log = DriftLog::default();
            log.record(drift);

            serde_json::to_string(&log).ok()
        });

        assert_eq!(
            encoded,
            Some(
                "[{\"example\":\"09-csv-manifest\",\"target\":{\"file\":\"out/Airport.json\"},\"kind\":{\"kind\":\"entity-count\",\"declared\":7000,\"found\":7698}}]"
                    .to_owned()
            )
        );
    }
}
