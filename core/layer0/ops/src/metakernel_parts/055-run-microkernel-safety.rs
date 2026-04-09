const MICROKERNEL_TYPED_SYSCALLS: &[&str] = &[
    "invoke_agent",
    "fork_instance",
    "verify_receipt",
    "halt_on_drift",
];

fn parse_usize_flag(raw: Option<&String>, fallback: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .min(max)
}

fn parse_f64_flag(raw: Option<&String>, fallback: f64, min: f64, max: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn normalize_token(raw: &str, fallback: &str, max_len: usize) -> String {
    let candidate = clean(raw.to_string(), max_len)
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
        .collect::<String>();
    if candidate.is_empty() {
        fallback.to_string()
    } else {
        candidate
    }
}

fn parse_allowlist(raw: Option<&String>, default_item: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(v) = raw {
        for row in v.split([',', ' ', '\n', '\t']) {
            let token = normalize_token(row, "", 96);
            if !token.is_empty() {
                out.insert(token);
            }
        }
    }
    if out.is_empty() {
        out.insert(default_item.to_string());
    }
    out
}

fn judicial_lock_path(root: &Path) -> PathBuf {
    state_root(root).join("judicial_lock.json")
}

fn session_slab_dir(root: &Path) -> PathBuf {
    state_root(root).join("session_slabs")
}

// LAYER 0 SAFETY MICROKERNEL: typed syscalls + least privilege + circuit breakers.
fn run_microkernel_safety(
    root: &Path,
    strict: bool,
    syscall_raw: Option<&String>,
    session_raw: Option<&String>,
    allow_raw: Option<&String>,
    instance_dna_raw: Option<&String>,
    step_raw: Option<&String>,
    step_cap_raw: Option<&String>,
    drift_raw: Option<&String>,
    drift_threshold_raw: Option<&String>,
) -> Value {
    let requested_syscall = normalize_token(
        syscall_raw.map(String::as_str).unwrap_or("invoke_agent"),
        "invoke_agent",
        96,
    );
    let typed_syscall_ok = MICROKERNEL_TYPED_SYSCALLS
        .iter()
        .any(|id| *id == requested_syscall);

    let allowlist = parse_allowlist(allow_raw, &requested_syscall);
    let least_privilege_ok = typed_syscall_ok && allowlist.contains(&requested_syscall);

    let step_cap = parse_usize_flag(step_cap_raw, 128, 1_000_000);
    let step = parse_usize_flag(step_raw, 1, 1_000_000);
    let step_cap_exceeded = step > step_cap;

    let drift_threshold = parse_f64_flag(drift_threshold_raw, 0.05, 0.0, 1.0);
    let drift_score = parse_f64_flag(drift_raw, 0.0, 0.0, 1.0);
    let drift_threshold_exceeded = drift_score > drift_threshold;

    let session_id = normalize_token(
        session_raw.map(String::as_str).unwrap_or("session-default"),
        "session-default",
        96,
    );
    let instance_dna = normalize_token(
        instance_dna_raw
            .map(String::as_str)
            .unwrap_or("instance-dna-default"),
        "instance-dna-default",
        128,
    );
    let cryptographic_session_id = deterministic_receipt_hash(&json!({
        "session_id": session_id,
        "instance_dna": instance_dna,
        "syscall": requested_syscall
    }));

    let slab_dir = session_slab_dir(root);
    let slab_path = slab_dir.join(format!("{cryptographic_session_id}.json"));
    let _ = fs::create_dir_all(&slab_dir);
    let slab_payload = json!({
        "type": "microkernel_session_slab",
        "session_id": session_id,
        "instance_dna": instance_dna,
        "cryptographic_session_id": cryptographic_session_id,
        "allowed_syscalls": allowlist.iter().cloned().collect::<Vec<_>>(),
        "last_syscall": requested_syscall,
        "updated_at": now_iso(),
    });
    write_json(&slab_path, &slab_payload);

    let mut violation_codes = Vec::new();
    if !typed_syscall_ok {
        violation_codes.push("typed_syscall_unknown".to_string());
    }
    if !least_privilege_ok {
        violation_codes.push("least_privilege_denied".to_string());
    }
    if step_cap_exceeded {
        violation_codes.push("step_cap_exceeded".to_string());
    }
    if drift_threshold_exceeded {
        violation_codes.push("drift_threshold_exceeded".to_string());
    }

    let judicial_lock_triggered = strict && !violation_codes.is_empty();
    let lock_path = judicial_lock_path(root);
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let lock_payload = json!({
        "type": "metakernel_judicial_lock",
        "active": judicial_lock_triggered,
        "trigger": "microkernel_safety",
        "ts": now_iso(),
        "session_id": session_id,
        "instance_dna": instance_dna,
        "cryptographic_session_id": cryptographic_session_id,
        "syscall": requested_syscall,
        "step": step,
        "step_cap": step_cap,
        "drift_score": drift_score,
        "drift_threshold": drift_threshold,
        "violation_codes": violation_codes,
    });
    write_json(&lock_path, &lock_payload);

    let all_checks_ok =
        typed_syscall_ok && least_privilege_ok && !step_cap_exceeded && !drift_threshold_exceeded;
    json!({
        "ok": if strict { all_checks_ok } else { true },
        "strict": strict,
        "syscall": {
            "requested": requested_syscall,
            "typed": typed_syscall_ok,
            "table": MICROKERNEL_TYPED_SYSCALLS,
        },
        "least_privilege": {
            "allowed": least_privilege_ok,
            "allowlist": allowlist.into_iter().collect::<Vec<_>>(),
        },
        "circuit_breakers": {
            "step": step,
            "step_cap": step_cap,
            "step_cap_exceeded": step_cap_exceeded,
            "drift_score": drift_score,
            "drift_threshold": drift_threshold,
            "drift_threshold_exceeded": drift_threshold_exceeded,
        },
        "session_isolation": {
            "session_id": session_id,
            "instance_dna": instance_dna,
            "cryptographic_session_id": cryptographic_session_id,
            "private_memory_slab_path": slab_path.to_string_lossy().to_string(),
        },
        "judicial_lock": {
            "triggered": judicial_lock_triggered,
            "lock_path": lock_path.to_string_lossy().to_string(),
            "violation_codes": lock_payload
                .get("violation_codes")
                .cloned()
                .unwrap_or_else(|| json!([])),
        }
    })
}
