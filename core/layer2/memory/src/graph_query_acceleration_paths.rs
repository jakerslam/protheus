use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use super::graph_query_acceleration_state::relation_label;
use super::graph_query_acceleration_types::{
    GraphPathQuery, GraphPathResult, GraphTraversalAlgorithm,
};
use super::KnowledgeGraph;

#[derive(Debug, Clone, Eq, PartialEq)]
struct ScoredNode {
    node: String,
    score: i64,
}

impl Ord for ScoredNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .cmp(&other.score)
            .then_with(|| self.node.cmp(&other.node))
    }
}

impl PartialOrd for ScoredNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl KnowledgeGraph {
    pub fn find_path(&self, query: GraphPathQuery) -> Option<GraphPathResult> {
        match query.algorithm {
            GraphTraversalAlgorithm::Bfs => self.bfs_path(&query),
            GraphTraversalAlgorithm::Dfs => self.dfs_path(&query),
            GraphTraversalAlgorithm::Dijkstra => self.dijkstra_path(&query),
            GraphTraversalAlgorithm::AStar => self.a_star_path(&query),
            GraphTraversalAlgorithm::Bidirectional => self.bidirectional_path(&query),
        }
    }

    fn bfs_path(&self, query: &GraphPathQuery) -> Option<GraphPathResult> {
        let mut queue = VecDeque::from([query.start_entity_id.clone()]);
        let mut parent = BTreeMap::<String, String>::new();
        let mut seen = BTreeSet::from([query.start_entity_id.clone()]);
        while let Some(current) = queue.pop_front() {
            if current == query.target_entity_id {
                return Some(reconstruct_path_result(parent, query, seen.len(), 0));
            }
            if depth_from_parent(&parent, &query.start_entity_id, &current) >= query.max_depth {
                continue;
            }
            for neighbor in self.adjacency.get(&current).cloned().unwrap_or_default() {
                if !edge_allowed_between(self, &current, &neighbor, query) {
                    continue;
                }
                if seen.insert(neighbor.clone()) {
                    parent.insert(neighbor.clone(), current.clone());
                    queue.push_back(neighbor);
                }
            }
        }
        None
    }

    fn dfs_path(&self, query: &GraphPathQuery) -> Option<GraphPathResult> {
        let mut stack = vec![(query.start_entity_id.clone(), 0usize)];
        let mut parent = BTreeMap::<String, String>::new();
        let mut seen = BTreeSet::from([query.start_entity_id.clone()]);
        while let Some((current, depth)) = stack.pop() {
            if current == query.target_entity_id {
                return Some(reconstruct_path_result(parent, query, seen.len(), 0));
            }
            if depth >= query.max_depth {
                continue;
            }
            for neighbor in self.adjacency.get(&current).cloned().unwrap_or_default() {
                if !edge_allowed_between(self, &current, &neighbor, query) {
                    continue;
                }
                if seen.insert(neighbor.clone()) {
                    parent.insert(neighbor.clone(), current.clone());
                    stack.push((neighbor, depth + 1));
                }
            }
        }
        None
    }

    fn dijkstra_path(&self, query: &GraphPathQuery) -> Option<GraphPathResult> {
        self.a_star_with_heuristic(query, false)
    }

    fn a_star_path(&self, query: &GraphPathQuery) -> Option<GraphPathResult> {
        self.a_star_with_heuristic(query, true)
    }

    fn bidirectional_path(&self, query: &GraphPathQuery) -> Option<GraphPathResult> {
        let mut left = BTreeSet::from([query.start_entity_id.clone()]);
        let mut right = BTreeSet::from([query.target_entity_id.clone()]);
        let mut parent_left = BTreeMap::<String, String>::new();
        let mut parent_right = BTreeMap::<String, String>::new();
        let mut seen_left = left.clone();
        let mut seen_right = right.clone();
        while !left.is_empty() && !right.is_empty() {
            if left.len() <= right.len() {
                let mut next = BTreeSet::new();
                for node in left {
                    for neighbor in self.adjacency.get(&node).cloned().unwrap_or_default() {
                        if !edge_allowed_between(self, &node, &neighbor, query) {
                            continue;
                        }
                        if seen_left.insert(neighbor.clone()) {
                            parent_left.insert(neighbor.clone(), node.clone());
                            next.insert(neighbor.clone());
                        }
                        if seen_right.contains(&neighbor) {
                            let path = stitch_bidirectional_path(
                                &query.start_entity_id,
                                &query.target_entity_id,
                                &parent_left,
                                &parent_right,
                                &neighbor,
                            );
                            return Some(GraphPathResult {
                                path_entity_ids: path,
                                explored_nodes: seen_left.len() + seen_right.len(),
                                total_cost_milli: 0,
                                algorithm: GraphTraversalAlgorithm::Bidirectional,
                            });
                        }
                    }
                }
                left = next;
            } else {
                let mut next = BTreeSet::new();
                for node in right {
                    for neighbor in self.adjacency.get(&node).cloned().unwrap_or_default() {
                        if !edge_allowed_between(self, &node, &neighbor, query) {
                            continue;
                        }
                        if seen_right.insert(neighbor.clone()) {
                            parent_right.insert(neighbor.clone(), node.clone());
                            next.insert(neighbor.clone());
                        }
                        if seen_left.contains(&neighbor) {
                            let path = stitch_bidirectional_path(
                                &query.start_entity_id,
                                &query.target_entity_id,
                                &parent_left,
                                &parent_right,
                                &neighbor,
                            );
                            return Some(GraphPathResult {
                                path_entity_ids: path,
                                explored_nodes: seen_left.len() + seen_right.len(),
                                total_cost_milli: 0,
                                algorithm: GraphTraversalAlgorithm::Bidirectional,
                            });
                        }
                    }
                }
                right = next;
            }
        }
        None
    }

