//! Tolerant wire adapter for declared work purpose; never stores prompt text.

use op_editor_core::HomeFamily;
use serde::{Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<S: Serializer>(
    family: &Option<HomeFamily>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match family {
        Some(family) => serializer.serialize_str(family.id()),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<HomeFamily>, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(id) => HomeFamily::from_id(&id),
        _ => None,
    })
}
