//! Stable construction project-control identities.

use std::fmt;

use crate::{ConstructionProjectError, Result};

const MAX_ID_CHARS: usize = 96;
const RESERVED_IDS: &[&str] = &[
    ".", "..", "-", "_", "none", "null", "nil", "system", "unknown",
];

macro_rules! id_type {
    ($name:ident, $kind:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Validates and builds the stable identifier.
            pub fn new(id: impl Into<String>) -> Result<Self> {
                let id = id.into();
                validate_id($kind, &id)?;
                Ok(Self(id))
            }

            /// Borrows the identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

id_type!(
    ProjectId,
    "project",
    "Stable construction project identifier."
);
id_type!(
    ControlId,
    "control",
    "Stable construction control identifier."
);
id_type!(RoleId, "role", "Stable accountable role identifier.");
id_type!(
    OrganizationId,
    "organization",
    "Stable organization identifier for project governance records."
);
id_type!(
    BaselineId,
    "baseline",
    "Stable project baseline identifier for accepted control states."
);

fn validate_id(kind: &'static str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(ConstructionProjectError::EmptyId { kind });
    }
    if value.chars().count() > MAX_ID_CHARS {
        return Err(ConstructionProjectError::IdTooLong {
            kind,
            value: value.to_owned(),
            max: MAX_ID_CHARS,
        });
    }
    if value.trim() != value || value.chars().any(char::is_whitespace) {
        return Err(ConstructionProjectError::WhitespaceAmbiguousId {
            kind,
            value: value.to_owned(),
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(ConstructionProjectError::SlashBearingId {
            kind,
            value: value.to_owned(),
        });
    }
    let lower = value.to_ascii_lowercase();
    if RESERVED_IDS.iter().any(|reserved| *reserved == lower) {
        return Err(ConstructionProjectError::ReservedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(ConstructionProjectError::UnsupportedIdCharacters {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
