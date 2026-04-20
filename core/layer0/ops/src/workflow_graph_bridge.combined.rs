// Split from workflow_graph_bridge.combined.rs into focused include parts for maintainability.
include!("workflow_graph_bridge.combined_parts/010-prelude-and-shared.rs");
include!("workflow_graph_bridge.combined_parts/020-default-state-rel-to-semantic-claim.rs");
include!("workflow_graph_bridge.combined_parts/030-emit-native-trace-to-checkpoint-run.rs");
include!("workflow_graph_bridge.combined_parts/040-inspect-state-to-resume-run.rs");
include!("workflow_graph_bridge.combined_parts/050-coordinate-subgraph-to-select-edge.rs");
include!("workflow_graph_bridge.combined_parts/060-stream-graph-to-run.rs");
