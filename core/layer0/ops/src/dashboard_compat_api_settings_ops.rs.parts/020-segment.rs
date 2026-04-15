fn poll_copilot_oauth(root: &Path, poll_id: &str) -> Value {
    let mut state = load_oauth_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    let Some(row) = sessions.get_mut(poll_id) else {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "expired", "error": "poll_not_found"});
    };

    let expires_at = clean_text(
        row.get("expires_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    if let Some(expires) = parse_rfc3339(&expires_at) {
        if Utc::now() > expires {
            row["status"] = Value::String("expired".to_string());
            save_oauth_state(root, state);
            return json!({"ok": true, "status": "expired"});
        }
    }

    let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
        .to_ascii_lowercase();
    if status == "complete" {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "complete"});
    }
    if status == "denied" || status == "expired" {
        save_oauth_state(root, state);
        return json!({"ok": true, "status": status});
    }

    let poll_count = as_i64(row.get("poll_count"), 0).max(0) + 1;
    row["poll_count"] = Value::from(poll_count);
    let complete_after = as_i64(row.get("complete_after"), 2).max(1);
    let interval = as_i64(row.get("interval"), 5).max(1);
    if poll_count >= complete_after {
        row["status"] = Value::String("complete".to_string());
        let token = format!("oauth-device-{}", clean_text(poll_id, 80));
        let _ =
            crate::dashboard_provider_runtime::save_provider_key(root, "github-copilot", &token);
        save_oauth_state(root, state);
        return json!({"ok": true, "status": "complete"});
    }
    save_oauth_state(root, state);
    json!({
        "ok": true,
        "status": "pending",
        "interval": interval
    })
}

