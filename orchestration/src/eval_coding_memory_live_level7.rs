use crate::coding_memory_runtime_bridge::{
    millis_now, project_snapshot, workspace_root, CodingMemoryRuntimeBridge,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel7SeedBatchReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub jobs: Vec<LiveLevel7Job>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel7Job {
    pub attempt_id: String,
    pub package: String,
    pub run_root: String,
    pub project_root: String,
    pub receipts_root: String,
    pub prompt_path: String,
    pub memory_db_path: String,
    pub resume_token: String,
    pub prior_memory_row_id: String,
    pub expected_new_memory_row_id: String,
    pub project_fingerprint: String,
    pub architecture_hash: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel7JudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub attempts: Vec<LiveLevel7AttemptJudge>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel7AttemptJudge {
    pub attempt_id: String,
    pub ok: bool,
    pub checks: Vec<LiveLevel7Check>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel7Check {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct DomainSpec {
    id: &'static str,
    package: &'static str,
    architecture_name: &'static str,
    primary_kind: &'static str,
    primary_destination: &'static str,
    secondary_prefix: &'static str,
    secondary_destination: &'static str,
}

const DOMAIN_SPECS: &[DomainSpec] = &[
    DomainSpec {
        id: "event_router",
        package: "event_router",
        architecture_name: "Event Router",
        primary_kind: "billing.invoice.created",
        primary_destination: "billing",
        secondary_prefix: "support",
        secondary_destination: "support",
    },
    DomainSpec {
        id: "incident_router",
        package: "incident_router",
        architecture_name: "Incident Router",
        primary_kind: "incident.created",
        primary_destination: "incident",
        secondary_prefix: "oncall",
        secondary_destination: "oncall",
    },
    DomainSpec {
        id: "fulfillment_router",
        package: "fulfillment_router",
        architecture_name: "Fulfillment Router",
        primary_kind: "order.shipped",
        primary_destination: "fulfillment",
        secondary_prefix: "warehouse",
        secondary_destination: "warehouse",
    },
    DomainSpec {
        id: "risk_router",
        package: "risk_router",
        architecture_name: "Risk Router",
        primary_kind: "risk.alert.created",
        primary_destination: "risk",
        secondary_prefix: "fraud",
        secondary_destination: "fraud",
    },
    DomainSpec {
        id: "notification_router",
        package: "notification_router",
        architecture_name: "Notification Router",
        primary_kind: "notification.email.queued",
        primary_destination: "email",
        secondary_prefix: "sms",
        secondary_destination: "sms",
    },
];

pub fn seed_live_level7_batch(attempt_count: usize) -> LiveLevel7SeedBatchReport {
    let count = attempt_count.max(1);
    let batch_root = std::env::temp_dir().join(format!(
        "coding-memory-live-level7-batch-{}-{}",
        std::process::id(),
        millis_now()
    ));
    let prompts_root = batch_root.join("prompts");
    let mut jobs = Vec::new();
    let mut failures = Vec::new();

    if let Err(error) = fs::create_dir_all(&prompts_root) {
        failures.push(format!("create_prompts_root_failed:{error}"));
    }

    for index in 0..count {
        let spec = &DOMAIN_SPECS[index % DOMAIN_SPECS.len()];
        match seed_live_attempt(index + 1, spec, &batch_root, &prompts_root) {
            Ok(job) => jobs.push(job),
            Err(error) => failures.push(error),
        }
    }

    let report = LiveLevel7SeedBatchReport {
        harness_kind: "coding_memory_live_level7_seed_v1".to_string(),
        ok: failures.is_empty() && jobs.len() == count,
        batch_root: batch_root.display().to_string(),
        attempt_count: jobs.len(),
        jobs,
        failures,
        operator_next_action: "spawn_one_worker_per_prompt_then_run_judge".to_string(),
    };
    let _ = write_file(
        &batch_root.join("jobs.json"),
        &serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()),
    );
    report
}

pub fn judge_live_level7_batch(batch_root: &Path) -> LiveLevel7JudgeReport {
    let mut failures = Vec::new();
    let jobs_path = batch_root.join("jobs.json");
    let seed_report = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LiveLevel7SeedBatchReport>(&raw).ok());
    let jobs = match seed_report {
        Some(report) => report.jobs,
        None => {
            failures.push(format!("jobs_json_unreadable:{}", jobs_path.display()));
            Vec::new()
        }
    };

    let mut attempts = Vec::new();
    for job in &jobs {
        attempts.push(judge_live_attempt(job));
    }
    for attempt in &attempts {
        if !attempt.ok {
            failures.extend(
                attempt
                    .failures
                    .iter()
                    .map(|failure| format!("{}:{failure}", attempt.attempt_id)),
            );
        }
    }
    let pass_count = attempts.iter().filter(|attempt| attempt.ok).count();
    let fail_count = attempts.len().saturating_sub(pass_count);
    LiveLevel7JudgeReport {
        harness_kind: "coding_memory_live_level7_judge_v1",
        ok: failures.is_empty() && !attempts.is_empty(),
        batch_root: batch_root.display().to_string(),
        attempt_count: attempts.len(),
        pass_count,
        fail_count,
        attempts,
        failures,
    }
}

fn seed_live_attempt(
    ordinal: usize,
    spec: &DomainSpec,
    batch_root: &Path,
    prompts_root: &Path,
) -> Result<LiveLevel7Job, String> {
    let attempt_id = format!("attempt_{ordinal:02}_{}", spec.id);
    let run_root = batch_root.join(&attempt_id);
    let project_root = run_root.join("project");
    let receipts_root = run_root.join("receipts");
    let memory_db_path = run_root.join("runtime_memory.sqlite");
    fs::create_dir_all(&receipts_root).map_err(|error| {
        format!(
            "{attempt_id}:create_receipts_root_failed:{}:{error}",
            receipts_root.display()
        )
    })?;
    seed_python_project(spec, &project_root)?;

    let validation = run_python_validation(&project_root);
    if !validation.ok {
        return Err(format!(
            "{attempt_id}:seed_validation_failed:{}",
            validation.detail
        ));
    }

    let architecture_text = read_to_string(&project_root.join("ARCHITECTURE.md"));
    let manifest_text = read_to_string(&project_root.join("PROJECT_MANIFEST.txt"));
    let validation_command = "PYTHONPATH=src python3 -m unittest discover -s tests";
    let snapshot = project_snapshot(
        &attempt_id,
        &project_root,
        &architecture_text,
        &manifest_text,
        validation_command,
    );
    let resume_token = format!("live_level7_resume_{}_{}", attempt_id, millis_now());
    let prior_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_001",
        snapshot.project_fingerprint
    );
    let expected_new_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_002",
        snapshot.project_fingerprint
    );
    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: memory_db_path.clone(),
        session_id: attempt_id.clone(),
    };
    let prior_payload = serde_json::to_string(&json!({
        "schema_version": "checkpoint_memory_write_v1",
        "project_id": attempt_id,
        "project_root": project_root.display().to_string(),
        "project_fingerprint": snapshot.project_fingerprint,
        "architecture_hash": snapshot.architecture_hash,
        "completed_checkpoint": "checkpoint_001_seed_router_baseline",
        "changed_files": [
            "ARCHITECTURE.md",
            "PROJECT_MANIFEST.txt",
            &format!("src/{}/models.py", spec.package),
            &format!("src/{}/router.py", spec.package),
            "tests/test_router.py"
        ],
        "validation_results": {
            "status": "pass",
            "command": validation_command,
            "exit_code": 0
        },
        "recommended_next_checkpoint": "checkpoint_002_delivery_attempt_ledger",
        "next_slice_goal": "Add a delivery attempt ledger that records routed events, summarizes attempt counts by destination, identifies retryable failures, preserves baseline routing behavior, and includes regression tests plus a checkpoint receipt.",
        "constraints": [
            "read current files before planning",
            "current workspace files remain authoritative over memory",
            "use Python stdlib only",
            "keep the slice small and production-shaped"
        ],
        "unique_probe_token": resume_token
    }))
    .map_err(|error| format!("{attempt_id}:prior_payload_json_failed:{error}"))?;
    let ingest = bridge.ingest(
        &prior_memory_row_id,
        &prior_payload,
        &["coding", "checkpoint", "resume", "project_context"],
    );
    if !ingest.ok {
        return Err(format!(
            "{attempt_id}:prior_memory_ingest_failed:{}",
            ingest.payload
        ));
    }

    let prompt_path = prompts_root.join(format!("{attempt_id}.txt"));
    let job = LiveLevel7Job {
        attempt_id: attempt_id.clone(),
        package: spec.package.to_string(),
        run_root: run_root.display().to_string(),
        project_root: project_root.display().to_string(),
        receipts_root: receipts_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        memory_db_path: memory_db_path.display().to_string(),
        resume_token,
        prior_memory_row_id,
        expected_new_memory_row_id,
        project_fingerprint: snapshot.project_fingerprint,
        architecture_hash: snapshot.architecture_hash,
        validation_command: validation_command.to_string(),
    };
    write_file(&prompt_path, &worker_prompt(&job)).map_err(|error| {
        format!(
            "{attempt_id}:write_worker_prompt_failed:{}:{error}",
            prompt_path.display()
        )
    })?;
    Ok(job)
}

