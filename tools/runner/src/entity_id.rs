use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The prefix every NGSI-LD entity identifier carries (ETSI GS CIM 009 v1.9.1, clause 4.5.1).
const URN_PREFIX: &str = "urn:ngsi-ld:";

/// An NGSI-LD entity identifier an example expects to find in its output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct EntityId(String);

impl EntityId {
    /// The identifier as it appears in the entity's `id` member.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EntityId {
    type Error = EntityIdError;

    fn try_from(value: String) -> Result<EntityId, EntityIdError> {
        if value.starts_with(URN_PREFIX) {
            Ok(EntityId(value))
        } else {
            Err(EntityIdError::NotAnEntityUrn { value })
        }
    }
}

/// Why a string could not be read as an entity identifier.
#[derive(Debug, Error)]
pub enum EntityIdError {
    /// The value does not carry the NGSI-LD URN prefix.
    #[error("{value} is not an NGSI-LD entity identifier, expected one starting with {URN_PREFIX}")]
    NotAnEntityUrn {
        /// The value as it was written.
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use crate::entity_id::{EntityId, EntityIdError};
    use serde::Deserialize;

    /// The shape an identifier is read in, since a bare string is not a TOML document.
    #[derive(Debug, Deserialize)]
    struct Assertion {
        contains: Vec<EntityId>,
    }

    #[test]
    fn an_ngsi_ld_urn_is_accepted() {
        assert_eq!(
            EntityId::try_from("urn:ngsi-ld:ChemicalElement:H".to_owned())
                .map(|id| id.as_str().to_owned())
                .ok(),
            Some("urn:ngsi-ld:ChemicalElement:H".to_owned())
        );
    }

    #[test]
    fn a_urn_of_another_namespace_is_rejected() {
        assert!(matches!(
            EntityId::try_from("urn:uuid:0f1e2d3c".to_owned()),
            Err(EntityIdError::NotAnEntityUrn { .. })
        ));
    }

    #[test]
    fn a_bare_name_is_rejected() {
        assert!(matches!(EntityId::try_from("H".to_owned()), Err(EntityIdError::NotAnEntityUrn { .. })));
    }

    #[test]
    fn identifiers_are_validated_as_a_descriptor_is_read() {
        let assertion: Result<Assertion, _> = toml::from_str("contains = [\"urn:ngsi-ld:ChemicalElement:H\"]\n");

        assert_eq!(assertion.map(|assertion| assertion.contains.len()).ok(), Some(1));
    }

    #[test]
    fn a_descriptor_quoting_a_bare_name_is_rejected_as_it_is_read() {
        let assertion: Result<Assertion, _> = toml::from_str("contains = [\"H\"]\n");

        assert!(assertion.is_err());
    }
}
