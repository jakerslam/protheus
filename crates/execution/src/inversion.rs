use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeImpactInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeImpactOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeModeInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeModeOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTargetInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTargetOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeResultInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeResultOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ObjectiveIdValidInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObjectiveIdValidOutput {
    pub valid: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TritVectorFromInputInput {
    #[serde(default)]
    pub trit_vector: Option<Vec<Value>>,
    #[serde(default)]
    pub trit_vector_csv: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TritVectorFromInputOutput {
    pub vector: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct JaccardSimilarityInput {
    #[serde(default)]
    pub left_tokens: Vec<String>,
    #[serde(default)]
    pub right_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JaccardSimilarityOutput {
    pub similarity: f64,
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let collapsed = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase();
    collapsed.chars().take(max_len).collect::<String>()
}

pub fn compute_normalize_impact(input: &NormalizeImpactInput) -> NormalizeImpactOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("medium"), 24);
    let value = match raw.as_str() {
        "low" | "medium" | "high" | "critical" => raw,
        _ => "medium".to_string(),
    };
    NormalizeImpactOutput { value }
}

pub fn compute_normalize_mode(input: &NormalizeModeInput) -> NormalizeModeOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("live"), 16);
    let value = if raw == "test" {
        "test".to_string()
    } else {
        "live".to_string()
    };
    NormalizeModeOutput { value }
}

pub fn compute_normalize_target(input: &NormalizeTargetInput) -> NormalizeTargetOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("tactical"), 24);
    let value = match raw.as_str() {
        "tactical" | "belief" | "identity" | "directive" | "constitution" => raw,
        _ => "tactical".to_string(),
    };
    NormalizeTargetOutput { value }
}

pub fn compute_normalize_result(input: &NormalizeResultInput) -> NormalizeResultOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or(""), 24);
    let value = match raw.as_str() {
        "success" | "neutral" | "fail" | "destructive" => raw,
        _ => String::new(),
    };
    NormalizeResultOutput { value }
}

fn is_valid_objective_id(raw: &str) -> bool {
    if raw.len() < 6 || raw.len() > 140 {
        return false;
    }
    let bytes = raw.as_bytes();
    let first = bytes[0] as char;
    let last = bytes[bytes.len() - 1] as char;
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return false;
    }
    if bytes.len() < 2 {
        return false;
    }
    for ch in &bytes[1..(bytes.len() - 1)] {
        let c = *ch as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == ':' || c == '-' {
            continue;
        }
        return false;
    }
    true
}

pub fn compute_objective_id_valid(input: &ObjectiveIdValidInput) -> ObjectiveIdValidOutput {
    let raw = input.value.as_deref().unwrap_or("").trim();
    ObjectiveIdValidOutput {
        valid: is_valid_objective_id(raw),
    }
}

fn normalize_trit_value(value: &Value) -> i32 {
    if let Some(n) = value.as_f64() {
        if n > 0.0 {
            return 1;
        }
        if n < 0.0 {
            return -1;
        }
        return 0;
    }
    if let Some(b) = value.as_bool() {
        return if b { 1 } else { 0 };
    }
    if let Some(text) = value.as_str() {
        let parsed = text.trim().parse::<f64>().ok().unwrap_or(0.0);
        if parsed > 0.0 {
            return 1;
        }
        if parsed < 0.0 {
            return -1;
        }
    }
    0
}

