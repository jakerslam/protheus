use super::*;
use serde_json::json;

fn build_graph() -> KnowledgeGraph {
    let mut graph = KnowledgeGraph::default();
    graph.upsert_entity(
        "person:alice",
        KnowledgeEntityKind::Person,
        "Alice",
        vec!["alice".to_string()],
        vec!["v1".to_string()],
        900,
        json!({}),
    );
    graph.upsert_entity(
        "person:bob",
        KnowledgeEntityKind::Person,
        "Bob",
        vec!["bob".to_string()],
        vec!["v2".to_string()],
        700,
        json!({}),
    );
    graph.upsert_entity(
        "project:atlas",
        KnowledgeEntityKind::Project,
        "Atlas",
        vec!["atlas".to_string()],
        vec!["v3".to_string()],
        800,
        json!({}),
    );
    graph.upsert_entity(
        "system:postgres",
        KnowledgeEntityKind::System,
        "Postgres",
        vec!["postgres".to_string()],
        vec!["v4".to_string()],
        600,
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
            "system:postgres",
            KnowledgeRelationKind::DependsOn,
            vec!["v2".to_string()],
            8500,
        )
        .expect("atlas depends on postgres");
    graph
        .connect(
            "person:bob",
            "project:atlas",
            KnowledgeRelationKind::Supports,
            vec!["v5".to_string()],
            7000,
        )
        .expect("bob supports atlas");
    graph
}

#[test]
fn planner_prefers_selective_pattern_order() {
    let graph = build_graph();
    let plan = graph.plan_triple_query(&[
        KnowledgeTriplePattern {
            subject: GraphQueryTerm::Var("x".to_string()),
            relation: Some(KnowledgeRelationKind::Owns),
            object: GraphQueryTerm::Var("y".to_string()),
        },
        KnowledgeTriplePattern {
            subject: GraphQueryTerm::Node("person:alice".to_string()),
            relation: Some(KnowledgeRelationKind::Owns),
            object: GraphQueryTerm::Var("y".to_string()),
        },
    ]);
    assert!(plan.steps[0].estimated_cardinality <= plan.steps[1].estimated_cardinality);
}

#[test]
fn query_execution_uses_cache_and_returns_bindings() {
    let mut graph = build_graph();
    let patterns = vec![
        KnowledgeTriplePattern {
            subject: GraphQueryTerm::Var("owner".to_string()),
            relation: Some(KnowledgeRelationKind::Owns),
            object: GraphQueryTerm::Node("project:atlas".to_string()),
        },
        KnowledgeTriplePattern {
            subject: GraphQueryTerm::Node("project:atlas".to_string()),
            relation: Some(KnowledgeRelationKind::DependsOn),
            object: GraphQueryTerm::Var("dependency".to_string()),
        },
    ];
    let first = graph.execute_triple_query(patterns.clone(), 8);
    assert_eq!(first.cache_status, "miss");
    assert!(first.bindings.iter().any(|row| row
        .get("owner")
        .map(|v| v == "person:alice")
        .unwrap_or(false)));
    let second = graph.execute_triple_query(patterns, 8);
    assert_eq!(second.cache_status, "exact_hit");
}

#[test]
fn bitmap_and_bloom_filters_surface_relation_candidates() {
    let graph = build_graph();
    assert!(graph.relation_exists_probabilistic("person:alice", KnowledgeRelationKind::Owns));
    let filtered = graph.filter_entities_with_all_relations(&[
        KnowledgeRelationKind::Owns,
        KnowledgeRelationKind::Supports,
    ]);
    assert!(filtered.iter().any(|row| row == "project:atlas"));
}

#[test]
fn path_algorithms_and_materialized_views_work() {
    let mut graph = build_graph();
    let path = graph.find_path(GraphPathQuery {
        start_entity_id: "person:alice".to_string(),
        target_entity_id: "system:postgres".to_string(),
        algorithm: GraphTraversalAlgorithm::Bidirectional,
        max_depth: 5,
        relation_filter: vec![],
    });
    assert!(path.is_some());
    let closure = graph.materialize_transitive_closure(KnowledgeRelationKind::DependsOn);
    assert!(closure
        .get("project:atlas")
        .map(|rows| rows.iter().any(|row| row == "system:postgres"))
        .unwrap_or(false));
}

#[test]
fn neighborhood_inference_sampling_embedding_and_partitioning_work() {
    let mut graph = build_graph();
    let summary = graph
        .neighborhood_summary("project:atlas")
        .expect("summary exists");
    assert!(summary.total_neighbors >= 2);
    let inferred = graph.materialize_inference_edges();
    assert!(inferred
        .iter()
        .all(|edge| edge.edge_id.starts_with("inferred_")));
    let sample = graph.sample_subgraph(GraphSamplingQuery {
        strategy: GraphSamplingStrategy::ForestFire,
        seed_entity_id: "project:atlas".to_string(),
        max_nodes: 3,
        spread_bps: 7000,
    });
    assert!(!sample.sampled_entity_ids.is_empty());
    let approx = graph.approximate_tail_candidates("person:alice", KnowledgeRelationKind::Owns, 3);
    assert!(!approx.is_empty());
    let partition = graph.build_partition_plan(GraphPartitionStrategy::Hash, 2);
    assert_eq!(partition.partitions.len(), 2);
}

#[test]
fn federated_plan_assigns_patterns_to_best_service() {
    let graph = build_graph();
    let plan = graph.build_federated_query_plan(
        &[KnowledgeTriplePattern {
            subject: GraphQueryTerm::Var("x".to_string()),
            relation: Some(KnowledgeRelationKind::Owns),
            object: GraphQueryTerm::Var("y".to_string()),
        }],
        vec![
            FederatedServiceProfile {
                service_id: "local".to_string(),
                supported_relations: vec![KnowledgeRelationKind::Owns],
                selectivity_hint_bps: 6000,
            },
            FederatedServiceProfile {
                service_id: "remote".to_string(),
                supported_relations: vec![KnowledgeRelationKind::Owns],
                selectivity_hint_bps: 8000,
            },
        ],
    );
    assert!(plan.ordered_services.iter().any(|row| row == "remote"));
    assert_eq!(plan.dispatch_steps[0].pattern_indexes, vec![0]);
}
