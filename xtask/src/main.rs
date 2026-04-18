use anyhow::{anyhow, bail, Context, Result};
use infring_agent_surface::{
    default_template_dir, run_runtime_lane, scaffold_template, RuntimeLaneRequest, TemplateKind,
    TemplateScaffoldOptions,
};
use nursery_runtime::{
    build_specialist_training_plan, containment_permissions_from_value, seed_manifest_from_value,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const INFRING_DETACH_XTASK_CONTRACT_ID: &str = "V6-INFRING-DETACH-001.7";

fn main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        bail!("xtask_missing_command");
    }

    let command = args.remove(0);
    match command.as_str() {
        "infring-detach-bootstrap" => run_infring_detach_bootstrap(&args),
        "verify-infring-detach" => run_verify_infring_detach(&args),
        "emit-nursery-plan" => run_emit_nursery_plan(&args),
        "infring-new" => run_infring_new(&args),
        "infring-agent-run" => run_infring_agent_run(&args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            print_usage();
            bail!("xtask_unknown_command:{other}")
        }
    }
}

fn print_usage() {
    println!("xtask commands:");
    println!(
        "  cargo run -p xtask -- infring-detach-bootstrap [--source-root=..] [--apply=1|0] [--strict=1|0] [--max-copy-mb=2048]"
    );
    println!(
        "  cargo run -p xtask -- verify-infring-detach [--root=. ]  # validates assimilated artifacts"
    );
    println!(
        "  cargo run -p xtask -- emit-nursery-plan --seed=<seed_manifest.json> --permissions=<permissions.json> --out=<plan.json>"
    );
    println!(
        "  cargo run -p xtask -- infring-new --template=single-agent|swarm|rag|voice --name=<project-name> [--out=<dir>] [--force=1|0]"
    );
    println!(
        "  cargo run -p xtask -- infring-agent-run --name=<agent> --prompt=<text> [--provider=local-echo] [--pack=research,web-ops] [--tool=web.search,web.fetch] [--lifespan=3600] [--permissions=<json|@file>] [--wasm-policy=<json|@file>] [--voice=<json|@file>] [--receipt-merkle=1|0] [--receipt-merkle-seed=<seed>] [--prev-receipt-root=<hash>]"
    );
}

fn parse_flag_map(args: &[String]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for arg in args {
        if !arg.starts_with("--") {
            continue;
        }
        if let Some((k, v)) = arg[2..].split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        } else {
            out.insert(arg[2..].trim().to_string(), "1".to_string());
        }
    }
    out
}

fn parse_bool(raw: Option<&String>, default: bool) -> bool {
    let Some(v) = raw else {
        return default;
    };
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_u64(raw: Option<&String>, default: u64) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(default)
}

fn resolve_workspace_root() -> Result<PathBuf> {
    let cwd = env::current_dir().context("xtask_current_dir_failed")?;
    if cwd.join("Cargo.toml").exists() {
        return Ok(cwd);
    }
    let mut probe = cwd.as_path();
    while let Some(parent) = probe.parent() {
        if parent.join("Cargo.toml").exists() {
            return Ok(parent.to_path_buf());
        }
        probe = parent;
    }
    Err(anyhow!("xtask_workspace_root_not_found"))
}