fn seed_python_project(spec: &DomainSpec, root: &Path) -> Result<(), String> {
    write_file(
        &root.join("ARCHITECTURE.md"),
        &format!(
            "# {} Architecture\n\nThis existing project is a Python stdlib router package. Keep domain models in `models.py`, routing decisions in `router.py`, and add new behavior behind focused modules with regression tests. Current files are the source of truth; stored memory can guide resume planning but must not override changed files.\n\nCheckpoint policy: each checkpoint should preserve baseline routing, add one coherent production slice, and leave a receipt describing changed files, validation, and follow-up scope.\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join("PROJECT_MANIFEST.txt"),
        &format!(
            "python-stdlib unittest {} existing-project live-level7\n",
            spec.id
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/__init__.py", spec.package)),
        &format!(
            "\"\"\"{} live Level 7 probe package.\"\"\"\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/models.py", spec.package)),
        "from dataclasses import dataclass\nfrom typing import Mapping\n\n\n@dataclass(frozen=True)\nclass Event:\n    event_id: str\n    kind: str\n    payload: Mapping[str, str]\n",
    )?;
    write_file(
        &root.join(format!("src/{}/router.py", spec.package)),
        &format!(
            "from {}.models import Event\n\n\ndef route_event(event: Event) -> str:\n    if event.kind == \"{}\":\n        return \"{}\"\n    if event.kind.startswith(\"{}.\"):\n        return \"{}\"\n    return \"default\"\n",
            spec.package,
            spec.primary_kind,
            spec.primary_destination,
            spec.secondary_prefix,
            spec.secondary_destination
        ),
    )?;
    write_file(
        &root.join("tests/test_router.py"),
        &format!(
            "import unittest\n\nfrom {package}.models import Event\nfrom {package}.router import route_event\n\n\nclass RouterBaselineTest(unittest.TestCase):\n    def test_routes_primary_event(self):\n        event = Event(\"evt-1\", \"{primary_kind}\", {{\"source\": \"primary\"}})\n        self.assertEqual(route_event(event), \"{primary_destination}\")\n\n    def test_routes_secondary_family(self):\n        event = Event(\"evt-2\", \"{secondary_prefix}.created\", {{\"source\": \"secondary\"}})\n        self.assertEqual(route_event(event), \"{secondary_destination}\")\n\n    def test_routes_unknown_to_default(self):\n        event = Event(\"evt-3\", \"analytics.page.viewed\", {{}})\n        self.assertEqual(route_event(event), \"default\")\n\n\nif __name__ == \"__main__\":\n    unittest.main()\n",
            package = spec.package,
            primary_kind = spec.primary_kind,
            primary_destination = spec.primary_destination,
            secondary_prefix = spec.secondary_prefix,
            secondary_destination = spec.secondary_destination
        ),
    )?;
    Ok(())
}

fn worker_prompt(job: &LiveLevel7Job) -> String {
    format!(
        "You are running a live Level 7 coding-memory resume probe. You are not alone in the broader codebase: do not revert or modify anything outside the assigned temp run directory. Your write ownership is limited to {project_root} and {receipts_root}.\n\nGoal: continue the existing local Python project by using current files plus stored checkpoint memory. Do not ask follow-up questions. Complete one coherent checkpoint slice.\n\nEnvironment:\n- Project root: {project_root}\n- Python package: {package}\n- Isolated memory DB: {memory_db_path}\n- Resume token: {resume_token}\n- Prior memory row id: {prior_memory_row_id}\n- Expected new memory row id: {expected_new_memory_row_id}\n- Memory CLI command pattern: INFRING_MEMORY_DB_PATH={memory_db_path} cargo run --quiet --manifest-path /Users/jay/.openclaw/workspace/core/layer0/memory/Cargo.toml --bin memory-cli -- <command>\n- Validation command from project root: {validation_command}\n\nWorkflow requirements:\n1. Read the local project files first. Current files are authoritative.\n2. Retrieve checkpoint memory using the resume token and/or row id with the memory CLI.\n3. Decide the next checkpoint from local context plus memory.\n4. Implement checkpoint_002_delivery_attempt_ledger in multiple files. The slice must expose a DeliveryAttempt model and DeliveryAttemptLedger abstraction using those exact code identifiers, compute or persist counts_by_destination, expose retryable_failures or retryable metadata, integrate with existing route_event/routing behavior, preserve baseline routing, and add regression tests for both the ledger and existing routing. The existing baseline 3 tests passing is not completion evidence; validation should exercise the newly added ledger behavior.\n5. Run the validation command.\n6. Write a checkpoint receipt under {receipts_root}/checkpoint_002_handoff.json.\n7. Write a new checkpoint memory row to the isolated DB using the expected new memory row id and tags coding,checkpoint,resume,project_context. Include changed files, validation result, known risks, and recommended next checkpoint.\n\nFinal response should include: whether it passed, changed file paths, validation command/result, new memory row id, and any caveats. Do not commit anything.\n",
        project_root = job.project_root,
        receipts_root = job.receipts_root,
        package = job.package,
        memory_db_path = job.memory_db_path,
        resume_token = job.resume_token,
        prior_memory_row_id = job.prior_memory_row_id,
        expected_new_memory_row_id = job.expected_new_memory_row_id,
        validation_command = job.validation_command
    )
}

fn judge_live_attempt(job: &LiveLevel7Job) -> LiveLevel7AttemptJudge {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let project_root = PathBuf::from(&job.project_root);
    let receipt_path = PathBuf::from(&job.receipts_root).join("checkpoint_002_handoff.json");

    let validation = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "validation_passes_after_live_worker",
        validation.ok,
        validation.detail,
    );

    let receipt = read_json_file(&receipt_path);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_receipt_written",
        receipt.is_some(),
        receipt_path.display().to_string(),
    );
    if let Some(receipt) = &receipt {
        let completed_checkpoint = receipt
            .get("completed_checkpoint")
            .or_else(|| receipt.get("checkpoint"))
            .and_then(Value::as_str)
            .unwrap_or("missing_completed_checkpoint");
        let completed_checkpoint_ok =
            completed_checkpoint == "checkpoint_002_delivery_attempt_ledger";
        push_check(
            &mut checks,
            &mut failures,
            "receipt_declares_checkpoint_002",
            completed_checkpoint_ok,
            completed_checkpoint.to_string(),
        );
        let changed_file_count = receipt
            .get("changed_files")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        push_check(
            &mut checks,
            &mut failures,
            "receipt_declares_multiple_changed_files",
            changed_file_count >= 2,
            format!("changed_file_count={changed_file_count}"),
        );
    }

    let (delivery_module_present, delivery_module_detail) =
        delivery_attempt_slice_present(&project_root, &job.package);
    push_check(
        &mut checks,
        &mut failures,
        "delivery_attempt_slice_present",
        delivery_module_present,
        delivery_module_detail,
    );

    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: PathBuf::from(&job.memory_db_path),
        session_id: format!("{}_judge", job.attempt_id),
    };
    let memory_get = bridge.get(&job.expected_new_memory_row_id);
    let memory_content = memory_get
        .payload
        .pointer("/row/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_002_memory_row_written",
        memory_get.ok && memory_content.contains("checkpoint_002_delivery_attempt_ledger"),
        format!("ok={}", memory_get.ok),
    );
    push_check(
        &mut checks,
        &mut failures,
        "memory_row_preserves_validation_result",
        memory_get.ok && memory_content.contains("\"status\"") && memory_content.contains("pass"),
        "memory row includes validation status".to_string(),
    );

    LiveLevel7AttemptJudge {
        attempt_id: job.attempt_id.clone(),
        ok: failures.is_empty(),
        checks,
        failures,
    }
}

