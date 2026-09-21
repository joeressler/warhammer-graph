use std::collections::BTreeMap;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

/// Node and edge attributes.
///
/// JSONL writes a JSON object whose values are strings or arrays of strings.
/// Postcard is not self-describing, so the binary form is an explicit list of
/// tagged values that decodes back to that same object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attrs(BTreeMap<String, AttrValue>);

#[derive(Clone, Debug, PartialEq, Eq)]
enum AttrValue {
    Text(String),
    List(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Packed {
    Text(String),
    List(Vec<String>),
}

impl Attrs {
    pub fn insert_text(&mut self, key: &str, value: impl Into<String>) {
        self.0
            .insert(key.to_string(), AttrValue::Text(value.into()));
    }

    pub fn insert_list(&mut self, key: &str, values: Vec<String>) {
        self.0.insert(key.to_string(), AttrValue::List(values));
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        match self.0.get(key) {
            Some(AttrValue::Text(value)) => Some(value.as_str()),
            _ => None,
        }
    }
}

impl Serialize for Attrs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut map = serializer.serialize_map(Some(self.0.len()))?;
            for (key, value) in &self.0 {
                match value {
                    AttrValue::Text(text) => map.serialize_entry(key, text)?,
                    AttrValue::List(items) => map.serialize_entry(key, items)?,
                }
            }
            map.end()
        } else {
            let packed: Vec<(String, Packed)> = self
                .0
                .iter()
                .map(|(key, value)| {
                    let packed = match value {
                        AttrValue::Text(text) => Packed::Text(text.clone()),
                        AttrValue::List(items) => Packed::List(items.clone()),
                    };
                    (key.clone(), packed)
                })
                .collect();
            packed.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Attrs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(AttrsVisitor)
        } else {
            let packed = Vec::<(String, Packed)>::deserialize(deserializer)?;
            let mut attrs = Attrs::default();
            for (key, value) in packed {
                let stored = match value {
                    Packed::Text(text) => AttrValue::Text(text),
                    Packed::List(items) => AttrValue::List(items),
                };
                attrs.0.insert(key, stored);
            }
            Ok(attrs)
        }
    }
}

struct AttrsVisitor;

impl<'de> Visitor<'de> for AttrsVisitor {
    type Value = Attrs;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object of strings or arrays of strings")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut attrs = Attrs::default();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value::<serde_json::Value>()?;
            let stored = match value {
                serde_json::Value::String(text) => AttrValue::Text(text),
                serde_json::Value::Array(items) => {
                    let mut strings = Vec::with_capacity(items.len());
                    for item in items {
                        match item {
                            serde_json::Value::String(text) => strings.push(text),
                            other => {
                                return Err(de::Error::custom(format!(
                                    "attribute array values must be strings, got {other}"
                                )));
                            }
                        }
                    }
                    AttrValue::List(strings)
                }
                other => {
                    return Err(de::Error::custom(format!(
                        "attribute values must be strings or arrays of strings, got {other}"
                    )));
                }
            };
            attrs.0.insert(key, stored);
        }
        Ok(attrs)
    }
}
