use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::GraphError;

#[derive(Debug, Deserialize)]
struct CorpusManifestFile {
    schema_version: serde_json::Value,
    edition: String,
    last_update: String,
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: String,
    pub table: String,
    pub fields: BTreeMap<String, String>,
    pub refs: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Deserialize)]
struct EntityFile {
    id: String,
    table: String,
    fields: BTreeMap<String, String>,
    refs: BTreeMap<String, Option<String>>,
}

pub struct Corpus {
    pub edition: String,
    pub last_update: String,
    pub entities: Vec<Entity>,
    pub fingerprint: String,
}

pub fn load_corpus(dir: &Path) -> Result<Corpus, GraphError> {
    let manifest_path = dir.join("manifest.json");
    let entities_path = dir.join("entities.jsonl");
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|err| GraphError::read(&manifest_path, err))?;
    let manifest: CorpusManifestFile = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| GraphError::read(&manifest_path, format!("malformed JSON: {err}")))?;
    if manifest.schema_version.as_u64() != Some(1) {
        return Err(GraphError::read(
            &manifest_path,
            format!(
                "schema_version is not 1 (found {})",
                manifest.schema_version
            ),
        ));
    }
    let entity_bytes =
        fs::read(&entities_path).map_err(|err| GraphError::read(&entities_path, err))?;
    let entities = parse_entities(&entities_path, &entity_bytes)?;
    Ok(Corpus {
        edition: manifest.edition,
        last_update: manifest.last_update,
        entities,
        fingerprint: sha256_hex(&entity_bytes),
    })
}

fn parse_entities(path: &Path, bytes: &[u8]) -> Result<Vec<Entity>, GraphError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| GraphError::read(path, format!("not UTF-8: {err}")))?;
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    let mut entities = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            return Err(GraphError::read(path, format!("blank line {}", index + 1)));
        }
        let parsed: EntityFile = serde_json::from_str(line).map_err(|err| {
            GraphError::read(path, format!("malformed JSON on line {}: {err}", index + 1))
        })?;
        entities.push(Entity {
            id: parsed.id,
            table: parsed.table,
            fields: parsed.fields,
            refs: parsed.refs,
        });
    }
    Ok(entities)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

pub fn field<'a>(entity: &'a Entity, name: &str) -> &'a str {
    entity.fields.get(name).map(String::as_str).unwrap_or("")
}

pub fn ref_id<'a>(entity: &'a Entity, name: &str) -> Option<&'a str> {
    match entity.refs.get(name) {
        Some(Some(value)) if !value.is_empty() => Some(value.as_str()),
        _ => None,
    }
}
