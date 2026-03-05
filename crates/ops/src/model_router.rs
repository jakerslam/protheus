use crate::legacy_bridge::{resolve_script_path, run_legacy_script};
use std::collections::HashSet;
use std::path::Path;

const LEGACY_SCRIPT_ENV: &str = "PROTHEUS_MODEL_ROUTER_LEGACY_SCRIPT";
const LEGACY_SCRIPT_DEFAULT: &str = "systems/routing/model_router_legacy.js";

pub fn run(root: &Path, args: &[String]) -> i32 {
    let script = resolve_script_path(root, LEGACY_SCRIPT_ENV, LEGACY_SCRIPT_DEFAULT);
    run_legacy_script(root, "model_router", &script, args, false)
}

pub fn is_local_ollama_model(model_id: &str) -> bool {
    let model = model_id.trim();
    !model.is_empty() && model.starts_with("ollama/") && !model.contains(":cloud")
}

pub fn infer_tier(risk: &str, complexity: &str) -> u8 {
    let risk_norm = risk.trim().to_ascii_lowercase();
    let complexity_norm = complexity.trim().to_ascii_lowercase();
    if risk_norm == "high" || complexity_norm == "high" {
        return 3;
    }
    if risk_norm == "medium" || complexity_norm == "medium" {
        return 2;
    }
    1
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

fn has_any_exact(tokens: &HashSet<String>, words: &[&str]) -> bool {
    words.iter().any(|w| tokens.contains(&w.to_ascii_lowercase()))
}

fn has_prefix(tokens: &HashSet<String>, prefix: &str) -> bool {
    let p = prefix.to_ascii_lowercase();
    tokens.iter().any(|t| t.starts_with(&p))
}

pub fn infer_role(intent: &str, task: &str) -> String {
    let combined = format!("{} {}", intent, task);
    let tokens = tokenize(&combined);

    if has_any_exact(
        &tokens,
        &[
            "code",
            "refactor",
            "patch",
            "bug",
            "test",
            "typescript",
            "javascript",
            "python",
            "node",
            "compile",
        ],
    ) {
        return "coding".to_string();
    }

    if has_any_exact(
        &tokens,
        &[
            "tool",
            "api",
            "curl",
            "exec",
            "command",
            "shell",
            "cli",
            "automation",
        ],
    ) || has_prefix(&tokens, "integrat")
    {
        return "tools".to_string();
    }

    let has_parallel_agent = tokens.contains("parallel") && tokens.contains("agent");
    if has_any_exact(&tokens, &["swarm", "multi-agent", "handoff", "delegate"])
        || has_parallel_agent
    {
        return "swarm".to_string();
    }

    if has_any_exact(&tokens, &["plan", "roadmap", "strategy", "backlog", "roi"]) || has_prefix(&tokens, "priorit") {
        return "planning".to_string();
    }

    if has_any_exact(&tokens, &["prove", "formal", "derive", "reason", "logic", "constraint"]) {
        return "logic".to_string();
    }

    if has_any_exact(&tokens, &["chat", "reply", "post", "comment", "write", "summar", "explain"])
    {
        return "chat".to_string();
    }

    "general".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ollama_model_detection_is_strict() {
        assert!(is_local_ollama_model("ollama/llama3"));
        assert!(!is_local_ollama_model("ollama/llama3:cloud"));
        assert!(!is_local_ollama_model("openai/gpt-4.1"));
    }

    #[test]
    fn tier_inference_matches_risk_complexity_contract() {
        assert_eq!(infer_tier("high", "low"), 3);
        assert_eq!(infer_tier("low", "high"), 3);
        assert_eq!(infer_tier("medium", "low"), 2);
        assert_eq!(infer_tier("low", "medium"), 2);
        assert_eq!(infer_tier("low", "low"), 1);
    }

    #[test]
    fn role_inference_preserves_persona_lens_priority() {
        assert_eq!(infer_role("fix compile issue", "patch node script"), "coding");
        assert_eq!(infer_role("integrate with api", "cli automation"), "tools");
        assert_eq!(infer_role("plan next sprint", "roadmap prioritization"), "planning");
        assert_eq!(infer_role("derive proof", "logic constraints"), "logic");
        assert_eq!(infer_role("write summary", "explain status"), "chat");
        assert_eq!(infer_role("random", "unclassified"), "general");
    }
}
