use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::bundle::{hex_encode, sha256, sha256_file, LoadedBundle};
use crate::error::AskError;
use crate::model::{l2_normalize, EmbedIdentity, Embedder};

const MAGIC: &[u8; 6] = b"WHASK1";
const HEADER_LEN: usize = 6 + 4 + 4 + 32 + 32 + 32 + 64 + 8 + 8;

struct Header {
    dimension: u32,
    count: u32,
    passage_sha: [u8; 32],
    model_sha: [u8; 32],
    nodes_sha: [u8; 32],
    fingerprint: [u8; 64],
    model_len: u64,
    model_mtime: u64,
}

/// Resolve the embedding file's hash.
///
/// The header caches the hash. When the file length and mtime are unchanged,
/// that cached hash is reused. When either changes, the file is hashed again
/// before a cached index is reused.
pub fn model_identity(
    config_dir: &Path,
    bundle_dir: &Path,
    model_path: &Path,
) -> Result<EmbedIdentity, AskError> {
    if !model_path.is_file() {
        return Err(AskError::model(format!(
            "{}: embedding GGUF failed to load",
            model_path.display()
        )));
    }
    let meta = fs::metadata(model_path)
        .map_err(|err| AskError::model(format!("{}: {err}", model_path.display())))?;
    let len = meta.len();
    let mtime_secs = mtime_secs(&meta);
    let passages = fs::read(bundle_dir.join("passages.jsonl"))
        .map_err(|err| AskError::bundle(&bundle_dir.join("passages.jsonl"), err))?;
    let passage_sha = sha256(&passages);
    if let Some(header) = find_header_by_file(config_dir, &passage_sha, len, mtime_secs) {
        return Ok(EmbedIdentity {
            sha256: header.model_sha,
            len,
            mtime_secs,
        });
    }
    Ok(EmbedIdentity {
        sha256: sha256_file(model_path)?,
        len,
        mtime_secs,
    })
}

pub fn load_or_build(
    config_dir: &Path,
    bundle: &LoadedBundle,
    identity: &EmbedIdentity,
    embedder: &mut dyn Embedder,
) -> Result<Vec<Vec<f32>>, AskError> {
    let path = index_path(config_dir, &bundle.passages_sha256, &identity.sha256);
    if let Some(stored) = read_index(&path) {
        if header_matches(&stored.0, bundle, identity, embedder.dimension())
            && records_match(&stored.1, bundle)
        {
            return Ok(stored.1.into_iter().map(|(_, vector)| vector).collect());
        }
    }
    let mut vectors = Vec::with_capacity(bundle.passages.len());
    let dimension = embedder.dimension() as usize;
    for passage in &bundle.passages {
        let mut vector = embedder.embed_passage(&passage.node_id, &passage.text)?;
        if vector.len() != dimension {
            return Err(AskError::model(
                "embedding dimension does not match the model",
            ));
        }
        l2_normalize(&mut vector);
        vectors.push((passage.node_id.clone(), vector));
    }
    write_index(&path, bundle, identity, embedder.dimension(), &vectors)?;
    Ok(vectors.into_iter().map(|(_, vector)| vector).collect())
}

fn header_matches(
    header: &Header,
    bundle: &LoadedBundle,
    identity: &EmbedIdentity,
    dimension: u32,
) -> bool {
    header.dimension == dimension
        && header.count == bundle.passages.len() as u32
        && header.passage_sha == bundle.passages_sha256
        && header.nodes_sha == bundle.nodes_sha256
        && header.fingerprint == bundle.fingerprint_bytes()
        && header.model_sha == identity.sha256
}

fn records_match(records: &[(String, Vec<f32>)], bundle: &LoadedBundle) -> bool {
    records.len() == bundle.passages.len()
        && records
            .iter()
            .zip(bundle.passages.iter())
            .all(|(record, passage)| record.0 == passage.node_id)
}

fn index_path(config_dir: &Path, passage_sha: &[u8; 32], model_sha: &[u8; 32]) -> PathBuf {
    config_dir
        .join("indexes")
        .join(hex_encode(passage_sha))
        .join(hex_encode(model_sha))
        .join("index.bin")
}

fn find_header_by_file(
    config_dir: &Path,
    passage_sha: &[u8; 32],
    len: u64,
    mtime_secs: u64,
) -> Option<Header> {
    let dir = config_dir.join("indexes").join(hex_encode(passage_sha));
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path().join("index.bin");
        let Some((header, _)) = read_index(&path) else {
            continue;
        };
        if header.model_len == len && header.model_mtime == mtime_secs {
            return Some(header);
        }
    }
    None
}

fn read_index(path: &Path) -> Option<(Header, Vec<(String, Vec<f32>)>)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < HEADER_LEN || &bytes[..6] != MAGIC {
        return None;
    }
    let dimension = u32::from_le_bytes(bytes[6..10].try_into().ok()?);
    let count = u32::from_le_bytes(bytes[10..14].try_into().ok()?);
    let mut passage_sha = [0u8; 32];
    passage_sha.copy_from_slice(&bytes[14..46]);
    let mut model_sha = [0u8; 32];
    model_sha.copy_from_slice(&bytes[46..78]);
    let mut nodes_sha = [0u8; 32];
    nodes_sha.copy_from_slice(&bytes[78..110]);
    let mut fingerprint = [0u8; 64];
    fingerprint.copy_from_slice(&bytes[110..174]);
    let model_len = u64::from_le_bytes(bytes[174..182].try_into().ok()?);
    let model_mtime = u64::from_le_bytes(bytes[182..190].try_into().ok()?);
    let header = Header {
        dimension,
        count,
        passage_sha,
        model_sha,
        nodes_sha,
        fingerprint,
        model_len,
        model_mtime,
    };
    let mut cursor = HEADER_LEN;
    let mut records = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if cursor + 4 > bytes.len() {
            return None;
        }
        let id_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?) as usize;
        cursor += 4;
        if cursor + id_len > bytes.len() {
            return None;
        }
        let node_id = std::str::from_utf8(&bytes[cursor..cursor + id_len])
            .ok()?
            .to_string();
        cursor += id_len;
        let vector_bytes = dimension as usize * 4;
        if cursor + vector_bytes > bytes.len() {
            return None;
        }
        let mut vector = Vec::with_capacity(dimension as usize);
        for _ in 0..dimension {
            let value = f32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
            vector.push(value);
            cursor += 4;
        }
        records.push((node_id, vector));
    }
    Some((header, records))
}

fn write_index(
    path: &Path,
    bundle: &LoadedBundle,
    identity: &EmbedIdentity,
    dimension: u32,
    records: &[(String, Vec<f32>)],
) -> Result<(), AskError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| AskError::model(format!("{}: {err}", parent.display())))?;
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(dimension).to_le_bytes());
    bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&bundle.passages_sha256);
    bytes.extend_from_slice(&identity.sha256);
    bytes.extend_from_slice(&bundle.nodes_sha256);
    bytes.extend_from_slice(&bundle.fingerprint_bytes());
    bytes.extend_from_slice(&identity.len.to_le_bytes());
    bytes.extend_from_slice(&identity.mtime_secs.to_le_bytes());
    for (node_id, vector) in records {
        let id_bytes = node_id.as_bytes();
        bytes.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(id_bytes);
        for value in vector {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    let temp = path.with_extension("bin.tmp");
    fs::write(&temp, &bytes)
        .map_err(|err| AskError::model(format!("{}: {err}", temp.display())))?;
    fs::rename(&temp, path).map_err(|err| AskError::model(format!("{}: {err}", path.display())))?;
    Ok(())
}

fn mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
