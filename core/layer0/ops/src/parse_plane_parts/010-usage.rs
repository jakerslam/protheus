// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::parse_plane (authoritative)

use crate::v8_kernel::{
    attach_conduit, build_plane_conduit_enforcement, canonical_json_string, canonicalize_json,
    conduit_bypass_requested, emit_plane_receipt, load_json_or, parse_bool, parse_u64,
    plane_status, print_json, read_json, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "PARSE_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "parse_plane";

const PARSE_CONTRACT_PATH: &str = "planes/contracts/parse/mapping_rule_parser_contract_v1.json";
const VISUALIZE_CONTRACT_PATH: &str =
    "planes/contracts/parse/parse_instruction_pipeline_contract_v1.json";
const TABLE_POSTPROCESS_CONTRACT_PATH: &str =
    "planes/contracts/parse/table_postprocessing_contract_v1.json";
const FLATTEN_TRANSFORM_CONTRACT_PATH: &str =
    "planes/contracts/parse/flatten_unnest_transform_contract_v1.json";
const TEMPLATE_GOVERNANCE_CONTRACT_PATH: &str =
    "planes/contracts/parse/parser_template_governance_contract_v1.json";
const TEMPLATE_MANIFEST_PATH: &str = "planes/contracts/parse/parser_template_pack_manifest_v1.json";
const DEFAULT_MAPPING_ROOT: &str = "planes/contracts/parse/mappings";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops parse-plane status");
    println!("  protheus-ops parse-plane parse-doc [--file=<path>|--source=<text>] [--mapping=<id>|--mapping-path=<path>] [--strict=1|0]");
    println!("  protheus-ops parse-plane visualize [--from-path=<path>] [--strict=1|0]");
    println!("  protheus-ops parse-plane postprocess-table [--table-json=<json>|--table-path=<path>|--from-path=<path>] [--max-rows=<n>] [--max-cols=<n>] [--strict=1|0]");
    println!("  protheus-ops parse-plane flatten [--json=<json>|--json-path=<path>|--from-path=<path>] [--max-depth=<n>] [--format=dot|slash] [--strict=1|0]");
    println!("  protheus-ops parse-plane export [--from-path=<path>] [--output-path=<path>] [--format=json|jsonl|md] [--strict=1|0]");
    println!("  protheus-ops parse-plane template-governance [--manifest=<path>] [--templates-root=<path>] [--strict=1|0]");
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "parse_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "parse_plane_status")
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "parse_conduit_enforcement",
        "core/layer0/ops/parse_plane",
        bypass_requested,
        "all_parse_apply_and_visualize_actions_route_through_conduit_with_bypass_rejection",
        &["V6-PARSE-001.6"],
    )
}

