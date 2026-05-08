use crate::evidence_sanitizer::sanitize_text_for_evidence;
use crate::schemas::{EvidenceArtifactRef, NormalizedToolResult};
use serde_json::Value;

pub(crate) fn pick_artifact_refs(
    source: &Value,
    result: &NormalizedToolResult,
    source_ref: &str,
    source_location: &str,
) -> Vec<EvidenceArtifactRef> {
    let source_url_hint = looks_like_url(source_ref).then(|| source_ref.to_string());
    if let Some(rows) = source.as_array() {
        return rows
            .iter()
            .enumerate()
            .filter_map(|(idx, row)| {
                build_artifact_ref(
                    row,
                    result,
                    &format!("{source_location}[{idx}]"),
                    source_url_hint.as_deref(),
                )
            })
            .collect::<Vec<_>>();
    }
    build_artifact_ref(source, result, source_location, source_url_hint.as_deref())
        .into_iter()
        .collect::<Vec<_>>()
}

pub(crate) fn collect_text_fragments(source: &Value, fragments: &mut Vec<String>) {
    if let Some(text) = source.as_str() {
        if !looks_like_url(text) {
            let cleaned = clean_text(text, 240);
            if !cleaned.is_empty() {
                fragments.push(cleaned);
            }
        }
        return;
    }
    let Some(obj) = source.as_object() else {
        return;
    };
    if is_artifact_candidate(source) {
        return;
    }
    let type_name = obj
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if type_name == "text" {
        if let Some(text) = obj.get("text").and_then(Value::as_str) {
            if !looks_like_url(text) {
                let cleaned = clean_text(text, 240);
                if !cleaned.is_empty() {
                    fragments.push(cleaned);
                }
            }
        }
        return;
    }
    if let Some(text) = first_string(
        source,
        &[
            "excerpt",
            "snippet",
            "content",
            "text",
            "description",
            "message",
            "body",
            "title",
        ],
    ) {
        if !looks_like_url(text) {
            let cleaned = clean_text(text, 240);
            if !cleaned.is_empty() {
                fragments.push(cleaned);
            }
        }
    }
}

pub(crate) fn summarize_artifact_refs(
    artifact_refs: &[EvidenceArtifactRef],
    source_ref: &str,
) -> String {
    let mut summaries = artifact_refs
        .iter()
        .take(2)
        .map(describe_artifact_ref)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        return String::new();
    }
    let source_hint = artifact_refs
        .iter()
        .filter_map(|artifact| artifact.source_url.as_deref())
        .find(|value| looks_like_url(value))
        .or_else(|| looks_like_url(source_ref).then_some(source_ref));
    if let Some(source_url) = source_hint {
        summaries[0] = format!("{} from {}", summaries[0], clean_text(source_url, 140));
    }
    clean_text(&summaries.join("; "), 300)
}

pub(crate) fn pick_source_ref(source: &Value, result: &NormalizedToolResult) -> String {
    clean_text(
        find_source_ref_candidate(source)
            .unwrap_or_else(|| result.raw_ref.clone())
            .as_str(),
        2000,
    )
}

pub(crate) fn pick_source_scope(
    source: &Value,
    result: &NormalizedToolResult,
    source_ref: &str,
) -> String {
    if let Some(scope) = result
        .normalized_args
        .get("source_scope")
        .and_then(Value::as_str)
        .map(|value| clean_text(value, 200))
        .filter(|value| !value.is_empty())
    {
        return scope;
    }
    if let Some(scope) = first_string(
        source,
        &[
            "source_scope",
            "sourceScope",
            "adaptive_domain",
            "adaptiveDomain",
        ],
    )
    .map(|value| clean_text(value, 200))
    .filter(|value| !value.is_empty())
    {
        return scope;
    }
    derive_source_scope(source_ref)
}

