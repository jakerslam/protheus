use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct CodingMemoryRuntimeBridge {
    pub workspace_root: PathBuf,
    pub memory_db_path: PathBuf,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryCommandResult {
    pub ok: bool,
    pub command: String,
    pub payload: Value,
}

impl CodingMemoryRuntimeBridge {
    pub fn isolated(session_id: &str) -> Self {
        let workspace_root = workspace_root();
        let memory_db_path = std::env::temp_dir()
            .join(format!(
                "coding-memory-runtime-{}-{}",
                std::process::id(),
                millis_now()
            ))
            .join("runtime_memory.sqlite");
        Self {
            workspace_root,
            memory_db_path,
            session_id: session_id.to_string(),
        }
    }

    pub fn resume_from(&self, session_id: &str) -> Self {
        Self {
            workspace_root: self.workspace_root.clone(),
            memory_db_path: self.memory_db_path.clone(),
            session_id: session_id.to_string(),
        }
    }

    pub fn ingest(&self, id: &str, content: &str, tags: &[&str]) -> MemoryCommandResult {
        let tag_arg = tags.join(",");
        self.run_memory_cli(&[
            "ingest",
            &format!("--id={id}"),
            &format!("--content={content}"),
            &format!("--tags={tag_arg}"),
            "--repetitions=4",
            "--lambda=0.02",
        ])
    }

    pub fn recall(&self, query: &str, limit: u32) -> MemoryCommandResult {
        self.run_memory_cli(&[
            "recall",
            &format!("--query={query}"),
            &format!("--limit={limit}"),
        ])
    }

    pub fn get(&self, id: &str) -> MemoryCommandResult {
        self.run_memory_cli(&["get", &format!("--id={id}")])
    }

    fn run_memory_cli(&self, args: &[&str]) -> MemoryCommandResult {
        let manifest_path = self.workspace_root.join("core/layer0/memory/Cargo.toml");
        let command = format!(
            "cargo run --quiet --manifest-path {} --bin memory-cli -- {}",
            manifest_path.display(),
            args.join(" ")
        );
        let output = Command::new("cargo")
            .arg("run")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(manifest_path)
            .arg("--bin")
            .arg("memory-cli")
            .arg("--")
            .args(args)
            .env("INFRING_MEMORY_DB_PATH", &self.memory_db_path)
            .current_dir(&self.workspace_root)
            .output();
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let payload = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| {
                    json!({
                        "ok": false,
                        "error": "memory_cli_invalid_json",
                        "status": output.status.code(),
                        "stdout": stdout,
                        "stderr": stderr
                    })
                });
                MemoryCommandResult {
                    ok: payload.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    command,
                    payload,
                }
            }
            Err(error) => MemoryCommandResult {
                ok: false,
                command,
                payload: json!({
                    "ok": false,
                    "error": format!("memory_cli_spawn_failed:{error}")
                }),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectContextSnapshot {
    pub project_id: String,
    pub project_root: String,
    pub project_fingerprint: String,
    pub architecture_hash: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryFreshnessDecision {
    pub status: &'static str,
    pub allowed_memory_use: &'static str,
    pub current_files_source_of_truth: bool,
}

pub fn decide_memory_freshness(
    current: &ProjectContextSnapshot,
    remembered_project_fingerprint: Option<&str>,
    remembered_architecture_hash: Option<&str>,
) -> MemoryFreshnessDecision {
    match (remembered_project_fingerprint, remembered_architecture_hash) {
        (None, _) => freshness("no_memory_found", "continue_without_memory"),
        (Some(memory_fingerprint), Some(memory_architecture_hash))
            if memory_fingerprint == current.project_fingerprint
                && memory_architecture_hash == current.architecture_hash =>
        {
            freshness("fresh", "seed_checkpoint_definition_and_slice_planning")
        }
        (Some(memory_fingerprint), Some(memory_architecture_hash))
            if memory_fingerprint == current.project_fingerprint
                && memory_architecture_hash != current.architecture_hash =>
        {
            freshness(
                "conflicting_ignore_for_decisions",
                "ignore_for_architecture_and_implementation_decisions",
            )
        }
        (Some(_), _) => freshness("stale_hints_only", "use_only_as_search_hints"),
    }
}

fn freshness(status: &'static str, allowed_memory_use: &'static str) -> MemoryFreshnessDecision {
    MemoryFreshnessDecision {
        status,
        allowed_memory_use,
        current_files_source_of_truth: true,
    }
}

pub fn stable_hash(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn project_snapshot(
    project_id: &str,
    project_root: &Path,
    architecture_text: &str,
    manifest_text: &str,
    validation_command: &str,
) -> ProjectContextSnapshot {
    let architecture_hash = stable_hash(&[architecture_text]);
    let project_fingerprint = stable_hash(&[
        project_id,
        &project_root.display().to_string(),
        architecture_text,
        manifest_text,
        validation_command,
    ]);
    ProjectContextSnapshot {
        project_id: project_id.to_string(),
        project_root: project_root.display().to_string(),
        project_fingerprint,
        architecture_hash,
        validation_command: validation_command.to_string(),
    }
}

pub fn millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
