// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

mod algorithms;
mod commands;
mod input;
mod receipts;

use crate::parse_args;
use commands::{
    run_betweenness, run_centrality, run_communities, run_jaccard, run_label_propagation,
    run_link_prediction, run_louvain, run_pagerank,
};
use input::parse_graph_input;
use receipts::{command_status, emit};
use serde_json::json;
use std::path::Path;

const STATE_ENV: &str = "GRAPH_TOOLKIT_STATE_ROOT";
const STATE_SCOPE: &str = "graph_toolkit";
const ROUTE_TAG: &str = "conduit";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops graph-toolkit status");
    println!("  protheus-ops graph-toolkit pagerank [--graph-json-base64=<b64>] [--dataset=memory-vault|code-graph] [--damping=0.85] [--iterations=24]");
    println!("  protheus-ops graph-toolkit louvain [--graph-json-base64=<b64>] [--max-iter=24]");
    println!(
        "  protheus-ops graph-toolkit jaccard [--source=<node>] [--target=<node>] [--top-k=20]"
    );
    println!("  protheus-ops graph-toolkit label-propagation [--max-iter=32]");
    println!("  protheus-ops graph-toolkit betweenness [--normalize=1|0]");
    println!("  protheus-ops graph-toolkit predict-links [--top-k=20]");
    println!("  protheus-ops graph-toolkit centrality [--metric=pagerank|betweenness]");
    println!("  protheus-ops graph-toolkit communities [--algo=louvain|label-propagation]");
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    if command == "status" {
        return command_status(root);
    }

    let graph = match parse_graph_input(root, &parsed) {
        Ok(value) => value,
        Err(err) => {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "graph_toolkit_error",
                    "lane": "core/layer0/ops",
                    "error": err
                }),
            );
        }
    };

    match command.as_str() {
        "pagerank" => run_pagerank(root, &parsed, &graph),
        "louvain" => run_louvain(root, &parsed, &graph),
        "jaccard" => run_jaccard(root, &parsed, &graph),
        "label-propagation" | "label_propagation" | "label" => {
            run_label_propagation(root, &parsed, &graph)
        }
        "betweenness" => run_betweenness(root, &parsed, &graph),
        "predict-links" | "predict_links" | "link-predict" => {
            run_link_prediction(root, &parsed, &graph)
        }
        "centrality" => run_centrality(root, &parsed, &graph),
        "communities" => run_communities(root, &parsed, &graph),
        _ => emit(
            root,
            json!({
                "ok": false,
                "type": "graph_toolkit_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        ),
    }
}

#[cfg(test)]
mod tests;