    fn a_star_with_heuristic(
        &self,
        query: &GraphPathQuery,
        use_heuristic: bool,
    ) -> Option<GraphPathResult> {
        let mut cost = BTreeMap::<String, i64>::new();
        let mut parent = BTreeMap::<String, String>::new();
        let mut heap = BinaryHeap::<ScoredNode>::new();
        cost.insert(query.start_entity_id.clone(), 0);
        heap.push(ScoredNode {
            node: query.start_entity_id.clone(),
            score: 0,
        });
        let mut explored = 0usize;
        while let Some(ScoredNode { node, .. }) = heap.pop() {
            explored = explored.saturating_add(1);
            if node == query.target_entity_id {
                let total = cost.get(&node).copied().unwrap_or(0).max(0) as u64;
                return Some(reconstruct_path_result(parent, query, explored, total));
            }
            for neighbor in self.adjacency.get(&node).cloned().unwrap_or_default() {
                if !edge_allowed_between(self, &node, &neighbor, query) {
                    continue;
                }
                let step = edge_cost_between(self, &node, &neighbor).unwrap_or(1000) as i64;
                let tentative = cost.get(&node).copied().unwrap_or(i64::MAX / 4) + step;
                let known = cost.get(&neighbor).copied().unwrap_or(i64::MAX / 2);
                if tentative < known {
                    cost.insert(neighbor.clone(), tentative);
                    parent.insert(neighbor.clone(), node.clone());
                    let heuristic = if use_heuristic {
                        heuristic_distance(self, neighbor.as_str(), query.target_entity_id.as_str())
                    } else {
                        0
                    };
                    heap.push(ScoredNode {
                        node: neighbor,
                        score: -(tentative + heuristic),
                    });
                }
            }
        }
        None
    }
}

fn edge_allowed_between(
    graph: &KnowledgeGraph,
    source: &str,
    target: &str,
    query: &GraphPathQuery,
) -> bool {
    graph.edges.iter().any(|edge| {
        let connected = (edge.source_entity_id == source && edge.target_entity_id == target)
            || (edge.source_entity_id == target && edge.target_entity_id == source);
        connected
            && (query.relation_filter.is_empty()
                || query
                    .relation_filter
                    .iter()
                    .any(|relation| relation_label(relation) == relation_label(&edge.relation)))
    })
}

fn edge_cost_between(graph: &KnowledgeGraph, source: &str, target: &str) -> Option<u64> {
    graph
        .edges
        .iter()
        .filter(|edge| {
            (edge.source_entity_id == source && edge.target_entity_id == target)
                || (edge.source_entity_id == target && edge.target_entity_id == source)
        })
        .map(|edge| u64::from(10001u16.saturating_sub(edge.weight_bps.max(1))))
        .min()
}

fn reconstruct_path_result(
    parent: BTreeMap<String, String>,
    query: &GraphPathQuery,
    explored_nodes: usize,
    total_cost_milli: u64,
) -> GraphPathResult {
    let mut out = vec![query.target_entity_id.clone()];
    let mut cursor = query.target_entity_id.clone();
    while cursor != query.start_entity_id {
        let Some(next) = parent.get(&cursor).cloned() else {
            break;
        };
        out.push(next.clone());
        cursor = next;
    }
    out.reverse();
    GraphPathResult {
        path_entity_ids: out,
        explored_nodes,
        total_cost_milli,
        algorithm: query.algorithm.clone(),
    }
}

fn depth_from_parent(parent: &BTreeMap<String, String>, start: &str, current: &str) -> usize {
    let mut depth = 0usize;
    let mut cursor = current.to_string();
    while cursor != start {
        let Some(next) = parent.get(&cursor) else {
            break;
        };
        cursor = next.clone();
        depth = depth.saturating_add(1);
    }
    depth
}

fn stitch_bidirectional_path(
    start: &str,
    target: &str,
    parent_left: &BTreeMap<String, String>,
    parent_right: &BTreeMap<String, String>,
    meeting: &str,
) -> Vec<String> {
    let mut left = vec![meeting.to_string()];
    let mut cursor = meeting.to_string();
    while cursor != start {
        let Some(next) = parent_left.get(&cursor).cloned() else {
            break;
        };
        left.push(next.clone());
        cursor = next;
    }
    left.reverse();
    let mut right = Vec::<String>::new();
    cursor = meeting.to_string();
    while cursor != target {
        let Some(next) = parent_right.get(&cursor).cloned() else {
            break;
        };
        right.push(next.clone());
        cursor = next;
    }
    left.extend(right);
    left
}

fn heuristic_distance(graph: &KnowledgeGraph, from: &str, to: &str) -> i64 {
    let Some(left) = graph.nodes.get(from) else {
        return 1;
    };
    let Some(right) = graph.nodes.get(to) else {
        return 1;
    };
    let left_terms = left.aliases.iter().cloned().collect::<BTreeSet<String>>();
    let right_terms = right.aliases.iter().cloned().collect::<BTreeSet<String>>();
    let overlap = left_terms.intersection(&right_terms).count() as i64;
    (3 - overlap).max(0)
}
