//! Lossless JSON normalization at the untrusted native-model text boundary.

use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

// serde_json::Value alone silently overwrites duplicate keys. Reject them at every
// depth before canonicalizing whitespace/key order; schema validation is downstream.
struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|value| UniqueValue(Value::Number(value)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.to_owned())))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    let UniqueValue(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

pub(crate) fn canonical_object(bytes: &[u8]) -> Result<Vec<u8>, serde_json::Error> {
    let UniqueValue(value) = serde_json::from_slice(bytes)?;
    if !value.is_object() {
        return Err(<serde_json::Error as de::Error>::custom(
            "expected JSON object",
        ));
    }
    serde_json::to_vec(&value)
}

/// Presentation-only types; native JSON schemas and validators remain authoritative.
pub(crate) fn parameter_type(schema: &Value) -> Result<String, ()> {
    fn render(value: &Value, root: &Value, depth: u8) -> Result<String, ()> {
        if depth > 32 {
            return Err(());
        }
        if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
            return render(
                root.pointer(reference.strip_prefix('#').ok_or(())?)
                    .ok_or(())?,
                root,
                depth + 1,
            );
        }
        if let Some(constant) = value.get("const") {
            return Ok(constant.to_string());
        }
        if let Some(variants) = value.get("enum").and_then(Value::as_array) {
            return Ok(variants
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(" | "));
        }
        if let Some(variants) = value.get("oneOf").and_then(Value::as_array) {
            return Ok(variants
                .iter()
                .map(|v| render(v, root, depth + 1))
                .collect::<Result<Vec<_>, _>>()?
                .join(" | "));
        }
        if let Some(types) = value.get("type").and_then(Value::as_array) {
            return Ok(types
                .iter()
                .map(|kind| {
                    let mut variant = value.clone();
                    variant["type"] = kind.clone();
                    render(&variant, root, depth + 1)
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(" | "));
        }
        match value.get("type").and_then(Value::as_str) {
            Some("string") => Ok("string".to_owned()),
            Some("integer" | "number") => Ok("number".to_owned()),
            Some("boolean") => Ok("boolean".to_owned()),
            Some("null") => Ok("null".to_owned()),
            Some("array") => Ok(format!(
                "({})[]",
                render(value.get("items").ok_or(())?, root, depth + 1)?
            )),
            Some("object") => {
                let Some(properties) = value.get("properties").and_then(Value::as_object) else {
                    return Ok("object".to_owned());
                };
                let fields = properties
                    .iter()
                    .map(|(name, spec)| {
                        let required = value
                            .get("required")
                            .and_then(Value::as_array)
                            .is_some_and(|names| names.iter().any(|v| v.as_str() == Some(name)));
                        Ok(format!(
                            "{}{}: {}",
                            serde_json::to_string(name).map_err(|_| ())?,
                            if required { "" } else { "?" },
                            render(spec, root, depth + 1)?
                        ))
                    })
                    .collect::<Result<Vec<_>, ()>>()?;
                Ok(format!("{{ {} }}", fields.join(", ")))
            }
            _ => Err(()),
        }
    }
    render(schema, schema, 0)
}

pub(crate) fn tool_alias(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::canonical_object;

    #[test]
    fn codec_json_normalizes_syntax_without_repairing_or_discarding_fields() {
        assert_eq!(
            canonical_object(br#" { "z": [true,null,1], "a": "x" } "#).unwrap(),
            br#"{"a":"x","z":[true,null,1]}"#
        );
        for invalid in [
            br#"={"a":1}"#.as_slice(),
            br#"{"a":1,"a":2}"#,
            br#"{"x":[{"a":1,"\u0061":2}]}"#,
            br#"{"a":1} trailing"#,
            br#"{"a":1"#,
            b"[]",
            b"null",
        ] {
            assert!(canonical_object(invalid).is_err(), "{invalid:?}");
        }
    }
}