fn build_artifact_ref(
    source: &Value,
    result: &NormalizedToolResult,
    source_location: &str,
    source_url_hint: Option<&str>,
) -> Option<EvidenceArtifactRef> {
    if !is_artifact_candidate(source) {
        return None;
    }
    let artifact_ref = explicit_artifact_ref(source)
        .unwrap_or_else(|| format!("{}#{}", result.raw_ref, source_location));
    let source_url = explicit_source_url(source)
        .or_else(|| source_url_hint.map(str::to_string))
        .filter(|value| !value.is_empty());
    Some(EvidenceArtifactRef {
        artifact_kind: infer_artifact_kind(source),
        artifact_ref: clean_text(&artifact_ref, 2000),
        mime_type: first_string(source, &["mime_type", "mimeType"])
            .map(|value| clean_text(value, 120)),
        source_url,
        capture_mode: pick_capture_mode(source),
        capture_status: first_string(source, &["capture_status", "captureStatus"])
            .map(|value| clean_text(value, 120)),
        width_px: first_u32(source, &["width_px", "width"]),
        height_px: first_u32(source, &["height_px", "height"]),
    })
}

fn is_artifact_candidate(source: &Value) -> bool {
    let Some(obj) = source.as_object() else {
        return false;
    };
    let kind = obj
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    kind == "image"
        || kind == "screenshot"
        || obj.contains_key("image_url")
        || obj.contains_key("screenshot_url")
        || obj.contains_key("image_ref")
        || obj.contains_key("screenshot_ref")
        || obj.contains_key("artifact_ref")
        || (first_string(source, &["mime_type", "mimeType"]).is_some() && obj.contains_key("data"))
}

fn explicit_artifact_ref(source: &Value) -> Option<String> {
    first_string(
        source,
        &[
            "artifact_ref",
            "image_ref",
            "screenshot_ref",
            "image_url",
            "screenshot_url",
            "artifact_url",
            "uri",
            "file_path",
            "path",
        ],
    )
    .map(|value| value.to_string())
}

fn explicit_source_url(source: &Value) -> Option<String> {
    first_string(
        source,
        &[
            "source_url",
            "sourceUrl",
            "page_url",
            "pageUrl",
            "original_url",
            "originalUrl",
            "url",
        ],
    )
    .filter(|value| looks_like_url(value))
    .map(|value| value.to_string())
}

fn infer_artifact_kind(source: &Value) -> String {
    let raw_type = first_string(source, &["type"])
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if raw_type == "screenshot"
        || source.get("full_page").and_then(Value::as_bool) == Some(true)
        || source.get("fullPage").and_then(Value::as_bool) == Some(true)
        || source.get("capture_mode").is_some()
        || source.get("captureMode").is_some()
        || source.get("screenshot_url").is_some()
        || source.get("screenshot_ref").is_some()
    {
        return "screenshot".to_string();
    }
    "image".to_string()
}

