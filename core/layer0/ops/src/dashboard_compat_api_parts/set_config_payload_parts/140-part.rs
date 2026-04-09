// FILE_SIZE_EXCEPTION: reason=Function-scale route/tool block retained atomically during staged decomposition; owner=jay; expires=2026-04-22
fn summarize_tool_payload(tool_name: &str, payload: &Value) -> String {
    fn summary_excluded_key(key: &str) -> bool {
        matches!(
            key,
            "screenshotBase64"
                | "content_base64"
                | "raw_html"
                | "html"
                | "raw_content"
                | "payload"
                | "response_finalization"
                | "turn_loop_tracking"
                | "turn_transaction"
                | "workspace_hints"
                | "latent_tool_candidates"
                | "nexus_connection"
        )
    }

    fn scalar_summary_fragment(value: &Value) -> Option<String> {
        match value {
            Value::String(raw) => {
                let trimmed = clean_text(raw, 160);
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
            Value::Bool(raw) => Some(if *raw { "true" } else { "false" }.to_string()),
            Value::Number(raw) => Some(raw.to_string()),
            _ => None,
        }
    }

    fn summarize_unknown_tool_payload(normalized: &str, payload: &Value) -> String {
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return user_facing_tool_failure_summary(normalized, payload)
                .unwrap_or_else(|| format!("I couldn't complete `{normalized}` right now."));
        }
        if let Some(response) = payload.get("response").and_then(Value::as_str) {
            let candidate = clean_text(response, 1_400);
            if !candidate.is_empty()
                && !response_looks_like_tool_ack_without_findings(&candidate)
                && !response_looks_like_raw_web_artifact_dump(&candidate)
            {
                if let Some(unwrapped) = normalize_raw_response_payload_dump(&candidate) {
                    return trim_text(&unwrapped, 1_400);
                }
                return trim_text(&candidate, 1_400);
            }
        }
        if let Some(summary) = payload.get("summary").and_then(Value::as_str) {
            let candidate = clean_text(summary, 1_200);
            if !candidate.is_empty() && !response_looks_like_tool_ack_without_findings(&candidate) {
                return trim_text(&candidate, 1_200);
            }
        }
        let mut fields = Vec::<String>::new();
        if let Some(obj) = payload.as_object() {
            for (key, value) in obj {
                if key == "ok" || summary_excluded_key(key.as_str()) {
                    continue;
                }
                if let Some(fragment) = scalar_summary_fragment(value) {
                    fields.push(format!("{}={}", clean_text(key, 40), fragment));
                } else if let Some(rows) = value.as_array() {
                    if !rows.is_empty() {
                        fields.push(format!("{} count={}", clean_text(key, 40), rows.len()));
                    }
                }
                if fields.len() >= 3 {
                    break;
                }
            }
        }
        if fields.is_empty() {
            return format!("`{normalized}` completed. See tool details for structured output.");
        }
        trim_text(
            &format!(
                "`{normalized}` completed with {}.",
                clean_text(&fields.join(", "), 220)
            ),
            1_000,
        )
    }

    let normalized = normalize_tool_name(tool_name);
    if let Some(claims) = payload
        .pointer("/tool_pipeline/claim_bundle/claims")
        .and_then(Value::as_array)
    {
        let mut findings = claims
            .iter()
            .filter_map(|claim| {
                let status = clean_text(
                    claim.get("status").and_then(Value::as_str).unwrap_or(""),
                    40,
                )
                .to_ascii_lowercase();
                if status != "supported" && status != "partial" {
                    return None;
                }
                let text = clean_text(claim.get("text").and_then(Value::as_str).unwrap_or(""), 260);
                if text.is_empty() {
                    None
                } else {
                    Some(trim_text(&text, 220))
                }
            })
            .take(3)
            .collect::<Vec<_>>();
        if !findings.is_empty() {
            findings.retain(|row| !row.trim().is_empty());
            if !findings.is_empty() {
                return trim_text(&format!("Key findings: {}", findings.join(" | ")), 24_000);
            }
        }
    }
    if normalized == "spawn_subagents"
        || normalized == "spawn_swarm"
        || normalized == "agent_spawn"
        || normalized == "sessions_spawn"
    {
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return user_facing_tool_failure_summary(tool_name, payload)
                .unwrap_or_else(|| "I couldn't start parallel agents in this turn.".to_string());
        }
        let created_count = payload
            .get("created_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let requested_count = payload
            .get("requested_count")
            .and_then(Value::as_u64)
            .unwrap_or(created_count);
        let receipt = clean_text(
            payload
                .pointer("/directive/receipt")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let ids = payload
            .get("children")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("agent_id").and_then(Value::as_str))
                    .map(|row| clean_text(row, 60))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut summary = format!("Spawned {created_count}/{requested_count} descendant agents.");
        if !ids.is_empty() {
            summary.push_str(&format!(" IDs: {}.", ids.join(", ")));
        }
        if !receipt.is_empty() {
            summary.push_str(&format!(" Directive receipt: {receipt}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "memory_semantic_query" || normalized == "memory_query" {
        let query = clean_text(
            payload.get("query").and_then(Value::as_str).unwrap_or(""),
            200,
        );
        let matches = payload
            .get("matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if matches.is_empty() {
            if query.is_empty() {
                return "No semantic memory matches found.".to_string();
            }
            return trim_text(
                &format!("No semantic memory matches for `{query}`."),
                24_000,
            );
        }
        let mut lines = Vec::<String>::new();
        if query.is_empty() {
            lines.push("Semantic memory matches:".to_string());
        } else {
            lines.push(format!("Semantic memory matches for `{query}`:"));
        }
        for row in matches.into_iter().take(5) {
            let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 160);
            let snippet = clean_text(
                row.get("snippet").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            let score = row.get("score").and_then(Value::as_i64).unwrap_or(0);
            if key.is_empty() {
                continue;
            }
            if snippet.is_empty() {
                lines.push(format!("- {key} (score {score})"));
            } else {
                lines.push(format!("- {key} (score {score}): {snippet}"));
            }
        }
        return trim_text(&lines.join("\n"), 24_000);
    }
    if normalized == "cron_schedule" || normalized == "schedule_task" || normalized == "cron_create"
    {
        let job_id = clean_text(
            payload
                .pointer("/job/id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("job_id").and_then(Value::as_str))
                .unwrap_or(""),
            140,
        );
        let name = clean_text(
            payload
                .pointer("/job/name")
                .and_then(Value::as_str)
                .unwrap_or("scheduled-job"),
            180,
        );
        let next_run = clean_text(
            payload
                .pointer("/job/next_run")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let mut summary = format!("Scheduled cron job `{}`.", name);
        if !job_id.is_empty() {
            summary.push_str(&format!(" ID: {job_id}."));
        }
        if !next_run.is_empty() {
            summary.push_str(&format!(" Next run: {next_run}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "cron_cancel" || normalized == "cron_delete" || normalized == "schedule_cancel"
    {
        if payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
            && payload
                .get("deleted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            let job_id = clean_text(
                payload.get("job_id").and_then(Value::as_str).unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return "Deleted cron job.".to_string();
            }
            return format!("Deleted cron job `{job_id}`.");
        }
    }
    if normalized == "cron_run" || normalized == "schedule_run" || normalized == "cron_trigger" {
        if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let job_id = clean_text(
                payload.get("job_id").and_then(Value::as_str).unwrap_or(""),
                140,
            );
            if job_id.is_empty() {
                return "Ran scheduled job successfully.".to_string();
            }
            return format!("Ran scheduled job `{job_id}`.");
        }
    }
    if normalized == "cron_list" || normalized == "schedule_list" || normalized == "cron_jobs" {
        let jobs = payload
            .get("jobs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut names = jobs
            .iter()
            .take(4)
            .filter_map(|row| row.get("name").and_then(Value::as_str))
            .map(|name| clean_text(name, 80))
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        names.dedup();
        let mut summary = format!("Cron jobs available: {}.", jobs.len());
        if !names.is_empty() {
            summary.push_str(&format!(" {}", names.join(", ")));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "session_rollback_last_turn"
        || normalized == "undo_last_turn"
        || normalized == "rewind_turn"
    {
        let removed = payload
            .get("removed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if removed == 0 {
            return "No recent turn available to undo.".to_string();
        }
        let rollback_id = clean_text(
            payload
                .get("rollback_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let mut summary = format!("Undid the most recent turn (removed {removed} messages).");
        if !rollback_id.is_empty() {
            summary.push_str(&format!(" Rollback receipt: {rollback_id}."));
        }
        return trim_text(&summary, 24_000);
    }
    if normalized == "file_read" || normalized == "read_file" || normalized == "file" {
        let content = payload
            .pointer("/file/content")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !content.is_empty() {
            return trim_text(content, 24_000);
        }
        if payload
            .pointer("/file/binary")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let bytes = payload
                .pointer("/file/bytes")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let mime = clean_text(
                payload
                    .pointer("/file/content_type")
                    .and_then(Value::as_str)
                    .unwrap_or("application/octet-stream"),
                120,
            );
            let file_name = clean_text(
                payload
                    .pointer("/file/file_name")
                    .and_then(Value::as_str)
                    .unwrap_or("binary file"),
                180,
            );
            return trim_text(
                format!(
                    "Read binary file `{file_name}` ({mime}, {bytes} bytes). Use `allow_binary=true` to retrieve `content_base64`."
                )
                .as_str(),
                420,
            );
        }
    }
    if normalized == "file_read_many"
        || normalized == "read_files"
        || normalized == "files_read"
        || normalized == "batch_file_read"
    {
        let files = payload
            .get("files")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let failed = payload
            .get("failed")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut sections = Vec::<String>::new();
        for entry in files.iter().take(3) {
            let path = clean_text(entry.get("path").and_then(Value::as_str).unwrap_or(""), 220);
            let content = clean_text(
                entry.get("content").and_then(Value::as_str).unwrap_or(""),
                4_000,
            );
            if content.is_empty() {
                continue;
            }
            sections.push(format!(
                "[{}]\n{}",
                if path.is_empty() {
                    "file".to_string()
                } else {
                    path
                },
                content
            ));
        }
        if !sections.is_empty() {
            return trim_text(sections.join("\n\n").as_str(), 24_000);
        }
        if !files.is_empty() || !failed.is_empty() {
            return trim_text(
                format!(
                    "Batch file read finished: {} succeeded, {} failed.",
                    files.len(),
                    failed.len()
                )
                .as_str(),
                420,
            );
        }
    }
    if normalized == "folder_export"
        || normalized == "list_folder"
        || normalized == "folder_tree"
        || normalized == "folder"
    {
        let tree = payload
            .pointer("/folder/tree")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !tree.is_empty() {
            return trim_text(tree, 24_000);
        }
    }
    if normalized == "terminal_exec"
        || normalized == "run_terminal"
        || normalized == "terminal"
        || normalized == "shell_exec"
    {
        let stdout = payload.get("stdout").and_then(Value::as_str).unwrap_or("");
        let stderr = payload.get("stderr").and_then(Value::as_str).unwrap_or("");
        let merged = if stderr.is_empty() {
            stdout.to_string()
        } else if stdout.is_empty() {
            stderr.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };
        if !merged.trim().is_empty() {
            return trim_text(&merged, 24_000);
        }
    }
    if normalized == "web_fetch" || normalized == "browse" || normalized == "web_conduit_fetch" {
        let summary = summarize_web_fetch_payload(payload);
        if !summary.is_empty() {
            return trim_text(&summary, 1_200);
        }
    }
    if normalized == "batch_query" || normalized == "batch-query" {
        let status = clean_text(
            payload.get("status").and_then(Value::as_str).unwrap_or(""),
            40,
        )
        .to_ascii_lowercase();
        let summary = clean_text(
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            2400,
        );
        if status == "blocked" {
            if !summary.is_empty() {
                return trim_text(&summary, 1200);
            }
            return "Batch query was blocked by policy.".to_string();
        }
        if !summary.is_empty()
            && !response_looks_like_tool_ack_without_findings(&summary)
            && !response_looks_like_raw_web_artifact_dump(&summary)
            && !response_looks_like_unsynthesized_web_snippet_dump(&summary)
        {
            return trim_text(&summary, 1200);
        }
        let evidence_refs = payload
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !evidence_refs.is_empty() {
            let mut lines = vec!["Batch query evidence:".to_string()];
            for row in evidence_refs.into_iter().take(4) {
                let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 180);
                let locator = clean_text(
                    row.get("locator").and_then(Value::as_str).unwrap_or(""),
                    220,
                );
                if title.is_empty() && locator.is_empty() {
                    continue;
                }
                if locator.is_empty() {
                    lines.push(format!("- {title}"));
                } else if title.is_empty() {
                    lines.push(format!("- {locator}"));
                } else {
                    lines.push(format!("- {title} ({locator})"));
                }
            }
            return trim_text(&lines.join("\n"), 1200);
        }
        if status == "no_results" {
            return no_findings_user_facing_response();
        }
        return "Search returned no useful information.".to_string();
    }
    if normalized == "web_search"
        || normalized == "search_web"
        || normalized == "search"
        || normalized == "web_query"
    {
        if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return user_facing_tool_failure_summary(tool_name, payload)
                .unwrap_or_else(|| "Web search couldn't complete right now.".to_string());
        }
        let query = clean_text(
            payload.get("query").and_then(Value::as_str).unwrap_or(""),
            220,
        );
        let summary = clean_text(
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            2_400,
        );
        let content = clean_text(
            payload.get("content").and_then(Value::as_str).unwrap_or(""),
            2_400,
        );
        let requested_url = clean_text(
            payload
                .get("requested_url")
                .or_else(|| payload.pointer("/receipt/requested_url"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            220,
        );
        let domain = clean_text(
            payload.get("domain").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if !summary.is_empty()
            && !looks_like_search_engine_chrome_summary(&summary)
            && !response_looks_like_tool_ack_without_findings(&summary)
        {
            return trim_text(&summary, 1_200);
        }
        let combined = if content.is_empty() {
            summary.clone()
        } else if summary.is_empty() {
            content.clone()
        } else {
            format!("{summary}\n{content}")
        };
        let findings = extract_search_result_findings(&combined, 3);
        if !findings.is_empty() {
            let findings_lines = findings
                .iter()
                .map(|row| format!("- {row}"))
                .collect::<Vec<_>>()
                .join("\n");
            let findings_summary =
                trim_text(&format!("Key web findings:\n{findings_lines}"), 1_200);
            if !response_looks_like_unsynthesized_web_snippet_dump(&findings_summary) {
                return findings_summary;
            }
        }
        let sources = extract_search_result_domains(&combined, 4);
        if !sources.is_empty() {
            let joined = sources.join(", ");
            return web_search_no_findings_fallback(
                &query,
                &format!("{combined}\n{joined}"),
                &requested_url,
                &domain,
            );
        }
        return web_search_no_findings_fallback(&query, &combined, &requested_url, &domain);
    }
    summarize_unknown_tool_payload(&normalized, payload)
}

