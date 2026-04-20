fn detect_parent_lineage_loop(
    state: &SwarmState,
    start_parent_id: Option<&str>,
    max_hops: usize,
) -> Option<Value> {
    let mut current = start_parent_id.map(ToString::to_string);
    let mut visited = BTreeSet::new();
    let mut lineage = Vec::new();
    let mut hops = 0usize;

    while let Some(session_id) = current {
        hops = hops.saturating_add(1);
        if hops > max_hops.max(1) {
            return Some(json!({
                "detected": true,
                "reason": "cost_guard_exceeded",
                "max_hops": max_hops.max(1),
                "hops": hops,
                "lineage": lineage,
            }));
        }
        if !visited.insert(session_id.clone()) {
            lineage.push(session_id.clone());
            return Some(json!({
                "detected": true,
                "reason": "cycle_detected",
                "cycle_at": session_id,
                "hops": hops,
                "lineage": lineage,
            }));
        }
        lineage.push(session_id.clone());
        current = state
            .sessions
            .get(&session_id)
            .and_then(|session| session.parent_id.clone());
    }

    None
}
