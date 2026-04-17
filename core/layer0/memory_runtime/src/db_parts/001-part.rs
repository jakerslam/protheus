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
        db.migrate_legacy_hot_state_cipher()?;
        Ok(db)
    }

    pub fn rel_db_path(&self, root: &Path) -> String {
        self.db_path
            .strip_prefix(root)
            .unwrap_or(&self.db_path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn query_single_i64(&self, sql: &str) -> Result<i64, String> {
        self.conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(|err| format!("db_query_scalar_failed:{err}"))
    }

    fn query_table_count(&self, table: &str) -> Result<u64, String> {
        let sql = match table {
            "hot_state" => "SELECT COUNT(1) FROM hot_state",
            "memory_index" => "SELECT COUNT(1) FROM memory_index",
            "embeddings" => "SELECT COUNT(1) FROM embeddings",
            _ => return Err("db_unknown_table_count_request".to_string()),
        };
        let count = self
            .conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .map_err(|err| format!("db_table_count_failed:{err}"))?;
        Ok(count.max(0) as u64)
    }

    fn query_memory_index_kind_count(&self, kind: &str) -> Result<u64, String> {
        let count = self
            .conn
            .query_row(
                "SELECT COUNT(1) FROM memory_index WHERE kind = ?1",
                params![kind],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|err| format!("db_kind_count_failed:{err}"))?;
        Ok(count.max(0) as u64)
    }

    pub fn fragmentation_stats(&self) -> Result<DbFragmentationStats, String> {
        let page_count = self.query_single_i64("PRAGMA page_count;")?.max(0) as u64;
        let free_pages = self.query_single_i64("PRAGMA freelist_count;")?.max(0) as u64;
        let physical_fragmentation_ratio = if page_count == 0 {
            0.0
        } else {
            (free_pages as f64) / (page_count as f64)
        };
        let db_file_bytes = fs::metadata(&self.db_path).map(|meta| meta.len()).unwrap_or(0);
        let working_rows = self.query_table_count("hot_state")?;
        let episodic_rows = self.query_memory_index_kind_count("episodic")?;
        let semantic_rows = self.query_memory_index_kind_count("semantic")?;
        let procedural_rows = self.query_memory_index_kind_count("procedural")?;
        let tier_total = working_rows
            .saturating_add(episodic_rows)
            .saturating_add(semantic_rows)
            .saturating_add(procedural_rows);
        let tier_max = [working_rows, episodic_rows, semantic_rows, procedural_rows]
            .into_iter()
            .max()
            .unwrap_or(0);
        let tier_min = [working_rows, episodic_rows, semantic_rows, procedural_rows]
            .into_iter()
            .min()
            .unwrap_or(0);
        let tier_fragmentation_ratio = if tier_total == 0 {
            0.0
        } else {
            ((tier_max.saturating_sub(tier_min)) as f64) / (tier_total as f64)
        };
        let fragmentation_ratio = physical_fragmentation_ratio.max(tier_fragmentation_ratio);
        Ok(DbFragmentationStats {
            page_count,
            free_pages,
            fragmentation_ratio,
            physical_fragmentation_ratio,
            tier_fragmentation_ratio,
            db_file_bytes,
            working_rows,
            episodic_rows,
            semantic_rows,
            procedural_rows,
        })
    }

    pub fn predictive_realign_compaction(&self) -> Result<(), String> {
        let working_rows = self.query_table_count("hot_state")?;
        let episodic_rows = self.query_table_count("memory_index")?;
        let semantic_rows = self.query_table_count("embeddings")?;
        let target_working = episodic_rows
            .saturating_add(semantic_rows)
            .saturating_add(8)
            .max(32);
        if working_rows > target_working {
            let prune_rows = (working_rows - target_working) as i64;
            self.conn
                .execute(
                    "DELETE FROM hot_state
                     WHERE state_key IN (
                        SELECT state_key
                        FROM hot_state
                        ORDER BY updated_ts ASC, state_key ASC
                        LIMIT ?1
                     )",
                    params![prune_rows],
                )
                .map_err(|err| format!("db_predictive_realign_prune_failed:{err}"))?;
        }
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; ANALYZE;")
            .map_err(|err| format!("db_predictive_realign_failed:{err}"))?;
        Ok(())
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
  kind TEXT NOT NULL DEFAULT 'episodic',
  source TEXT NOT NULL,
  updated_ts TEXT NOT NULL,
  PRIMARY KEY (node_id, file_rel)
);

CREATE INDEX IF NOT EXISTS idx_memory_index_file ON memory_index(file_rel);
CREATE INDEX IF NOT EXISTS idx_memory_index_uid ON memory_index(uid);
CREATE INDEX IF NOT EXISTS idx_memory_index_node ON memory_index(node_id);
CREATE INDEX IF NOT EXISTS idx_memory_index_source ON memory_index(source);
CREATE INDEX IF NOT EXISTS idx_memory_index_kind ON memory_index(kind);
CREATE INDEX IF NOT EXISTS idx_graph_edges_src ON temporal_graph_edges(src_node_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_dst ON temporal_graph_edges(dst_node_id);
"#,
            )
            .map_err(|err| format!("db_schema_failed:{err}"))?;
        let _ = self.conn.execute(
            "ALTER TABLE memory_index ADD COLUMN kind TEXT NOT NULL DEFAULT 'episodic'",
            [],
        );

        // Optional sqlite-vec extension table; non-fatal when extension is unavailable.
        let _ = self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS embedding_vectors USING vec0(vector float[1536])",
            [],
        );
        Ok(())
    }

    fn migrate_legacy_hot_state_cipher(&self) -> Result<usize, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT state_key, state_value_json FROM hot_state")
            .map_err(|err| format!("db_hot_state_scan_prepare_failed:{err}"))?;
        let mapped = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| format!("db_hot_state_scan_failed:{err}"))?;
        let mut pending: Vec<(String, String)> = vec![];
        for row in mapped {
            let (state_key, value_raw) =
                row.map_err(|err| format!("db_hot_state_scan_row_failed:{err}"))?;
            let maybe_plain = decode_hot_state_payload_for_migration(&self.cipher_key, &value_raw)
                .map_err(|err| format!("db_hot_state_legacy_migrate_decode_failed:{err}"))?;
            let Some(plain) = maybe_plain else {
                continue;
            };
            let envelope = wrap_hot_state_envelope(&self.cipher_key, &plain)
                .map_err(|err| format!("db_hot_state_envelope_wrap_failed:{err}"))?;
            pending.push((state_key, envelope));
        }
        if pending.is_empty() {
            return Ok(0);
        }
        let now = now_iso();
        for (state_key, encrypted) in pending.iter() {
            self.conn
            .execute(
                "UPDATE hot_state SET state_value_json = ?1, updated_ts = ?2 WHERE state_key = ?3",
                params![encrypted, now, state_key],
            )
            .map_err(|err| format!("db_hot_state_migrate_update_failed:{err}"))?;
        }
        Ok(pending.len())
    }

    pub fn count_index_rows(&self) -> Result<usize, String> {
        let count = self
            .conn
            .query_row("SELECT COUNT(1) FROM memory_index", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|err| format!("db_count_index_failed:{err}"))?;
        Ok(count.max(0) as usize)
    }

    pub fn load_index_entries(&self) -> Result<Vec<DbIndexEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT node_id, uid, file_rel, summary, tags_json, kind
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
                    kind: row.get::<_, String>(5)?,
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
            let tags_json = serde_json::to_string(&entry.tags)
                .map_err(|err| format!("db_tags_encode_failed:{err}"))?;
            tx.execute(
                "INSERT INTO memory_index (node_id, uid, file_rel, summary, tags_json, kind, source, updated_ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.node_id,
                    entry.uid,
                    entry.file_rel,
                    entry.summary,
                    tags_json,
                    entry.kind,
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

    pub fn replace_embeddings(
        &mut self,
        entries: &[(String, Vec<f32>, Value)],
        source: &str,
    ) -> Result<usize, String> {
        let tx = self
            .conn
            .transaction()
            .map_err(|err| format!("db_embedding_tx_start_failed:{err}"))?;
        tx.execute("DELETE FROM embeddings", [])
            .map_err(|err| format!("db_embedding_clear_failed:{err}"))?;
        let now = now_iso();
        for (node_id, vector, metadata) in entries {
            let embedding_id = format!("{}::{}", source, node_id);
            let vector_blob = encode_vector_blob(vector)?;
            let metadata_json = serde_json::to_string(metadata)
                .map_err(|err| format!("db_embedding_metadata_encode_failed:{err}"))?;
            tx.execute(
                "INSERT INTO embeddings (embedding_id, node_id, vector_blob, metadata_json, created_ts, updated_ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    embedding_id,
                    node_id,
                    vector_blob,
                    metadata_json,
                    now,
                    now
                ],
            )
            .map_err(|err| format!("db_embedding_insert_failed:{err}"))?;
        }
        tx.commit()
            .map_err(|err| format!("db_embedding_tx_commit_failed:{err}"))?;
        Ok(entries.len())
    }

    pub fn load_embedding_map(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<f32>>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT node_id, vector_blob
                 FROM embeddings",
            )
            .map_err(|err| format!("db_prepare_embedding_load_failed:{err}"))?;
        let mapped = stmt
            .query_map([], |row| {
                let node_id = row.get::<_, String>(0)?;
                let blob = row.get::<_, Vec<u8>>(1)?;
                Ok((node_id, blob))
            })
            .map_err(|err| format!("db_query_embedding_failed:{err}"))?;
        let mut out = std::collections::HashMap::new();
        for row in mapped {
            match row {
                Ok((node_id, blob)) => {
                    let vector = decode_vector_blob(&blob)?;
                    if !vector.is_empty() {
                        out.insert(node_id, vector);
                    }
                }
                Err(err) => return Err(format!("db_embedding_row_decode_failed:{err}")),
            }
        }
        Ok(out)
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
                let plain = decrypt_hot_state_envelope(&self.cipher_key, &cipher)
                    .map_err(|err| format!("db_hot_state_decrypt_failed:{err}"))?;
                let parsed = serde_json::from_str::<Value>(&plain)
                    .map_err(|err| format!("db_hot_state_decode_failed:{err}"))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub fn set_hot_state_json(&self, key: &str, value: &Value) -> Result<(), String> {
        let encoded = serde_json::to_string(value)
            .map_err(|err| format!("db_hot_state_encode_failed:{err}"))?;
        let envelope = wrap_hot_state_envelope(&self.cipher_key, &encoded)
            .map_err(|err| format!("db_hot_state_encrypt_failed:{err}"))?;
        self.conn
            .execute(
                "INSERT INTO hot_state (state_key, state_value_json, updated_ts)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(state_key) DO UPDATE SET
                   state_value_json = excluded.state_value_json,
                   updated_ts = excluded.updated_ts",
                params![key, envelope, now_iso()],
            )
            .map_err(|err| format!("db_hot_state_upsert_failed:{err}"))?;
        Ok(())
    }

    pub fn hot_state_envelope_stats(&self) -> Result<HotStateEnvelopeStats, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT state_value_json FROM hot_state")
            .map_err(|err| format!("db_hot_state_stats_prepare_failed:{err}"))?;
        let mapped = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|err| format!("db_hot_state_stats_query_failed:{err}"))?;
        let mut stats = HotStateEnvelopeStats::default();
        for row in mapped {
            let raw = row.map_err(|err| format!("db_hot_state_stats_row_failed:{err}"))?;
            stats.total_rows += 1;
            let body = raw.trim();
            if parse_hot_state_envelope_ciphertext(body).is_some() {
                stats.enveloped_rows += 1;
            } else if body.starts_with("aead-v1:") || body.starts_with("enc-v1:") {
                stats.legacy_cipher_rows += 1;
            } else {
                stats.plain_rows += 1;
            }
        }
        Ok(stats)
    }
}