fn run_infring_detach_bootstrap(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let root = resolve_workspace_root()?;
    let source_root = flags
        .get("source-root")
        .cloned()
        .unwrap_or_else(|| "..".to_string());
    let apply = parse_bool(flags.get("apply"), true);
    let strict = parse_bool(flags.get("strict"), true);
    let max_copy_mb = parse_u64(flags.get("max-copy-mb"), 2048);

    let contract_ids = [
        "V6-INFRING-DETACH-001.1",
        "V6-INFRING-DETACH-001.2",
        "V6-INFRING-DETACH-001.3",
        "V6-INFRING-DETACH-001.4",
    ];

    let payload = json!({
        "source_root": source_root,
        "max_assimilation_copy_mb": max_copy_mb,
    })
    .to_string();

    let mut executed = Vec::<Value>::new();
    for id in contract_ids {
        let output = Command::new("cargo")
            .current_dir(&root)
            .arg("run")
            .arg("--quiet")
            .arg("--package")
            .arg("protheus-ops-core")
            .arg("--bin")
            .arg("protheus-ops")
            .arg("--")
            .arg("runtime-systems")
            .arg("run")
            .arg(format!("--id={id}"))
            .arg(format!("--apply={}", if apply { 1 } else { 0 }))
            .arg(format!("--strict={}", if strict { 1 } else { 0 }))
            .arg(format!("--payload-json={payload}"))
            .output()
            .with_context(|| format!("xtask_infring_detach_exec_failed:{id}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!(
                "xtask_infring_detach_contract_failed:{id}:stdout={stdout}:stderr={stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed =
            serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({"raw": stdout}));
        executed.push(json!({"id": id, "result": parsed}));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
            "executed_contracts": executed,
            "source_root": source_root,
            "apply": apply,
            "strict": strict,
            "max_copy_mb": max_copy_mb,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_verify_infring_detach(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let root = flags
        .get("root")
        .map(PathBuf::from)
        .unwrap_or(resolve_workspace_root()?);

    let required = [
        "config/infring_assimilation/cron/jobs.json",
        "config/infring_assimilation/nursery/containment/permissions.json",
        "config/infring_assimilation/nursery/manifests/seed_manifest.json",
        "config/infring_assimilation/llm/model_registry.json",
        "local/state/nursery/promotion/specialist_training_plan.json",
        "local/state/llm_runtime/model_registry.json",
    ];

    let mut missing = Vec::<String>::new();
    let mut present = Vec::<String>::new();
    for rel in required {
        let path = root.join(rel);
        if path.exists() {
            present.push(rel.to_string());
        } else {
            missing.push(rel.to_string());
        }
    }

    let out = json!({
        "ok": missing.is_empty(),
        "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
        "root": root.display().to_string(),
        "present": present,
        "missing": missing,
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("encode"));

    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        bail!("xtask_infring_detach_verify_missing_required_artifacts");
    }
    Ok(())
}

fn run_emit_nursery_plan(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let seed_path = flags
        .get("seed")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_seed_path"))?;
    let permissions_path = flags
        .get("permissions")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_permissions_path"))?;
    let out_path = flags
        .get("out")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask_missing_out_path"))?;

    let seed_raw = read_json_file(&seed_path)?;
    let permissions_raw = read_json_file(&permissions_path)?;

    let seed = seed_manifest_from_value(&seed_raw);
    let permissions = containment_permissions_from_value(&permissions_raw);
    let generated_at = now_isoish();
    let plan = build_specialist_training_plan(&generated_at, &seed, &permissions);

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("xtask_emit_nursery_plan_mkdir_failed:{}", parent.display())
        })?;
    }
    fs::write(
        &out_path,
        serde_json::to_string_pretty(&plan).expect("encode nursery plan"),
    )
    .with_context(|| {
        format!(
            "xtask_emit_nursery_plan_write_failed:{}",
            out_path.display()
        )
    })?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "contract_id": INFRING_DETACH_XTASK_CONTRACT_ID,
            "seed_path": seed_path.display().to_string(),
            "permissions_path": permissions_path.display().to_string(),
            "out_path": out_path.display().to_string(),
            "specialist_count": plan.specialists.len(),
            "max_train_minutes": plan.max_train_minutes,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_infring_new(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let template_raw = flags
        .get("template")
        .map(|value| value.as_str())
        .unwrap_or("single-agent");
    let template = TemplateKind::parse(template_raw)
        .ok_or_else(|| anyhow!("xtask_invalid_template:{template_raw}"))?;
    let name = flags
        .get("name")
        .cloned()
        .unwrap_or_else(|| "infring-agent-app".to_string());
    let root = resolve_workspace_root()?;
    let out = flags
        .get("out")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_template_dir(&root));
    let force = parse_bool(flags.get("force"), false);

    let result = scaffold_template(
        template,
        &TemplateScaffoldOptions {
            name: name.clone(),
            out_dir: out.clone(),
            force,
        },
    )
    .with_context(|| format!("xtask_infring_new_failed:name={name}:out={}", out.display()))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "template": result.template,
            "root": result.root,
            "created_files": result.created_files,
            "name": name,
            "out_dir": out.display().to_string(),
            "force": force,
        }))
        .expect("encode")
    );
    Ok(())
}

fn run_infring_agent_run(args: &[String]) -> Result<()> {
    let flags = parse_flag_map(args);
    let name = flags
        .get("name")
        .cloned()
        .unwrap_or_else(|| "infring-agent".to_string());
    let prompt = flags
        .get("prompt")
        .cloned()
        .ok_or_else(|| anyhow!("xtask_missing_prompt"))?;
    let preamble = flags.get("preamble").cloned();
    let provider = flags.get("provider").cloned();
    let model = flags.get("model").cloned();
    let lifespan_seconds = parse_u64(flags.get("lifespan"), 3600);
    let packs = csv_tokens(flags.get("pack"));
    let tools = csv_tokens(flags.get("tool"));
    let permissions_manifest = parse_json_flag(flags.get("permissions"))?;
    let wasm_sandbox = parse_json_flag(flags.get("wasm-policy"))?;
    let voice_session = parse_json_flag(flags.get("voice"))?;
    let receipt_merkle = if parse_bool(flags.get("receipt-merkle"), false) {
        Some(json!({
            "enabled": true,
            "seed": flags.get("receipt-merkle-seed").cloned()
        }))
    } else {
        None
    };
    let previous_receipt_root = flags.get("prev-receipt-root").cloned();

    let response = run_runtime_lane(RuntimeLaneRequest {
        name,
        preamble,
        initial_prompt: prompt,
        provider,
        model,
        tools,
        capability_packs: packs,
        lifespan_seconds: Some(lifespan_seconds),
        metadata: json!({
            "source": "xtask.infring-agent-run"
        }),
        permissions_manifest,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
    })
    .map_err(|error| anyhow!("xtask_infring_agent_run_failed:{error}"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&response).expect("encode runtime lane")
    );
    Ok(())
}

fn csv_tokens(raw: Option<&String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn parse_json_flag(raw: Option<&String>) -> Result<Option<Value>> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Some(path) = trimmed.strip_prefix('@') {
        let bytes = fs::read(path)
            .with_context(|| format!("xtask_json_flag_read_failed:{path}"))?;
        let parsed = serde_json::from_slice::<Value>(&bytes)
            .with_context(|| format!("xtask_json_flag_parse_failed:{path}"))?;
        return Ok(Some(parsed));
    }
    let parsed = serde_json::from_str::<Value>(trimmed)
        .with_context(|| format!("xtask_json_flag_parse_failed:inline:{trimmed}"))?;
    Ok(Some(parsed))
}

fn read_json_file(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| format!("xtask_read_failed:{}", path.display()))?;
    serde_json::from_slice::<Value>(&bytes)
        .with_context(|| format!("xtask_parse_json_failed:{}", path.display()))
}

fn now_isoish() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{ts}")
}