fn pick_capture_mode(source: &Value) -> Option<String> {
    if let Some(mode) = first_string(source, &["capture_mode", "captureMode", "mode"]) {
        let cleaned = clean_text(mode, 120);
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    if source.get("full_page").and_then(Value::as_bool) == Some(true)
        || source.get("fullPage").and_then(Value::as_bool) == Some(true)
    {
        return Some("full_page".to_string());
    }
    None
}

fn first_u32(source: &Value, keys: &[&str]) -> Option<u32> {
    for key in keys {
        if let Some(value) = source.get(*key).and_then(Value::as_u64) {
            if let Ok(parsed) = u32::try_from(value) {
                return Some(parsed);
            }
        }
    }
    None
}

fn describe_artifact_ref(artifact: &EvidenceArtifactRef) -> String {
    let mut facets = Vec::<String>::new();
    if let Some(mime_type) = artifact.mime_type.as_deref() {
        if !mime_type.is_empty() {
            facets.push(clean_text(mime_type, 80));
        }
    }
    if let Some(capture_mode) = artifact.capture_mode.as_deref() {
        if !capture_mode.is_empty() {
            facets.push(format!("mode={}", clean_text(capture_mode, 80)));
        }
    }
    match (artifact.width_px, artifact.height_px) {
        (Some(width), Some(height)) => facets.push(format!("{width}x{height}px")),
        (Some(width), None) => facets.push(format!("width={width}px")),
        (None, Some(height)) => facets.push(format!("height={height}px")),
        (None, None) => {}
    }
    if let Some(status) = artifact.capture_status.as_deref() {
        if !status.is_empty() {
            facets.push(format!("status={}", clean_text(status, 80)));
        }
    }
    if facets.is_empty() {
        return format!("{} artifact", artifact.artifact_kind);
    }
    format!(
        "{} artifact ({})",
        artifact.artifact_kind,
        facets.join(", ")
    )
}

fn find_source_ref_candidate(source: &Value) -> Option<String> {
    if let Some(value) = first_string(
        source,
        &[
            "source_ref",
            "source_url",
            "sourceUrl",
            "page_url",
            "pageUrl",
            "original_url",
            "originalUrl",
            "repository_url",
            "repo_url",
            "url",
            "source",
            "repository",
            "file_path",
            "workspace_path",
            "repo_path",
            "repo",
            "file",
            "path",
        ],
    ) {
        return Some(value.to_string());
    }
    if let Some(text) = source.get("text").and_then(Value::as_str) {
        if looks_like_url(text) {
            return Some(text.to_string());
        }
    }
    if let Some(text) = source.as_str() {
        if looks_like_url(text) {
            return Some(text.to_string());
        }
    }
    if let Some(rows) = source.as_array() {
        for row in rows {
            if let Some(value) = find_source_ref_candidate(row) {
                return Some(value);
            }
        }
    }
    None
}

fn first_string<'a>(source: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = source.get(*key).and_then(Value::as_str) {
            return Some(value);
        }
    }
    None
}

fn clean_text(raw: &str, max_len: usize) -> String {
    sanitize_text_for_evidence(raw, max_len).text
}

fn looks_like_url(raw: &str) -> bool {
    let normalized = raw.trim();
    normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("file://")
        || normalized.starts_with("workspace://")
        || normalized.starts_with("raw://")
}

fn derive_source_scope(source_ref: &str) -> String {
    let normalized = source_ref.trim();
    if normalized.is_empty() {
        return "unspecified".to_string();
    }
    if let Some(nested) = archive_nested_url(normalized) {
        if let Some(host) = host_from_url(nested) {
            return host;
        }
    }
    if let Some(scope) = workspace_scope(normalized) {
        return scope;
    }
    if normalized.starts_with("file://") {
        return "file".to_string();
    }
    if normalized.starts_with("raw://") {
        return "raw".to_string();
    }
    if let Some(host) = host_from_url(normalized) {
        return host;
    }
    if let Some(segment) = normalized
        .split('/')
        .find(|segment| !segment.trim().is_empty())
        .map(|segment| clean_text(segment, 120))
        .filter(|segment| !segment.is_empty())
    {
        return format!("path:{segment}");
    }
    clean_text(normalized, 120)
}

fn archive_nested_url(raw: &str) -> Option<&str> {
    let marker = "/web/";
    let index = raw.find(marker)?;
    let tail = raw.get(index + marker.len()..)?;
    let nested_start = tail.find("http://").or_else(|| tail.find("https://"))?;
    tail.get(nested_start..)
}

fn host_from_url(raw: &str) -> Option<String> {
    let (_, remainder) = raw.split_once("://")?;
    let host_port = remainder
        .split('/')
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default();
    let host = host_port
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('.');
    if host.is_empty() {
        return None;
    }
    Some(
        host.strip_prefix("www.")
            .unwrap_or(host)
            .to_ascii_lowercase(),
    )
}

fn workspace_scope(raw: &str) -> Option<String> {
    let normalized = raw.trim();
    let remainder = normalized.strip_prefix("workspace://")?;
    let scope = remainder
        .split('/')
        .find(|segment| !segment.trim().is_empty())
        .unwrap_or_default()
        .trim();
    if scope.is_empty() {
        return Some("workspace".to_string());
    }
    Some(format!("workspace:{scope}"))
}
