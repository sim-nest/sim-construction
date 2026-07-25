//! Serde bridge for open outcome symbols.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use sim_kernel::Symbol;

pub(crate) fn serialize<S>(symbol: &Symbol, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    symbol.as_qualified_str().serialize(serializer)
}

pub(crate) fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Symbol, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if let Some((namespace, name)) = value.split_once('/') {
        if namespace.is_empty() || name.is_empty() || name.contains('/') {
            return Err(D::Error::custom("invalid qualified symbol"));
        }
        Ok(Symbol::qualified(namespace.to_owned(), name.to_owned()))
    } else {
        Symbol::checked(value).map_err(D::Error::custom)
    }
}
