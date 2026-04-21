
fn run_npm_script(root: &Path, script: &str, args: &[String]) -> Value {
    let mut command = Command::new("npm");
    command
        .arg("run")
        .arg("-s")
        .arg(script);
    if !args.is_empty() {
        command.arg("--");
        for arg in args {
            command.arg(arg);
        }
    }
    let output = command.current_dir(root).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            json!({
                "ok": out.status.success(),
                "status": out.status.code().unwrap_or(-1),
                "stdout": clean(&stdout, 4000),
                "stderr": clean(&stderr, 4000)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status": -1,
            "stdout": "",
            "stderr": clean(&format!("spawn_failed:{err}"), 4000)
        }),
    }
}

fn run_dynamic_legacy_lane(root: &Path, id: &str) -> Value {
    let adapter = root
        .join("client")
        .join("runtime")
        .join("systems")
        .join("compat")
        .join("legacy_alias_adapter.ts");
    if !adapter.exists() {
        return json!({
            "ok": false,
            "status": -1,
            "stdout": "",
            "stderr": "dynamic_legacy_adapter_missing"
        });
    }
    let output = Command::new("node")
        .arg(adapter)
        .arg("run")
        .arg(format!("--lane-id={id}"))
        .arg("--strict=1")
        .current_dir(root)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            json!({
                "ok": out.status.success(),
                "status": out.status.code().unwrap_or(-1),
                "stdout": clean(&stdout, 4000),
                "stderr": clean(&stderr, 4000),
                "route": "dynamic_legacy_adapter"
            })
        }
        Err(err) => json!({
            "ok": false,
            "status": -1,
            "stdout": "",
            "stderr": clean(&format!("spawn_failed:{err}"), 4000),
            "route": "dynamic_legacy_adapter"
        }),
    }
}

fn run_core_runtime_system_lane(root: &Path, id: &str) -> Value {
    match runtime_systems::execute_contract_lane(root, id, true, true) {
        Ok(receipt) => json!({
            "ok": true,
            "status": 0,
            "stdout": clean(&serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string()), 4000),
            "stderr": "",
            "route": "core_runtime_systems"
        }),
        Err(err) => json!({
            "ok": false,
            "status": 1,
            "stdout": "",
            "stderr": clean(&format!("runtime_system_contract_failed:{err}"), 4000),
            "route": "core_runtime_systems"
        }),
    }
}
