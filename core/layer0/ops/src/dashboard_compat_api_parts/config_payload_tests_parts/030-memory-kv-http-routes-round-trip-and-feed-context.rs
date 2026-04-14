
#[test]
fn memory_kv_http_routes_round_trip_and_feed_context_pool() {
    let root = tempfile::tempdir().expect("tempdir");
    init_git_repo(root.path());
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Memory KV Probe","role":"analyst"}"#,
        &json!({"ok": true}),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let set = handle(
        root.path(),
        "PUT",
        &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
        br#"{"value":"reliability"}"#,
        &json!({"ok": true}),
    )
    .expect("set memory kv");
    assert_eq!(set.status, 200);

    let listed = handle(
        root.path(),
        "GET",
        &format!("/api/memory/agents/{agent_id}/kv"),
        &[],
        &json!({"ok": true}),
    )
    .expect("list memory kv");
    assert_eq!(listed.status, 200);
    let keys = listed
        .payload
        .get("kv_pairs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|row| row.get("key").and_then(Value::as_str))
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    assert!(keys.iter().any(|key| key == "focus.topic"));

    let got = handle(
        root.path(),
        "GET",
        &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
        &[],
        &json!({"ok": true}),
    )
    .expect("get memory kv");
    assert_eq!(got.status, 200);
    assert_eq!(
        got.payload.get("value").and_then(Value::as_str),
        Some("reliability")
    );

    let semantic = handle(
        root.path(),
        "GET",
        &format!("/api/memory/agents/{agent_id}/semantic-query?q=reliability&limit=4"),
        &[],
        &json!({"ok": true}),
    )
    .expect("semantic memory query");
    assert_eq!(semantic.status, 200);
    assert!(semantic
        .payload
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));

    let message = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use stored memory if present."}"#,
        &json!({"ok": true}),
    )
    .expect("message with memory kv");
    assert_eq!(message.status, 200);
    assert!(
        message
            .payload
            .pointer("/context_pool/memory_kv_entries")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= 1
    );

    let deleted = handle(
        root.path(),
        "DELETE",
        &format!("/api/memory/agents/{agent_id}/kv/focus.topic"),
        &[],
        &json!({"ok": true}),
    )
    .expect("delete memory kv");
    assert_eq!(deleted.status, 200);
    assert_eq!(
        deleted.payload.get("removed").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn agents_routes_terminal_and_artifact_endpoints_round_trip() {
    let root = tempfile::tempdir().expect("tempdir");
    let notes_dir = root.path().join("notes");
    let _ = fs::create_dir_all(&notes_dir);
    let _ = fs::write(notes_dir.join("plan.txt"), "ship it");
    let _ = fs::create_dir_all(notes_dir.join("sub"));
    let _ = fs::write(notes_dir.join("sub").join("extra.txt"), "plus one");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
        &json!({"ok": true}),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let file_read = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt"}"#,
        &json!({"ok": true}),
    )
    .expect("file read");
    assert_eq!(
        file_read
            .payload
            .pointer("/file/ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        file_read
            .payload
            .pointer("/file/content")
            .and_then(Value::as_str),
        Some("ship it")
    );
    let file_read_limited = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt","max_bytes":4}"#,
        &json!({"ok": true}),
    )
    .expect("file read limited");
    assert_eq!(
        file_read_limited
            .payload
            .pointer("/file/truncated")
            .and_then(Value::as_bool),
        Some(true)
    );

    let file_read_full = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt","max_bytes":4,"full":true}"#,
        &json!({"ok": true}),
    )
    .expect("file read full");
    assert_eq!(
        file_read_full
            .payload
            .pointer("/file/truncated")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        file_read_full
            .payload
            .pointer("/file/content")
            .and_then(Value::as_str),
        Some("ship it")
    );

    let _ = fs::write(
        notes_dir.join("blob.bin"),
        vec![0_u8, 159, 10, 11, 12, 255, 0, 1, 2, 3],
    );
    let binary_blocked = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/blob.bin"}"#,
        &json!({"ok": true}),
    )
    .expect("binary read blocked");
    assert_eq!(binary_blocked.status, 415);
    assert_eq!(
        binary_blocked.payload.get("error").and_then(Value::as_str),
        Some("binary_file_requires_opt_in")
    );

    let binary_allowed = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/blob.bin","allow_binary":true,"max_bytes":8}"#,
        &json!({"ok": true}),
    )
    .expect("binary read allowed");
    assert_eq!(binary_allowed.status, 200);
    assert_eq!(
        binary_allowed
            .payload
            .pointer("/file/binary")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(!binary_allowed
        .payload
        .pointer("/file/content_base64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty());

    let file_read_many = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read-many"),
        br#"{"paths":["notes/plan.txt","notes/blob.bin"],"allow_binary":false,"max_bytes":16}"#,
        &json!({"ok": true}),
    )
    .expect("file read many");
    assert_eq!(file_read_many.status, 200);
    assert_eq!(file_read_many.payload.pointer("/counts/ok").and_then(Value::as_u64), Some(1));
    assert_eq!(file_read_many.payload.pointer("/counts/failed").and_then(Value::as_u64), Some(1));
    assert_eq!(file_read_many.payload.pointer("/files/0/content").and_then(Value::as_str), Some("ship it"));
    assert_eq!(file_read_many.payload.pointer("/failed/0/error").and_then(Value::as_str), Some("binary_file_requires_opt_in"));
    assert_eq!(file_read_many.payload.pointer("/counts/text").and_then(Value::as_u64), Some(1));
    assert_eq!(file_read_many.payload.pointer("/counts/binary").and_then(Value::as_u64), Some(1));
    assert_eq!(
        file_read_many
            .payload
            .pointer("/groups/text/0")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("plan.txt"),
        true
    );

    let file_read_many_binary = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read-many"),
        br#"{"paths":["notes/blob.bin"],"allow_binary":true,"max_bytes":8}"#,
        &json!({"ok": true}),
    )
    .expect("file read many binary");
    assert_eq!(file_read_many_binary.status, 200);
    assert_eq!(file_read_many_binary.payload.pointer("/counts/ok").and_then(Value::as_u64), Some(1));
    assert!(!file_read_many_binary
        .payload
        .pointer("/files/0/content_base64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty());
    assert_eq!(file_read_many_binary.payload.pointer("/counts/binary").and_then(Value::as_u64), Some(1));

    let folder_export = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/folder/export"),
        br#"{"path":"notes"}"#,
        &json!({"ok": true}),
    )
    .expect("folder export");
    assert_eq!(folder_export.payload.pointer("/folder/ok").and_then(Value::as_bool), Some(true));
    assert!(folder_export
        .payload
        .pointer("/folder/tree")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("plan.txt"));
    assert!(folder_export
        .payload
        .pointer("/folder/tree")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("extra.txt"));

    let folder_export_limited = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/folder/export"),
        br#"{"path":"notes","max_entries":1}"#,
        &json!({"ok": true}),
    )
    .expect("folder export limited");
    assert_eq!(folder_export_limited.payload.pointer("/folder/truncated").and_then(Value::as_bool), Some(true));

    let folder_export_full = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/folder/export"),
        br#"{"path":"notes","max_entries":1,"full":true}"#,
        &json!({"ok": true}),
    )
    .expect("folder export full");
    assert_eq!(folder_export_full.payload.pointer("/folder/truncated").and_then(Value::as_bool), Some(false));
    assert!(folder_export_full
        .payload
        .pointer("/folder/tree")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("extra.txt"));

    let terminal = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/terminal"),
        br#"{"command":"printf 'ok'","cwd":"notes"}"#,
        &json!({"ok": true}),
    )
    .expect("terminal");
    assert_eq!(terminal.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(terminal.payload.get("stdout").and_then(Value::as_str), Some("ok"));

    let upload = handle_with_headers(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/upload"),
        b"voice",
        &[("X-Filename", "voice.webm"), ("Content-Type", "audio/webm")],
        &json!({"ok": true}),
    )
    .expect("upload");
    assert_eq!(upload.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert!(!clean_text(
        upload
            .payload
            .get("file_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180
    )
    .is_empty());
    assert_eq!(upload.payload.get("filename").and_then(Value::as_str), Some("voice.webm"));
}

#[test]
fn full_mode_overrides_file_and_folder_limits() {
    let root = tempfile::tempdir().expect("tempdir");
    let notes_dir = root.path().join("notes");
    let _ = fs::create_dir_all(notes_dir.join("sub"));
    let _ = fs::write(notes_dir.join("plan.txt"), "ship it");
    let _ = fs::write(notes_dir.join("sub").join("extra.txt"), "plus one");

    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"Ops","role":"operator"}"#,
        &json!({"ok": true}),
    )
    .expect("create agent");
    let agent_id = clean_text(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    assert!(!agent_id.is_empty());

    let file_read_limited = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt","max_bytes":4}"#,
        &json!({"ok": true}),
    )
    .expect("file read limited");
    assert_eq!(file_read_limited.payload.pointer("/file/truncated").and_then(Value::as_bool), Some(true));

    let file_read_full = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt","max_bytes":4,"full":true}"#,
        &json!({"ok": true}),
    )
    .expect("file read full");
    assert_eq!(file_read_full.payload.pointer("/file/truncated").and_then(Value::as_bool), Some(false));

    let folder_export_limited = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/folder/export"),
        br#"{"path":"notes","max_entries":1}"#,
        &json!({"ok": true}),
    )
    .expect("folder export limited");
    assert_eq!(folder_export_limited.payload.pointer("/folder/truncated").and_then(Value::as_bool), Some(true));

    let folder_export_full = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/folder/export"),
        br#"{"path":"notes","max_entries":1,"full":true}"#,
        &json!({"ok": true}),
    )
    .expect("folder export full");
    assert_eq!(folder_export_full.payload.pointer("/folder/truncated").and_then(Value::as_bool), Some(false));
}