fn strip_tags(raw: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in raw.chars() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '>' {
            in_tag = false;
            out.push(' ');
            continue;
        }
        if !in_tag {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_title(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if let Some(start) = lower.find("<title>") {
        let body = &raw[start + 7..];
        if let Some(end) = body.to_ascii_lowercase().find("</title>") {
            return clean(&body[..end], 240);
        }
    }
    if let Some(first) = raw.lines().next() {
        return clean(first, 240);
    }
    "untitled".to_string()
}

fn extract_between(raw: &str, start: &str, end: &str) -> Option<String> {
    let from = raw.find(start)?;
    let rest = &raw[from + start.len()..];
    let until = rest.find(end)?;
    Some(clean(&rest[..until], 500))
}

fn extract_prefix_line(raw: &str, prefix: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix(prefix) {
            return Some(clean(stripped.trim(), 500));
        }
    }
    None
}

fn load_source(root: &Path, parsed: &crate::ParsedArgs) -> Result<(String, String), String> {
    if let Some(inline) = parsed.flags.get("source") {
        let source = clean(inline, 200_000);
        if source.is_empty() {
            return Err("source_empty".to_string());
        }
        return Ok(("inline".to_string(), source));
    }
    if let Some(file_rel) = parsed
        .flags
        .get("file")
        .or_else(|| parsed.positional.get(1))
    {
        let path = if Path::new(file_rel).is_absolute() {
            PathBuf::from(file_rel)
        } else {
            root.join(file_rel)
        };
        let source = fs::read_to_string(&path)
            .map_err(|_| format!("source_file_not_found:{}", path.display()))?;
        if source.trim().is_empty() {
            return Err("source_empty".to_string());
        }
        return Ok((path.display().to_string(), source));
    }
    Err("missing_source".to_string())
}

fn load_mapping(root: &Path, parsed: &crate::ParsedArgs) -> Result<(String, Value), String> {
    if let Some(path_raw) = parsed.flags.get("mapping-path") {
        let path = if Path::new(path_raw).is_absolute() {
            PathBuf::from(path_raw)
        } else {
            root.join(path_raw)
        };
        let value =
            read_json(&path).ok_or_else(|| format!("mapping_not_found:{}", path.display()))?;
        return Ok((path.display().to_string(), value));
    }

    let mapping_id = clean(
        parsed
            .flags
            .get("mapping")
            .cloned()
            .unwrap_or_else(|| "default".to_string()),
        120,
    );
    let path = root
        .join(DEFAULT_MAPPING_ROOT)
        .join(format!("{mapping_id}.json"));
    let value = read_json(&path).ok_or_else(|| format!("mapping_not_found:{}", path.display()))?;
    Ok((path.display().to_string(), value))
}

fn apply_rule(rule: &Value, source_raw: &str, source_plain: &str) -> (String, Value, bool) {
    let field = rule
        .get("field")
        .and_then(Value::as_str)
        .map(|v| clean(v, 120))
        .unwrap_or_else(|| "field".to_string());
    let strategy = rule
        .get("strategy")
        .and_then(Value::as_str)
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .unwrap_or_else(|| "contains".to_string());
    let required = rule
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let value = match strategy.as_str() {
        "title" => Value::String(parse_title(source_raw)),
        "between" => {
            let start = rule
                .get("start")
                .and_then(Value::as_str)
                .map(|v| clean(v, 120))
                .unwrap_or_default();
            let end = rule
                .get("end")
                .and_then(Value::as_str)
                .map(|v| clean(v, 120))
                .unwrap_or_default();
            if start.is_empty() || end.is_empty() {
                Value::Null
            } else {
                extract_between(source_raw, &start, &end)
                    .map(Value::String)
                    .unwrap_or(Value::Null)
            }
        }
        "prefix_line" => {
            let prefix = rule
                .get("prefix")
                .and_then(Value::as_str)
                .map(|v| clean(v, 120))
                .unwrap_or_default();
            if prefix.is_empty() {
                Value::Null
            } else {
                extract_prefix_line(source_raw, &prefix)
                    .map(Value::String)
                    .unwrap_or(Value::Null)
            }
        }
        "constant" => rule.get("value").cloned().unwrap_or(Value::Null),
        "contains" => {
            let token = rule
                .get("token")
                .and_then(Value::as_str)
                .map(|v| clean(v, 120))
                .unwrap_or_default();
            if token.is_empty() {
                Value::Null
            } else {
                Value::Bool(
                    source_plain
                        .to_ascii_lowercase()
                        .contains(&token.to_ascii_lowercase()),
                )
            }
        }
        _ => Value::Null,
    };

    let present = if value.is_null() {
        false
    } else if let Some(s) = value.as_str() {
        !s.is_empty()
    } else {
        true
    };
    let valid = if required { present } else { true };
    (field, value, valid)
}

fn value_to_table(value: &Value) -> Option<Vec<Vec<String>>> {
    if let Some(rows) = value.as_array() {
        if rows.is_empty() {
            return Some(Vec::new());
        }
        if rows.iter().all(|row| row.is_array()) {
            let mut out = Vec::<Vec<String>>::new();
            for row in rows {
                let mut rendered = Vec::<String>::new();
                for cell in row.as_array().cloned().unwrap_or_default() {
                    rendered.push(clean(cell.as_str().unwrap_or(&cell.to_string()), 800));
                }
                out.push(rendered);
            }
            return Some(out);
        }
        if rows.iter().all(|row| row.is_object()) {
            let mut keys = rows
                .iter()
                .filter_map(Value::as_object)
                .flat_map(|obj| obj.keys().cloned().collect::<Vec<_>>())
                .collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            let mut out = vec![keys.clone()];
            for row in rows {
                let mut rendered = Vec::<String>::new();
                if let Some(obj) = row.as_object() {
                    for key in &keys {
                        let v = obj.get(key).cloned().unwrap_or(Value::Null);
                        rendered.push(clean(v.as_str().unwrap_or(&v.to_string()), 800));
                    }
                }
                out.push(rendered);
            }
            return Some(out);
        }
    }
    if let Some(raw) = value.as_str() {
        let mut out = Vec::<Vec<String>>::new();
        for line in raw.lines() {
            if !line.contains('|') {
                continue;
            }
            let row = line
                .split('|')
                .map(|cell| clean(cell.trim(), 800))
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            if !row.is_empty() {
                out.push(row);
            }
        }
        return Some(out);
    }
    None
}

fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '-' | '=' | ':' | '|' | ' '))
}

fn is_fake_row(row: &[String]) -> bool {
    if row.is_empty() {
        return true;
    }
    row.iter()
        .all(|cell| cell.trim().is_empty() || is_separator_cell(cell))
}

fn strip_footnote(cell: &str) -> (String, Option<String>) {
    let trimmed = cell.trim();
    if !trimmed.ends_with(']') {
        return (clean(trimmed, 800), None);
    }
    let Some(open_idx) = trimmed.rfind('[') else {
        return (clean(trimmed, 800), None);
    };
    if open_idx == 0 {
        return (clean(trimmed, 800), None);
    }
    let marker = &trimmed[open_idx + 1..trimmed.len() - 1];
    if marker.is_empty() || !marker.chars().all(|ch| ch.is_ascii_digit()) {
        return (clean(trimmed, 800), None);
    }
    let base = clean(trimmed[..open_idx].trim_end(), 800);
    let note = clean(&trimmed[open_idx..], 64);
    (base, Some(note))
}

