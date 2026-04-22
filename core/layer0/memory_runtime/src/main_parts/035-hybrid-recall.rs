use protheus_memory_core_v1::{
    self, CapabilityAction, CapabilityToken, Classification, DefaultVerityMemoryPolicy, MemoryKind,
    MemoryObject, MemoryRecallHit, MemoryRecallQuery, MemoryScope, OwnerScopeSettings, TrustState,
    UnifiedMemoryHeap, UnifiedMemoryHeapConfig,
};

fn runtime_memory_route() -> memory_core_v1::NexusRouteContext {
    memory_core_v1::NexusRouteContext {
        issuer: "memory_runtime".to_string(),
        source: "memory_runtime".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.runtime.hybrid_recall".to_string(),
        lease_id: "memory_runtime_hybrid_recall".to_string(),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(60_000),
    }
}

fn runtime_memory_capability() -> CapabilityToken {
    CapabilityToken {
        token_id: "memory_runtime_capability".to_string(),
        principal_id: "core:memory_runtime".to_string(),
        scopes: vec![MemoryScope::Core],
        allowed_actions: vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::Canonicalize,
            CapabilityAction::MaterializeContext,
        ],
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: "memory_runtime_receipt".to_string(),
    }
}

fn title_case(raw: &str) -> String {
    raw.split('_')
        .map(|chunk| {
            let mut chars = chunk.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn normalize_entity_tag(tag: &str) -> String {
    let trimmed = tag.trim().trim_start_matches('#').to_ascii_lowercase();
    trimmed
        .chars()
        .filter(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, ':' | '_' | '-' | '='))
        .collect::<String>()
}

fn parse_entity_refs_from_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let normalized = normalize_entity_tag(tag);
        if matches!(
            normalized.split(':').next(),
            Some("person" | "project" | "system" | "incident" | "session")
        ) {
            out.push(normalized);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn payload_from_entry(entry: &IndexEntry) -> Value {
    let mut payload = json!({
        "summary": entry.summary.clone(),
        "file": entry.file_rel.clone(),
        "uid": entry.uid.clone(),
        "node_id": entry.node_id.clone(),
        "tags": entry.tags.clone(),
    });
    let entity_refs = parse_entity_refs_from_tags(&entry.tags);
    if let Some(map) = payload.as_object_mut() {
        for entity_ref in &entity_refs {
            let parts = entity_ref.splitn(2, ':').collect::<Vec<&str>>();
            if parts.len() != 2 {
                continue;
            }
            match parts[0] {
                "person" => {
                    map.entry("person".to_string())
                        .or_insert_with(|| Value::String(title_case(parts[1])));
                }
                "project" => {
                    map.entry("project".to_string())
                        .or_insert_with(|| Value::String(title_case(parts[1])));
                }
                "system" => {
                    map.entry("system".to_string())
                        .or_insert_with(|| Value::String(title_case(parts[1])));
                }
                "incident" => {
                    map.entry("incident".to_string())
                        .or_insert_with(|| Value::String(title_case(parts[1])));
                }
                _ => {}
            }
        }
    }
    payload
}

fn infer_kind(tags: &[String]) -> MemoryKind {
    if tags.iter().any(|tag| normalize_tag(tag).starts_with("procedure")) {
        return MemoryKind::Procedural;
    }
    if tags
        .iter()
        .any(|tag| matches!(normalize_tag(tag).as_str(), "semantic" | "fact" | "knowledge"))
    {
        return MemoryKind::Semantic;
    }
    MemoryKind::Episodic
}

fn build_runtime_heap(bundle: &RuntimeIndexBundle) -> UnifiedMemoryHeap<DefaultVerityMemoryPolicy> {
    let mut heap = UnifiedMemoryHeap::with_config(
        DefaultVerityMemoryPolicy,
        UnifiedMemoryHeapConfig {
            owner_settings: OwnerScopeSettings::default(),
        },
    );
    let route = runtime_memory_route();
    let capability = runtime_memory_capability();
    for entry in &bundle.entries {
        let entity_refs = parse_entity_refs_from_tags(&entry.tags);
        let _ = heap.write_memory_object(
            &route,
            "core:memory_runtime",
            &capability,
            MemoryObject {
                object_id: entry.node_id.clone(),
                scope: MemoryScope::Core,
                kind: infer_kind(&entry.tags),
                classification: Classification::Internal,
                namespace: "memory.runtime.index".to_string(),
                key: entry.uid.clone(),
                payload: payload_from_entry(entry),
                metadata: json!({
                    "entity_refs": entity_refs,
                    "tags": entry.tags,
                    "file": entry.file_rel,
                    "node_id": entry.node_id,
                    "uid": entry.uid,
                }),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            TrustState::Validated,
            vec![format!("runtime_index:{}", entry.node_id)],
        );
    }
    heap
}

fn recall_query_from_args(
    q: &str,
    top: usize,
    session_entity_hints: Vec<String>,
) -> MemoryRecallQuery {
    MemoryRecallQuery {
        query: q.to_string(),
        requested_scopes: vec![MemoryScope::Core],
        top_k: top,
        allowed_kinds: Vec::new(),
        session_entity_hints,
    }
}

fn remember_session_entities(root: &Path, args: &HashMap<String, String>, session_id: &str, hits: &[MemoryRecallHit]) {
    let entity_refs = hits
        .iter()
        .flat_map(|hit| hit.explanation.matched_entity_ids.clone())
        .chain(hits.iter().flat_map(|hit| hit.explanation.expanded_entity_ids.clone()))
        .take(12)
        .collect::<Vec<String>>();
    if entity_refs.is_empty() {
        return;
    }
    let db_path = arg_any(args, &["db-path", "db_path"]);
    if let Ok(db) = MemoryDb::open(root, &db_path) {
        let _ = db.set_hot_state_json(
            &format!("session_recall::{session_id}"),
            &json!({ "entity_refs": entity_refs }),
        );
    }
}

fn load_session_entities(root: &Path, args: &HashMap<String, String>, session_id: &str) -> Vec<String> {
    let db_path = arg_any(args, &["db-path", "db_path"]);
    let Ok(db) = MemoryDb::open(root, &db_path) else {
        return Vec::new();
    };
    let Ok(Some(value)) = db.get_hot_state_json(&format!("session_recall::{session_id}")) else {
        return Vec::new();
    };
    value
        .get("entity_refs")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|rows| rows.iter().filter_map(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<String>>()
}

fn hybrid_query_hits(
    root: &Path,
    args: &HashMap<String, String>,
    bundle: &RuntimeIndexBundle,
    q: &str,
    top: usize,
    session_id: Option<&str>,
) -> Vec<QueryHit> {
    let session_entity_hints = session_id
        .map(|id| load_session_entities(root, args, id))
        .unwrap_or_default();
    let heap = build_runtime_heap(bundle);
    let capability = runtime_memory_capability();
    let recall_hits = heap
        .hybrid_recall(
            "core:memory_runtime",
            &capability,
            recall_query_from_args(q, top, session_entity_hints),
        )
        .unwrap_or_default();
    if let Some(id) = session_id {
        remember_session_entities(root, args, id, &recall_hits);
    }
    recall_hits
        .into_iter()
        .map(|hit| QueryHit {
            node_id: hit.object_id.clone(),
            uid: hit
                .payload
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            file: hit
                .payload
                .get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            summary: hit.summary,
            tags: hit
                .payload
                .get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flat_map(|rows| rows.iter().filter_map(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<String>>(),
            score: hit.score,
            reasons: hit.explanation.rationale.clone(),
            memory_kind: Some(format!("{:?}", hit.kind).to_ascii_lowercase()),
            trust_state: Some(format!("{:?}", hit.trust_state).to_ascii_lowercase()),
            entity_refs: Some(hit.explanation.matched_entity_ids.clone()),
            recall_explanation: serde_json::to_value(&hit.explanation).ok(),
            section_excerpt: None,
            section_hash: None,
            section_source: None,
            expand_blocked: None,
            expand_error: None,
        })
        .collect::<Vec<QueryHit>>()
}