pub fn compute_trit_vector_from_input(input: &TritVectorFromInputInput) -> TritVectorFromInputOutput {
    if let Some(vec) = &input.trit_vector {
        let out = vec.iter().map(normalize_trit_value).collect::<Vec<_>>();
        return TritVectorFromInputOutput { vector: out };
    }
    let raw = input.trit_vector_csv.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return TritVectorFromInputOutput { vector: Vec::new() };
    }
    let out = raw
        .split(',')
        .map(|token| {
            let parsed = token.trim().parse::<f64>().ok().unwrap_or(0.0);
            if parsed > 0.0 {
                1
            } else if parsed < 0.0 {
                -1
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    TritVectorFromInputOutput { vector: out }
}

pub fn compute_jaccard_similarity(input: &JaccardSimilarityInput) -> JaccardSimilarityOutput {
    let left = input
        .left_tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect::<BTreeSet<_>>();
    let right = input
        .right_tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect::<BTreeSet<_>>();
    if left.is_empty() && right.is_empty() {
        return JaccardSimilarityOutput { similarity: 1.0 };
    }
    if left.is_empty() || right.is_empty() {
        return JaccardSimilarityOutput { similarity: 0.0 };
    }
    let inter = left.intersection(&right).count() as f64;
    let union = left.union(&right).count() as f64;
    let similarity = if union > 0.0 { inter / union } else { 0.0 };
    JaccardSimilarityOutput { similarity }
}

fn decode_input<T>(payload: &Value, key: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Default,
{
    let value = payload
        .get(key)
        .cloned()
        .unwrap_or_else(|| json!({}));
    serde_json::from_value(value).map_err(|e| format!("inversion_decode_{key}_failed:{e}"))
}

pub fn run_inversion_json(payload_json: &str) -> Result<String, String> {
    let payload: Value =
        serde_json::from_str(payload_json).map_err(|e| format!("inversion_payload_parse_failed:{e}"))?;
    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    if mode.is_empty() {
        return Err("inversion_mode_missing".to_string());
    }
    if mode == "normalize_impact" {
        let input: NormalizeImpactInput = decode_input(&payload, "normalize_impact_input")?;
        let out = compute_normalize_impact(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_impact",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_impact_failed:{e}"));
    }
    if mode == "normalize_mode" {
        let input: NormalizeModeInput = decode_input(&payload, "normalize_mode_input")?;
        let out = compute_normalize_mode(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_mode",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_mode_failed:{e}"));
    }
    if mode == "normalize_target" {
        let input: NormalizeTargetInput = decode_input(&payload, "normalize_target_input")?;
        let out = compute_normalize_target(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_target",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_target_failed:{e}"));
    }
    if mode == "normalize_result" {
        let input: NormalizeResultInput = decode_input(&payload, "normalize_result_input")?;
        let out = compute_normalize_result(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "normalize_result",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_normalize_result_failed:{e}"));
    }
    if mode == "objective_id_valid" {
        let input: ObjectiveIdValidInput = decode_input(&payload, "objective_id_valid_input")?;
        let out = compute_objective_id_valid(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "objective_id_valid",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_objective_id_valid_failed:{e}"));
    }
    if mode == "trit_vector_from_input" {
        let input: TritVectorFromInputInput = decode_input(&payload, "trit_vector_from_input_input")?;
        let out = compute_trit_vector_from_input(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "trit_vector_from_input",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_trit_vector_from_input_failed:{e}"));
    }
    if mode == "jaccard_similarity" {
        let input: JaccardSimilarityInput = decode_input(&payload, "jaccard_similarity_input")?;
        let out = compute_jaccard_similarity(&input);
        return serde_json::to_string(&json!({
            "ok": true,
            "mode": "jaccard_similarity",
            "payload": out
        }))
        .map_err(|e| format!("inversion_encode_jaccard_similarity_failed:{e}"));
    }
    Err(format!("inversion_mode_unsupported:{mode}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_impact_matches_expected_set() {
        assert_eq!(
            compute_normalize_impact(&NormalizeImpactInput {
                value: Some("CRITICAL".to_string())
            }),
            NormalizeImpactOutput {
                value: "critical".to_string()
            }
        );
        assert_eq!(
            compute_normalize_impact(&NormalizeImpactInput {
                value: Some("unknown".to_string())
            }),
            NormalizeImpactOutput {
                value: "medium".to_string()
            }
        );
    }

    #[test]
    fn normalize_mode_defaults_live() {
        assert_eq!(
            compute_normalize_mode(&NormalizeModeInput {
                value: Some("test".to_string())
            }),
            NormalizeModeOutput {
                value: "test".to_string()
            }
        );
        assert_eq!(
            compute_normalize_mode(&NormalizeModeInput {
                value: Some("prod".to_string())
            }),
            NormalizeModeOutput {
                value: "live".to_string()
            }
        );
    }

    #[test]
    fn normalize_target_enforces_known_targets() {
        assert_eq!(
            compute_normalize_target(&NormalizeTargetInput {
                value: Some("directive".to_string())
            }),
            NormalizeTargetOutput {
                value: "directive".to_string()
            }
        );
        assert_eq!(
            compute_normalize_target(&NormalizeTargetInput {
                value: Some("unknown".to_string())
            }),
            NormalizeTargetOutput {
                value: "tactical".to_string()
            }
        );
    }

    #[test]
    fn normalize_result_enforces_expected_results() {
        assert_eq!(
            compute_normalize_result(&NormalizeResultInput {
                value: Some("SUCCESS".to_string())
            }),
            NormalizeResultOutput {
                value: "success".to_string()
            }
        );
        assert_eq!(
            compute_normalize_result(&NormalizeResultInput {
                value: Some("maybe".to_string())
            }),
            NormalizeResultOutput {
                value: String::new()
            }
        );
    }

    #[test]
    fn inversion_json_mode_routes() {
        let payload = json!({
            "mode": "normalize_target",
            "normalize_target_input": { "value": "belief" }
        });
        let out = run_inversion_json(&payload.to_string()).expect("inversion normalize_target");
        assert!(out.contains("\"mode\":\"normalize_target\""));
        assert!(out.contains("\"value\":\"belief\""));
    }

    #[test]
    fn objective_id_validation_matches_expected_pattern() {
        let valid = compute_objective_id_valid(&ObjectiveIdValidInput {
            value: Some("T1_objective-alpha".to_string()),
        });
        assert!(valid.valid);
        let invalid = compute_objective_id_valid(&ObjectiveIdValidInput {
            value: Some("bad".to_string()),
        });
        assert!(!invalid.valid);
    }

    #[test]
    fn trit_vector_from_input_normalizes_numeric_tokens() {
        let out = compute_trit_vector_from_input(&TritVectorFromInputInput {
            trit_vector: Some(vec![json!(-2), json!(0), json!(3)]),
            trit_vector_csv: None,
        });
        assert_eq!(out.vector, vec![-1, 0, 1]);
    }

    #[test]
    fn jaccard_similarity_matches_overlap_ratio() {
        let out = compute_jaccard_similarity(&JaccardSimilarityInput {
            left_tokens: vec!["a".to_string(), "b".to_string()],
            right_tokens: vec!["b".to_string(), "c".to_string()],
        });
        assert!((out.similarity - (1.0 / 3.0)).abs() < 1e-9);
    }
}
