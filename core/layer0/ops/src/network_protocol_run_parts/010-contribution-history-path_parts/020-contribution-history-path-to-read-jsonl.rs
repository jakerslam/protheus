// FILE_SIZE_EXCEPTION: reason=Atomic network protocol runtime block requires semantic extraction to preserve protocol guarantees; owner=jay; expires=2026-04-12

fn contribution_history_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("contribution_history.jsonl")
}

fn consensus_ledger_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("consensus_ledger.jsonl")
}

fn membership_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("membership.json")
}

fn governance_votes_path(root: &Path) -> std::path::PathBuf {
    state_root(root).join("governance_votes.jsonl")
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}
