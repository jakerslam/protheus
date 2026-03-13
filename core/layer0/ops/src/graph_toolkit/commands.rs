// SPDX-License-Identifier: Apache-2.0

use super::algorithms::{
    betweenness_centrality, community_groups, jaccard_score, label_groups, label_propagation,
    louvain_simple, neighbor_sets, pagerank, predict_links, top_jaccard_pairs,
};
use super::input::GraphData;
use super::receipts::{emit, materialize_and_emit};
use crate::v8_kernel::{parse_bool, parse_f64, parse_u64};
use crate::{clean, ParsedArgs};
use serde_json::{json, Value};
use std::path::Path;

fn map_result_rows(nodes: &[String], scores: &[f64], key: &str, descending: bool) -> Vec<Value> {
    let mut out = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            json!({
                "node": node,
                key: scores.get(idx).copied().unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();
    out.sort_by(|left, right| {
        let l_score = left.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        let r_score = right.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        let cmp = if descending {
            r_score
                .partial_cmp(&l_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            l_score
                .partial_cmp(&r_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        };
        cmp.then_with(|| {
            left.get("node")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(right.get("node").and_then(Value::as_str).unwrap_or(""))
        })
    });
    out
}

fn node_index(nodes: &[String], name: &str) -> Option<usize> {
    nodes.iter().position(|node| node == name)
}

pub fn run_pagerank(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let damping = parse_f64(parsed.flags.get("damping"), 0.85).clamp(0.0, 1.0);
    let iterations = parse_u64(parsed.flags.get("iterations"), 24).clamp(1, 512) as usize;
    let scores = pagerank(&graph.out_adj, damping, iterations);
    let rows = map_result_rows(&graph.nodes, &scores, "score", true);
    materialize_and_emit(
        root,
        parsed,
        "pagerank",
        "graph_toolkit_pagerank",
        "V6-TOOLS-008.1",
        "graph:pagerank",
        graph,
        json!({
            "damping": damping,
            "iterations": iterations
        }),
        json!({
            "scores": rows
        }),
    )
}

pub fn run_louvain(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let max_iter = parse_u64(parsed.flags.get("max-iter"), 24).clamp(1, 256) as usize;
    let (assignments, modularity, passes) =
        louvain_simple(&graph.nodes, &graph.undirected_adj, max_iter);
    let groups = community_groups(&assignments)
        .into_iter()
        .map(|(community_id, members)| {
            let nodes = members
                .iter()
                .filter_map(|idx| graph.nodes.get(*idx).cloned())
                .collect::<Vec<_>>();
            json!({
                "community_id": community_id,
                "size": nodes.len(),
                "nodes": nodes
            })
        })
        .collect::<Vec<_>>();
    materialize_and_emit(
        root,
        parsed,
        "louvain",
        "graph_toolkit_louvain",
        "V6-TOOLS-008.2",
        "graph:louvain",
        graph,
        json!({
            "max_iter": max_iter
        }),
        json!({
            "passes": passes,
            "modularity": modularity,
            "communities": groups
        }),
    )
}

pub fn run_jaccard(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let neighbors = neighbor_sets(&graph.undirected_adj);
    let source = parsed.flags.get("source").map(|v| clean(v, 120));
    let target = parsed.flags.get("target").map(|v| clean(v, 120));
    let top_k = parse_u64(parsed.flags.get("top-k"), 20).clamp(1, 2000) as usize;

    let result = if let (Some(source_node), Some(target_node)) = (source.clone(), target.clone()) {
        let source_idx = node_index(&graph.nodes, &source_node);
        let target_idx = node_index(&graph.nodes, &target_node);
        match (source_idx, target_idx) {
            (Some(a), Some(b)) => {
                let (score, intersection, union) = jaccard_score(&neighbors, a, b);
                json!({
                    "source": source_node,
                    "target": target_node,
                    "score": score,
                    "intersection": intersection,
                    "union": union
                })
            }
            _ => {
                return emit(
                    root,
                    json!({
                        "ok": false,
                        "type": "graph_toolkit_jaccard",
                        "lane": "core/layer0/ops",
                        "error": "node_not_found",
                        "source": source_node,
                        "target": target_node
                    }),
                );
            }
        }
    } else {
        let pairs = top_jaccard_pairs(&neighbors, top_k)
            .into_iter()
            .map(|(a, b, score, intersection, union)| {
                json!({
                    "source": graph.nodes[a],
                    "target": graph.nodes[b],
                    "score": score,
                    "intersection": intersection,
                    "union": union
                })
            })
            .collect::<Vec<_>>();
        json!({ "pairs": pairs })
    };

    materialize_and_emit(
        root,
        parsed,
        "jaccard",
        "graph_toolkit_jaccard",
        "V6-TOOLS-008.3",
        "graph:jaccard",
        graph,
        json!({
            "source": source,
            "target": target,
            "top_k": top_k
        }),
        result,
    )
}

pub fn run_label_propagation(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let max_iter = parse_u64(parsed.flags.get("max-iter"), 32).clamp(1, 512) as usize;
    let (labels, rounds) = label_propagation(&graph.nodes, &graph.undirected_adj, max_iter);
    let groups = label_groups(&labels)
        .into_iter()
        .map(|(label, members)| {
            let nodes = members
                .iter()
                .filter_map(|idx| graph.nodes.get(*idx).cloned())
                .collect::<Vec<_>>();
            json!({
                "label": label,
                "size": nodes.len(),
                "nodes": nodes
            })
        })
        .collect::<Vec<_>>();
    materialize_and_emit(
        root,
        parsed,
        "label-propagation",
        "graph_toolkit_label_propagation",
        "V6-TOOLS-008.3",
        "graph:label-propagation",
        graph,
        json!({
            "max_iter": max_iter
        }),
        json!({
            "rounds": rounds,
            "groups": groups
        }),
    )
}

pub fn run_betweenness(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let normalize = parse_bool(parsed.flags.get("normalize"), true);
    let scores = betweenness_centrality(&graph.undirected_adj, normalize);
    let rows = map_result_rows(&graph.nodes, &scores, "score", true);
    materialize_and_emit(
        root,
        parsed,
        "betweenness",
        "graph_toolkit_betweenness",
        "V6-TOOLS-008.4",
        "graph:betweenness",
        graph,
        json!({
            "normalize": normalize
        }),
        json!({
            "scores": rows
        }),
    )
}

pub fn run_link_prediction(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let top_k = parse_u64(parsed.flags.get("top-k"), 20).clamp(1, 2000) as usize;
    let pagerank_scores = pagerank(&graph.out_adj, 0.85, 32);
    let rows = predict_links(
        &graph.undirected_adj,
        &graph.existing_edges,
        &pagerank_scores,
        top_k,
    )
    .into_iter()
    .map(|row| {
        json!({
            "source": graph.nodes[row.a],
            "target": graph.nodes[row.b],
            "score": row.score,
            "common_neighbors": row.common_neighbors,
            "preferential_attachment": row.preferential_attachment,
            "jaccard": row.jaccard,
            "pagerank_pair_sum": row.pagerank_pair_sum
        })
    })
    .collect::<Vec<_>>();
    materialize_and_emit(
        root,
        parsed,
        "predict-links",
        "graph_toolkit_predict_links",
        "V6-TOOLS-008.4",
        "graph:predict-links",
        graph,
        json!({
            "top_k": top_k
        }),
        json!({
            "predictions": rows
        }),
    )
}

pub fn run_centrality(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let metric = parsed
        .flags
        .get("metric")
        .map(|v| clean(v, 60).to_ascii_lowercase())
        .unwrap_or_else(|| "pagerank".to_string());
    match metric.as_str() {
        "pagerank" => run_pagerank(root, parsed, graph),
        "betweenness" => run_betweenness(root, parsed, graph),
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "graph_toolkit_centrality",
                "lane": "core/layer0/ops",
                "error": "unsupported_metric",
                "metric": metric
            }),
        ),
    }
}

pub fn run_communities(root: &Path, parsed: &ParsedArgs, graph: &GraphData) -> i32 {
    let algo = parsed
        .flags
        .get("algo")
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .unwrap_or_else(|| "louvain".to_string());
    match algo.as_str() {
        "louvain" => run_louvain(root, parsed, graph),
        "label-propagation" | "label_propagation" | "label" => {
            run_label_propagation(root, parsed, graph)
        }
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "graph_toolkit_communities",
                "lane": "core/layer0/ops",
                "error": "unsupported_algo",
                "algo": algo
            }),
        ),
    }
}
