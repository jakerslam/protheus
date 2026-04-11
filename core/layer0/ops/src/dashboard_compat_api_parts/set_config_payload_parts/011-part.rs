fn agent_parent_map(root: &Path, snapshot: &Value) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for row in build_agent_roster(root, snapshot, true) {
        let id = clean_agent_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        let parent = parent_agent_id_from_row(&row);
        if !parent.is_empty() {
            out.insert(id, parent);
        }
    }
    out
}

fn actor_has_lineage_path(parent_map: &HashMap<String, String>, child_id: &str, ancestor_id: &str) -> bool {
    let mut current = clean_agent_id(child_id);
    let target = clean_agent_id(ancestor_id);
    if current.is_empty() || target.is_empty() {
        return false;
    }
    let mut hops = 0usize;
    let mut seen = HashSet::<String>::new();
    while hops < 64 && seen.insert(current.clone()) {
        let Some(parent) = parent_map.get(&current).cloned() else {
            return false;
        };
        if parent == target {
            return true;
        }
        current = parent;
        hops += 1;
    }
    false
}

fn actor_can_manage_target(root: &Path, snapshot: &Value, actor_id: &str, target_id: &str) -> bool {
    let actor = clean_agent_id(actor_id);
    let target = clean_agent_id(target_id);
    if actor.is_empty() || target.is_empty() {
        return actor.is_empty();
    }
    actor == target || actor_has_lineage_path(&agent_parent_map(root, snapshot), &target, &actor)
}

fn actor_can_message_target(root: &Path, snapshot: &Value, actor_id: &str, target_id: &str) -> bool {
    let actor = clean_agent_id(actor_id);
    let target = clean_agent_id(target_id);
    if actor.is_empty() || target.is_empty() {
        return actor.is_empty();
    }
    if actor == target {
        return true;
    }
    let parent_map = agent_parent_map(root, snapshot);
    let actor_parent = parent_map.get(&actor).cloned().unwrap_or_default();
    let target_parent = parent_map.get(&target).cloned().unwrap_or_default();
    actor_parent == target_parent && !actor_parent.is_empty()
        || actor_has_lineage_path(&parent_map, &target, &actor)
        || actor_has_lineage_path(&parent_map, &actor, &target)
}

fn parent_can_archive_descendant_without_signoff(
    root: &Path,
    snapshot: &Value,
    actor_id: &str,
    normalized_tool: &str,
    input: &Value,
) -> bool {
    if !matches!(normalized_tool, "agent_action" | "manage_agent") {
        return false;
    }
    let action = clean_text(
        input.get("action").and_then(Value::as_str).unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    if !matches!(action.as_str(), "archive" | "delete") {
        return false;
    }
    let actor = clean_agent_id(actor_id);
    let target = clean_agent_id(input.get("agent_id").and_then(Value::as_str).unwrap_or(""));
    if actor.is_empty() || target.is_empty() || actor == target {
        return false;
    }
    actor_can_manage_target(root, snapshot, &actor, &target)
}
