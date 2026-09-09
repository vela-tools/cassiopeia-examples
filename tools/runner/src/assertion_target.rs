use crate::{data_model_name::DataModelName, entity_id::EntityId};
use derive_more::Display;
use serde::Serialize;
use std::path::PathBuf;

/// Where an assertion looks for the entities it checks.
///
/// A run either writes files or delivers to a broker, and both need the same claims made about
/// them: how many entities there are, and that the ones a page quotes are among them. Naming the
/// place as its own value keeps those claims in one shape instead of giving every assertion a file
/// it may not have.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssertionTarget {
    /// A JSON array the run wrote, relative to the example directory.
    #[display("{}", _0.display())]
    File(PathBuf),

    /// Every entity of one type a broker holds.
    #[display("the broker's {_0} entities")]
    BrokerEntities(DataModelName),

    /// One folded `EntityTemporal` a broker holds at its temporal endpoint, which is where a series
    /// representation belongs (ETSI GS CIM 009 v1.9.1, clause 5.2.20).
    #[display("{_0} at the broker's temporal endpoint")]
    BrokerTemporal(EntityId),
}

#[cfg(test)]
mod tests {
    use crate::{assertion_target::AssertionTarget, data_model_name::DataModelName, entity_id::EntityId};
    use std::path::PathBuf;

    #[test]
    fn a_file_target_reads_as_its_path() {
        assert_eq!(AssertionTarget::File(PathBuf::from("out/City.json")).to_string(), "out/City.json");
    }

    #[test]
    fn a_broker_entity_target_names_the_model() {
        let target = DataModelName::try_from("AirQualityObserved".to_owned()).map(AssertionTarget::BrokerEntities);

        assert_eq!(
            target.map(|target| target.to_string()).ok(),
            Some("the broker's AirQualityObserved entities".to_owned())
        );
    }

    #[test]
    fn a_broker_temporal_target_names_the_entity_and_the_endpoint() {
        let target = EntityId::try_from("urn:ngsi-ld:TropicalCyclone:AL122005".to_owned()).map(AssertionTarget::BrokerTemporal);

        assert_eq!(
            target.map(|target| target.to_string()).ok(),
            Some("urn:ngsi-ld:TropicalCyclone:AL122005 at the broker's temporal endpoint".to_owned())
        );
    }

    #[test]
    fn a_target_is_encoded_under_a_kebab_case_name() {
        let encoded = serde_json::to_string(&AssertionTarget::File(PathBuf::from("out/City.json")));

        assert_eq!(encoded.ok(), Some("{\"file\":\"out/City.json\"}".to_owned()));
    }
}
