// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::quarantine::SnapshotMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisCoreScore {
    pub language_fit: u32,
    pub similarity: u32,
    pub churn_risk: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TasteSignal {
    pub signal: String,
    pub score: u32,
    pub rationale: String,
    pub advisory_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisReport {
    pub snapshot_id: String,
    pub deterministic_score: AnalysisCoreScore,
    pub cravings: Vec<TasteSignal>,
    pub ranked_capabilities: Vec<String>,
}

fn score_language_fit(rs_count: usize, ts_count: usize, shell_count: usize) -> u32 {
    let dominant = rs_count.max(ts_count).max(shell_count) as f64;
    let total = (rs_count + ts_count + shell_count).max(1) as f64;
    ((dominant / total) * 100.0).round() as u32
}

fn score_similarity(tokens: &[String]) -> u32 {
    fn score_token(token: &str) -> u32 {
        let mut score = 0u32;
        if token.contains("policy") || token.contains("receipt") {
            score += 10;
        }
        if token.contains("kernel") || token.contains("runtime") {
            score += 8;
        }
        if token.contains("security") {
            score += 12;
        }
        if token.contains("adapter") {
            score += 6;
        }
        score
    }

    let mut score = 0u32;
    for token in tokens {
        score += score_token(token);
    }
    score.min(100)
}

fn score_churn_risk(file_count: usize) -> u32 {
    if file_count <= 20 {
        8
    } else if file_count <= 120 {
        24
    } else if file_count <= 400 {
        52
    } else {
        72
    }
}

pub fn deterministic_analyze(
    snapshot: &SnapshotMetadata,
    snapshot_root: &Path,
    reviewer_feedback: Option<&[(String, bool)]>,
) -> Result<AnalysisReport, String> {
    let mut rs_count = 0usize;
    let mut ts_count = 0usize;
    let mut shell_count = 0usize;
    let mut tokens = Vec::<String>::new();

    for entry in WalkDir::new(snapshot_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|row| row.to_str()).unwrap_or("");
        match ext {
            "rs" => rs_count += 1,
            "ts" | "tsx" => ts_count += 1,
            "sh" => shell_count += 1,
            _ => {}
        }
        if let Some(stem) = path.file_stem().and_then(|row| row.to_str()) {
            tokens.push(stem.to_ascii_lowercase());
        }
    }

    tokens.sort();
    let language_fit = score_language_fit(rs_count, ts_count, shell_count);
    let similarity = score_similarity(&tokens);
    let churn_risk = score_churn_risk(snapshot.file_count);
    let total = (language_fit * 4 + similarity * 4 + (100 - churn_risk) * 2) / 10;

    let feedback_gain = reviewer_feedback
        .map(|rows| {
            if rows.is_empty() {
                0u32
            } else {
                let accepted = rows.iter().filter(|(_, accepted)| *accepted).count() as f64;
                ((accepted / rows.len() as f64) * 100.0).round() as u32
            }
        })
        .unwrap_or(50);

    let mut cravings = vec![
        TasteSignal {
            signal: "ast_similarity".to_string(),
            score: similarity,
            rationale: "token overlap with known runtime kernel patterns".to_string(),
            advisory_only: true,
        },
        TasteSignal {
            signal: "language_fit".to_string(),
            score: language_fit,
            rationale: "dominant language fit for core authority lanes".to_string(),
            advisory_only: true,
        },
        TasteSignal {
            signal: "reviewer_feedback".to_string(),
            score: feedback_gain,
            rationale: "rolling acceptance trend from reviewed proposals".to_string(),
            advisory_only: true,
        },
    ];
    cravings.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.signal.cmp(&b.signal)));

    let mut ranked_capabilities = vec![
        "namespace_fix".to_string(),
        "header_injection".to_string(),
        "path_remap".to_string(),
        "adapter_scaffold".to_string(),
    ];
    ranked_capabilities.sort();

    Ok(AnalysisReport {
        snapshot_id: snapshot.snapshot_id.clone(),
        deterministic_score: AnalysisCoreScore {
            language_fit,
            similarity,
            churn_risk,
            total,
        },
        cravings,
        ranked_capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::SnapshotMetadata;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn deterministic_analyze_returns_stable_scores() {
        let root = tempdir().expect("tmp");
        fs::write(root.path().join("policy_kernel.rs"), "fn main(){}").expect("write rs");
        fs::write(root.path().join("adapter.ts"), "export {};").expect("write ts");
        let snapshot = SnapshotMetadata {
            snapshot_id: "x".to_string(),
            origin_url: "https://github.com/acme/repo".to_string(),
            quarantine_root: root.path().display().to_string(),
            tree_hash: "hash".to_string(),
            file_count: 2,
            symlink_count: 0,
            captured_at: "0".to_string(),
        };
        let out = deterministic_analyze(&snapshot, root.path(), None).expect("analyze");
        assert!(out.deterministic_score.total > 0);
        assert_eq!(out.ranked_capabilities.len(), 4);
    }
}