fn push_check(
    checks: &mut Vec<LiveLevel7Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    if !ok {
        failures.push(id.to_string());
    }
    checks.push(LiveLevel7Check { id, ok, detail });
}

struct ValidationResult {
    ok: bool,
    detail: String,
}

fn run_python_validation(project_root: &Path) -> ValidationResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg("unittest")
        .arg("discover")
        .arg("-s")
        .arg("tests")
        .env("PYTHONPATH", project_root.join("src"))
        .current_dir(project_root)
        .output();
    match output {
        Ok(output) => ValidationResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(error) => ValidationResult {
            ok: false,
            detail: format!("python_validation_spawn_failed:{error}"),
        },
    }
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn delivery_attempt_slice_present(project_root: &Path, package: &str) -> (bool, String) {
    let package_root = project_root.join(format!("src/{package}"));
    let entries = match fs::read_dir(&package_root) {
        Ok(entries) => entries,
        Err(error) => {
            return (
                false,
                format!("read_package_dir_failed:{}:{error}", package_root.display()),
            )
        }
    };
    let mut combined = String::new();
    let mut scanned = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if matches!(file_name, "__init__.py" | "models.py" | "router.py") {
            let content = read_to_string(&path);
            combined.push_str(&content);
            combined.push('\n');
            continue;
        }
        scanned.push(path.display().to_string());
        let content = read_to_string(&path);
        combined.push_str(&content);
        combined.push('\n');
    }
    let has_attempt_model = combined.contains("DeliveryAttempt");
    let has_ledger = combined.contains("DeliveryAttemptLedger");
    let has_destination_summary = combined.contains("counts_by_destination")
        || combined.contains("attempt_counts_by_destination")
        || combined.contains("summarize_by_destination")
        || combined.contains("by_destination");
    let has_retryable = combined.contains("retryable_failures") || combined.contains("retryable");
    let has_routing_integration = combined.contains("route_event")
        || combined.contains("route_and_record")
        || combined.contains("record_routed");
    let ok = has_attempt_model
        && has_ledger
        && has_destination_summary
        && has_retryable
        && has_routing_integration;
    (
        ok,
        format!(
            "attempt_model={has_attempt_model} ledger={has_ledger} destination_summary={has_destination_summary} retryable={has_retryable} routing={has_routing_integration} scanned={}",
            scanned.join(",")
        ),
    )
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("write_file_failed:{}:{error}", path.display()))
}
