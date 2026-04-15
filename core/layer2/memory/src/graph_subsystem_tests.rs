use super::*;
use serde_json::json;

#[test]
fn knowledge_graph_resolves_and_expands_multi_hop_entities() {
    let mut graph = KnowledgeGraph::default();
    graph.upsert_entity(
        "person:alice",
        KnowledgeEntityKind::Person,
        "Alice",
        vec![],
        vec!["v1".to_string()],
        500,
        json!({}),
    );
    graph.upsert_entity(
        "project:atlas",
        KnowledgeEntityKind::Project,
        "Project Atlas",
        vec!["atlas".to_string()],
        vec!["v2".to_string()],
        500,
        json!({}),
    );
    graph.upsert_entity(
        "system:postgresql",
        KnowledgeEntityKind::System,
        "PostgreSQL",
        vec!["postgres".to_string()],
        vec!["v3".to_string()],
        500,
        json!({}),
    );
    graph
        .connect(
            "person:alice",
            "project:atlas",
            KnowledgeRelationKind::Owns,
            vec!["v1".to_string()],
            9000,
        )
        .expect("alice owns atlas");
    graph
        .connect(
            "project:atlas",
            "system:postgresql",
            KnowledgeRelationKind::DependsOn,
            vec!["v2".to_string()],
            9000,
        )
        .expect("atlas depends on postgres");
    let resolved = graph.resolve_entities("Was Alice impacted?");
    assert_eq!(resolved[0].entity_id, "person:alice");
    let expanded = graph.expand_related_entity_ids(&["person:alice".to_string()], 2, 8);
    assert!(expanded.iter().any(|row| row == "project:atlas"));
    assert!(expanded.iter().any(|row| row == "system:postgresql"));
}

#[test]
fn duplicate_connect_updates_edge_without_expansion_duplication() {
    let mut graph = KnowledgeGraph::default();
    graph.upsert_entity(
        "person:alice",
        KnowledgeEntityKind::Person,
        "Alice",
        vec![],
        vec!["v1".to_string()],
        500,
        json!({}),
    );
    graph.upsert_entity(
        "project:atlas",
        KnowledgeEntityKind::Project,
        "Project Atlas",
        vec!["atlas".to_string()],
        vec!["v2".to_string()],
        500,
        json!({}),
    );
    graph
        .connect(
            "person:alice",
            "project:atlas",
            KnowledgeRelationKind::Owns,
            vec!["v1".to_string()],
            8000,
        )
        .expect("first edge insert");
    graph
        .connect(
            "person:alice",
            "project:atlas",
            KnowledgeRelationKind::Owns,
            vec!["v2".to_string()],
            9000,
        )
        .expect("same edge should update");
    let expanded = graph.expand_related_entity_ids(&["person:alice".to_string()], 3, 8);
    assert_eq!(
        expanded
            .iter()
            .filter(|row| *row == "project:atlas")
            .count(),
        1
    );
}
