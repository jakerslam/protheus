use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct DbIndexEntry {
    pub node_id: String,
    pub uid: String,
    pub file_rel: String,
    pub summary: String,
    pub tags: Vec<String>,
}

pub struct MemoryDb {
    conn: Connection,
    db_path: PathBuf,
    cipher_key: [u8; 32],
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn default_db_path(root: &Path) -> PathBuf {
    root.join("state")
        .join("memory")
        .join("runtime_memory.sqlite")
}

fn resolve_db_path(root: &Path, raw: &str) -> PathBuf {
    let v = raw.trim();
    if v.is_empty() {
        return default_db_path(root);
    }
    let p = PathBuf::from(v);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn derive_key_material(root: &Path) -> String {
    if let Ok(v) = env::var("PROTHEUS_MEMORY_DB_KEY") {
        let trimmed = v.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let keyring_path = root
        .join("state")
        .join("security")
        .join("organ_state_encryption")
        .join("keyring.json");
    if let Ok(raw) = fs::read_to_string(&keyring_path) {
        if !raw.trim().is_empty() {
            return raw;
        }
    }
    format!(
        "fallback:{}:memory-runtime-db",
        root.to_string_lossy()
    )
}

fn derive_cipher_key(root: &Path) -> [u8; 32] {
    let material = derive_key_material(root);
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    out
}

fn keystream_block(cipher_key: &[u8; 32], nonce: u64, block_index: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(cipher_key);
    hasher.update(nonce.to_le_bytes());
    hasher.update(block_index.to_le_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    out
}

fn xor_stream(cipher_key: &[u8; 32], nonce: u64, input: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; input.len()];
    let mut offset = 0usize;
    let mut block_index = 0u64;
    while offset < input.len() {
        let block = keystream_block(cipher_key, nonce, block_index);
        let mut local = 0usize;
        while local < block.len() && (offset + local) < input.len() {
            out[offset + local] = input[offset + local] ^ block[local];
            local += 1;
        }
        offset += block.len();
        block_index += 1;
    }
    out
}

fn encrypt_value(cipher_key: &[u8; 32], plaintext: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64);
    let cipher = xor_stream(cipher_key, nonce, plaintext.as_bytes());
    format!("enc-v1:{nonce:016x}:{}", hex::encode(cipher))
}

fn decrypt_value(cipher_key: &[u8; 32], payload: &str) -> String {
    let body = payload.trim();
    if !body.starts_with("enc-v1:") {
        return body.to_string();
    }
    let parts = body.splitn(3, ':').collect::<Vec<&str>>();
    if parts.len() != 3 {
        return body.to_string();
    }
    let nonce = u64::from_str_radix(parts[1], 16).unwrap_or(0);
    let Ok(cipher_bytes) = hex::decode(parts[2]) else {
        return body.to_string();
    };
    let plain = xor_stream(cipher_key, nonce, &cipher_bytes);
    String::from_utf8(plain).unwrap_or_else(|_| body.to_string())
}

fn parse_tags_json(raw: &str) -> Vec<String> {
    let parsed = serde_json::from_str::<Vec<String>>(raw).unwrap_or_default();
    let mut out = parsed
        .into_iter()
        .filter(|tag| !tag.trim().is_empty())
        .collect::<Vec<String>>();
    out.sort();
    out.dedup();
    out
}

impl MemoryDb {
    pub fn open(root: &Path, db_path_raw: &str) -> Result<Self, String> {
        let db_path = resolve_db_path(root, db_path_raw);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("db_parent_create_failed:{err}"))?;
        }
        let conn = Connection::open(&db_path).map_err(|err| format!("db_open_failed:{err}"))?;
        let db = Self {
            conn,
            db_path,
            cipher_key: derive_cipher_key(root),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn rel_db_path(&self, root: &Path) -> String {
        self.db_path
            .strip_prefix(root)
            .unwrap_or(&self.db_path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                r#"
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=3000;

CREATE TABLE IF NOT EXISTS embeddings (
  embedding_id TEXT PRIMARY KEY,
  node_id TEXT NOT NULL,
  vector_blob BLOB NOT NULL,
  metadata_json TEXT NOT NULL,
  created_ts TEXT NOT NULL,
  updated_ts TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS temporal_graph_nodes (
  node_id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  updated_ts TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS temporal_graph_edges (
  src_node_id TEXT NOT NULL,
  edge_type TEXT NOT NULL,
  dst_node_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  updated_ts TEXT NOT NULL,
  PRIMARY KEY (src_node_id, edge_type, dst_node_id)
);

CREATE TABLE IF NOT EXISTS hot_state (
  state_key TEXT PRIMARY KEY,
  state_value_json TEXT NOT NULL,
  updated_ts TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memory_index (
  node_id TEXT NOT NULL,
  uid TEXT NOT NULL,
  file_rel TEXT NOT NULL,
  summary TEXT NOT NULL,
  tags_json TEXT NOT NULL,
  source TEXT NOT NULL,
  updated_ts TEXT NOT NULL,
  PRIMARY KEY (node_id, file_rel)
);

CREATE INDEX IF NOT EXISTS idx_memory_index_file ON memory_index(file_rel);
CREATE INDEX IF NOT EXISTS idx_memory_index_uid ON memory_index(uid);
CREATE INDEX IF NOT EXISTS idx_memory_index_node ON memory_index(node_id);
CREATE INDEX IF NOT EXISTS idx_memory_index_source ON memory_index(source);
CREATE INDEX IF NOT EXISTS idx_graph_edges_src ON temporal_graph_edges(src_node_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_dst ON temporal_graph_edges(dst_node_id);
"#,
            )
            .map_err(|err| format!("db_schema_failed:{err}"))?;

        // Optional sqlite-vec extension table; non-fatal when extension is unavailable.
        let _ = self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS embedding_vectors USING vec0(vector float[1536])",
            [],
        );
        Ok(())
    }

    pub fn count_index_rows(&self) -> Result<usize, String> {
        let count = self
            .conn
            .query_row("SELECT COUNT(1) FROM memory_index", [], |row| row.get::<_, i64>(0))
            .map_err(|err| format!("db_count_index_failed:{err}"))?;
        Ok(count.max(0) as usize)
    }

    pub fn load_index_entries(&self) -> Result<Vec<DbIndexEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT node_id, uid, file_rel, summary, tags_json
                 FROM memory_index
                 ORDER BY file_rel ASC, node_id ASC",
            )
            .map_err(|err| format!("db_prepare_index_load_failed:{err}"))?;
        let mapped = stmt
            .query_map([], |row| {
                let tags_raw = row.get::<_, String>(4)?;
                Ok(DbIndexEntry {
                    node_id: row.get::<_, String>(0)?,
                    uid: row.get::<_, String>(1)?,
                    file_rel: row.get::<_, String>(2)?,
                    summary: row.get::<_, String>(3)?,
                    tags: parse_tags_json(&tags_raw),
                })
            })
            .map_err(|err| format!("db_query_index_failed:{err}"))?;
        let mut out: Vec<DbIndexEntry> = vec![];
        for row in mapped {
            match row {
                Ok(entry) => out.push(entry),
                Err(err) => return Err(format!("db_row_decode_failed:{err}")),
            }
        }
        Ok(out)
    }

    pub fn replace_index_entries(
        &mut self,
        entries: &[DbIndexEntry],
        source: &str,
    ) -> Result<usize, String> {
        let tx = self
            .conn
            .transaction()
            .map_err(|err| format!("db_tx_start_failed:{err}"))?;
        tx.execute("DELETE FROM memory_index", [])
            .map_err(|err| format!("db_index_clear_failed:{err}"))?;
        let now = now_iso();
        for entry in entries {
            let tags_json =
                serde_json::to_string(&entry.tags).map_err(|err| format!("db_tags_encode_failed:{err}"))?;
            tx.execute(
                "INSERT INTO memory_index (node_id, uid, file_rel, summary, tags_json, source, updated_ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    entry.node_id,
                    entry.uid,
                    entry.file_rel,
                    entry.summary,
                    tags_json,
                    source,
                    now
                ],
            )
            .map_err(|err| format!("db_index_insert_failed:{err}"))?;
        }
        tx.commit()
            .map_err(|err| format!("db_tx_commit_failed:{err}"))?;
        Ok(entries.len())
    }

    pub fn get_hot_state_json(&self, key: &str) -> Result<Option<Value>, String> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT state_value_json FROM hot_state WHERE state_key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| format!("db_hot_state_query_failed:{err}"))?;
        match raw {
            Some(cipher) => {
                let plain = decrypt_value(&self.cipher_key, &cipher);
                let parsed =
                    serde_json::from_str::<Value>(&plain).map_err(|err| format!("db_hot_state_decode_failed:{err}"))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub fn set_hot_state_json(&self, key: &str, value: &Value) -> Result<(), String> {
        let encoded = serde_json::to_string(value).map_err(|err| format!("db_hot_state_encode_failed:{err}"))?;
        let encrypted = encrypt_value(&self.cipher_key, &encoded);
        self.conn
            .execute(
                "INSERT INTO hot_state (state_key, state_value_json, updated_ts)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(state_key) DO UPDATE SET
                   state_value_json = excluded.state_value_json,
                   updated_ts = excluded.updated_ts",
                params![key, encrypted, now_iso()],
            )
            .map_err(|err| format!("db_hot_state_upsert_failed:{err}"))?;
        Ok(())
    }
}
