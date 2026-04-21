fn handle_global_receipts_lineage_get(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
) -> Option<CompatApiResponse> {
    if method != "GET" || path_only != "/api/receipts/lineage" {
        return None;
    }
    let task_id = clean_text(
        query_value(path, "task_id")
            .or_else(|| query_value(path, "taskId"))
            .as_deref()
            .unwrap_or(""),
        180,
    );
    if task_id.is_empty() {
        return Some(CompatApiResponse {
            status: 400,
            payload: json!({
                "ok": false,
                "error": "task_id_required"
            }),
        });
    }
    let trace_id = clean_text(
        query_value(path, "trace_id")
            .or_else(|| query_value(path, "traceId"))
            .as_deref()
            .unwrap_or(""),
        180,
    );
    let trace_opt = if trace_id.is_empty() {
        None
    } else {
        Some(trace_id.as_str())
    };
    let limit = query_value(path, "limit")
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(4000)
        .clamp(1, 50_000);
    let scan_root = clean_text(
        query_value(path, "scan_root")
            .or_else(|| query_value(path, "scanRoot"))
            .as_deref()
            .unwrap_or(""),
        500,
    );
    let scan_root_path = if scan_root.is_empty() {
        None
    } else {
        let candidate = PathBuf::from(scan_root);
        Some(if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        })
    };
    let sources = clean_text(
        query_value(path, "sources")
            .or_else(|| query_value(path, "sourcesCsv"))
            .as_deref()
            .unwrap_or(""),
        4_000,
    );
    let sources_opt = if sources.is_empty() {
        None
    } else {
        Some(sources.as_str())
    };
    let payload = match crate::action_receipts_kernel::query_task_lineage(
        root,
        &task_id,
        trace_opt,
        limit,
        scan_root_path.as_deref(),
        sources_opt,
    ) {
        Ok(out) => out,
        Err(err) => {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "error": clean_text(&err, 240)
                }),
            })
        }
    };
    Some(CompatApiResponse {
        status: 200,
        payload,
    })
}
