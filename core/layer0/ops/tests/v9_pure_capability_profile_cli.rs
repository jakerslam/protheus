use serde_json::Value;
use std::process::Command;

fn assert_no_runtime_context_leak(raw: &str) {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    for marker in FORBIDDEN {
        assert!(
            !raw.contains(marker),
            "runtime payload leaked forbidden marker `{marker}`: {raw}"
        );
    }
}

fn parse_json_output(stdout: &[u8], stderr: &[u8]) -> Value {
    let stdout_text = String::from_utf8_lossy(stdout);
    assert_no_runtime_context_leak(stdout_text.as_ref());
    if let Ok(value) = serde_json::from_str::<Value>(stdout_text.trim()) {
        return value;
    }
    let stderr_text = String::from_utf8_lossy(stderr);
    assert_no_runtime_context_leak(stderr_text.as_ref());
    if let Ok(value) = serde_json::from_str::<Value>(stderr_text.trim()) {
        return value;
    }
    panic!(
        "expected json output; stdout=`{}` stderr=`{}`",
        stdout_text, stderr_text
    );
}

fn run_infringd(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR")))
        .arg("--bin")
        .arg("infringd")
        .arg("--")
        .args(args)
        .output()
        .expect("run infringd")
}

#[test]
fn capability_profile_reports_mcu_shedding() {
    let output = run_infringd(&[
        "capability-profile",
        "--hardware-class=mcu",
        "--tiny-max=1",
        "--memory-mb=256",
        "--cpu-cores=1",
    ]);
    assert!(
        output.status.success(),
        "expected success, got status {:?}",
        output.status.code()
    );
    let payload = parse_json_output(&output.stdout, &output.stderr);
    assert_eq!(
        payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "infringd_capability_profile"
    );
    assert_eq!(
        payload
            .pointer("/profile/hardware_class")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "mcu"
    );
    assert_eq!(
        payload
            .pointer("/profile/capabilities/research_fetch")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        false
    );
}

#[test]
fn microcontroller_profile_blocks_heavy_orchestration_op() {
    let output = run_infringd(&[
        "orchestration",
        "invoke",
        "--op=coordinator.run",
        "--payload-json={\"items\":[1,2],\"agent_count\":2}",
        "--hardware-class=mcu",
        "--tiny-max=1",
    ]);
    assert!(
        !output.status.success(),
        "expected failure for heavy mcu op"
    );
    let payload = parse_json_output(&output.stdout, &output.stderr);
    assert_eq!(
        payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "infringd_error"
    );
    assert!(
        payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("hardware_profile_blocks_orchestration_op"),
        "unexpected payload: {payload}"
    );
}

#[test]
fn microcontroller_profile_blocks_swarm_depth_overflow() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("swarm.json");

    let state_path_owned = state_path.to_string_lossy().to_string();
    let output = run_infringd(&[
        "swarm-runtime",
        "spawn",
        "--task=test",
        "--max-depth=3",
        "--hardware-class=mcu",
        "--state-path",
        state_path_owned.as_str(),
    ]);
    assert!(
        !output.status.success(),
        "expected failure for mcu max-depth overflow"
    );
    let payload = parse_json_output(&output.stdout, &output.stderr);
    assert_eq!(
        payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "infringd_error"
    );
    assert!(
        payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("hardware_profile_max_swarm_depth_exceeded"),
        "unexpected payload: {payload}"
    );
}