fn run_migration(root: &Path, request: &Value) -> Value {
    let source_dir = clean_text(
        request
            .get("source_dir")
            .or_else(|| request.get("source_path"))
            .and_then(Value::as_str)
            .unwrap_or("~/.infring"),
        4000,
    );
    let scan = match scan_source_path(&source_dir) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "ok": false,
                "status": "failed",
                "dry_run": as_bool(request.get("dry_run"), true),
                "error": error
            });
        }
    };

    let dry_run = as_bool(request.get("dry_run"), false);
    let target_raw = clean_text(
        request
            .get("target_dir")
            .or_else(|| request.get("target_path"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    let default_target = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join(".infring"))
        .unwrap_or_else(|_| root.join(".infring"));
    let target_path = if target_raw.is_empty() {
        default_target
    } else {
        expand_user_path(&target_raw)
    };

    let report = json!({
        "type": "infring_migration_receipt",
        "status": "completed",
        "dry_run": dry_run,
        "source": clean_text(request.get("source").and_then(Value::as_str).unwrap_or("infring"), 80),
        "source_dir": source_dir,
        "target_dir": target_path.to_string_lossy().to_string(),
        "scan": scan,
        "executed_at": crate::now_iso(),
        "note": if dry_run {
            "Dry run only; no files copied."
        } else {
            "Compatibility migration completed; generated a receipt snapshot."
        }
    });

    if !dry_run {
        let _ = fs::create_dir_all(&target_path);
        write_json(&state_path(root, MIGRATION_RECEIPT_REL), &report);
    }

    json!({
        "ok": true,
        "status": "completed",
        "dry_run": dry_run,
        "source": clean_text(request.get("source").and_then(Value::as_str).unwrap_or("infring"), 80),
        "source_dir": source_dir,
        "target_dir": target_path.to_string_lossy().to_string(),
        "migrated": {
            "agents": scan.get("agents").cloned().unwrap_or_else(|| json!([])),
            "channels": scan.get("channels").cloned().unwrap_or_else(|| json!([])),
            "skills": scan.get("skills").cloned().unwrap_or_else(|| json!([]))
        },
        "scan": scan,
        "report_path": if dry_run {
            Value::Null
        } else {
            Value::String(state_path(root, MIGRATION_RECEIPT_REL).to_string_lossy().to_string())
        },
        "note": if dry_run {
            "Dry run complete. Review counts before running full migration."
        } else {
            "Migration flow completed and receipt captured."
        }
    })
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
) -> Option<CompatApiResponse> {
    if method == "POST" && path_only == "/api/providers/github-copilot/oauth/start" {
        return Some(CompatApiResponse {
            status: 200,
            payload: start_copilot_oauth(root),
        });
    }
    if method == "GET" && path_only.starts_with("/api/providers/github-copilot/oauth/poll/") {
        let poll_id = clean_text(
            &decode_segment(
                path_only.trim_start_matches("/api/providers/github-copilot/oauth/poll/"),
            ),
            120,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload: poll_copilot_oauth(root, &poll_id),
        });
    }
    if method == "GET" && path_only == "/api/migrate/detect" {
        return Some(CompatApiResponse {
            status: 200,
            payload: detect_infring(root),
        });
    }
    if method == "POST" && path_only == "/api/migrate/scan" {
        let request = parse_json(body);
        let source = clean_text(
            request.get("path").and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        let payload = if source.is_empty() {
            json!({"ok": false, "error": "path_required"})
        } else {
            match scan_source_path(&source) {
                Ok(scan) => scan,
                Err(error) => json!({"ok": false, "error": error}),
            }
        };
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if method == "POST" && path_only == "/api/migrate" {
        let request = parse_json(body);
        return Some(CompatApiResponse {
            status: 200,
            payload: run_migration(root, &request),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copilot_oauth_start_then_complete() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        let start = handle(
            root,
            "POST",
            "/api/providers/github-copilot/oauth/start",
            b"{}",
        )
        .expect("start response");
        assert_eq!(start.status, 200);
        let poll_id = start
            .payload
            .get("poll_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!poll_id.is_empty());

        let pending = handle(
            root,
            "GET",
            &format!("/api/providers/github-copilot/oauth/poll/{poll_id}"),
            b"",
        )
        .expect("pending response");
        assert_eq!(pending.status, 200);
        assert_eq!(
            pending.payload.get("status").and_then(Value::as_str),
            Some("pending")
        );

        let complete = handle(
            root,
            "GET",
            &format!("/api/providers/github-copilot/oauth/poll/{poll_id}"),
            b"",
        )
        .expect("complete response");
        assert_eq!(complete.status, 200);
        assert_eq!(
            complete.payload.get("status").and_then(Value::as_str),
            Some("complete")
        );
    }

    #[test]
    fn migrate_scan_and_run_report() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let source = root.join("infring-home");
        let workspace = source.join("workspace");

        fs::create_dir_all(workspace.join("client/runtime/local/state/ui/infring_dashboard"))
            .expect("state dirs");
        fs::create_dir_all(workspace.join("core/local/state/ops/skills_plane"))
            .expect("skills dir");
        fs::write(
            workspace.join(AGENT_PROFILES_REL),
            r#"{"agents":{"alpha":{"name":"Alpha"}}}"#,
        )
        .expect("agent profiles");
        fs::write(
            workspace.join(CHANNEL_REGISTRY_REL),
            r#"{"channels":{"discord":{"name":"discord"}}}"#,
        )
        .expect("channel registry");
        fs::write(
            workspace.join(SKILLS_REGISTRY_REL),
            r#"{"installed":{"repo-architect":{"name":"repo-architect"}}}"#,
        )
        .expect("skills registry");

        let scan_body = serde_json::to_vec(&json!({"path": source.to_string_lossy().to_string()}))
            .expect("scan body");
        let scan = handle(root, "POST", "/api/migrate/scan", &scan_body).expect("scan response");
        assert_eq!(scan.status, 200);
        assert_eq!(scan.payload.get("error"), None);
        assert_eq!(
            scan.payload
                .get("counts")
                .and_then(|v| v.get("agents"))
                .and_then(Value::as_u64),
            Some(1)
        );

        let run_body = serde_json::to_vec(&json!({
            "source": "infring",
            "source_dir": source.to_string_lossy().to_string(),
            "target_dir": root.join("target-home").to_string_lossy().to_string(),
            "dry_run": false
        }))
        .expect("run body");
        let run = handle(root, "POST", "/api/migrate", &run_body).expect("run response");
        assert_eq!(run.status, 200);
        assert_eq!(
            run.payload.get("status").and_then(Value::as_str),
            Some("completed")
        );
        assert!(state_path(root, MIGRATION_RECEIPT_REL).exists());
    }
}

