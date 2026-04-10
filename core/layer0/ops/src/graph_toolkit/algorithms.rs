// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct LinkPrediction {
    pub a: usize,
    pub b: usize,
    pub common_neighbors: usize,
    pub preferential_attachment: f64,
    pub jaccard: f64,
    pub pagerank_pair_sum: f64,
    pub score: f64,
}

pub fn pagerank(adj: &[Vec<(usize, f64)>], damping: f64, iterations: usize) -> Vec<f64> {
    let n = adj.len();
    if n == 0 {
        return Vec::new();
    }
    let clamped_damping = damping.clamp(0.0, 1.0);
    let mut rank = vec![1.0 / (n as f64); n];

    for _ in 0..iterations.max(1) {
        let mut next = vec![(1.0 - clamped_damping) / (n as f64); n];
        let mut dangling_mass = 0.0f64;

        for i in 0..n {
            let out = &adj[i];
            if out.is_empty() {
                dangling_mass += rank[i];
                continue;
            }
            let denom = out.iter().map(|(_, w)| *w).sum::<f64>().max(1e-12);
            for (target, weight) in out {
                let share = (rank[i] * *weight) / denom;
                next[*target] += clamped_damping * share;
            }
        }

        if dangling_mass > 0.0 {
            let bonus = clamped_damping * dangling_mass / (n as f64);
            for value in &mut next {
                *value += bonus;
            }
        }

        let sum = next.iter().sum::<f64>().max(1e-12);
        for value in &mut next {
            *value /= sum;
        }
        rank = next;
    }

    rank
}

fn modularity(assignments: &[usize], adj: &[Vec<(usize, f64)>], degrees: &[f64], m2: f64) -> f64 {
    if assignments.is_empty() || m2 <= 0.0 {
        return 0.0;
    }
    let mut q = 0.0f64;
    for i in 0..adj.len() {
        for (j, w) in &adj[i] {
            if assignments[i] == assignments[*j] {
                q += *w - ((degrees[i] * degrees[*j]) / m2);
            }
        }
    }
    q / m2
}

pub fn louvain_simple(
    _nodes: &[String],
    undirected_adj: &[Vec<(usize, f64)>],
    max_iter: usize,
) -> (Vec<usize>, f64, usize) {
    let n = undirected_adj.len();
    if n == 0 {
        return (Vec::new(), 0.0, 0);
    }

    let mut assignments = (0..n).collect::<Vec<_>>();
    let degrees = undirected_adj
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, w)| *w).sum::<f64>())
        .collect::<Vec<_>>();
    let m2 = degrees.iter().sum::<f64>().max(1e-12);
    let mut passes = 0usize;

    for _ in 0..max_iter.max(1) {
        passes += 1;
        let mut moved = false;
        let mut base_q = modularity(&assignments, undirected_adj, &degrees, m2);

        for node in 0..n {
            let current = assignments[node];
            let mut candidates = BTreeSet::new();
            candidates.insert(current);
            for (neighbor, _) in &undirected_adj[node] {
                candidates.insert(assignments[*neighbor]);
            }

            let mut best_assignment = current;
            let mut best_q = base_q;
            for candidate in candidates {
                if candidate == current {
                    continue;
                }
                assignments[node] = candidate;
                let q = modularity(&assignments, undirected_adj, &degrees, m2);
                if (q - best_q) > 1e-12
                    || ((q - best_q).abs() <= 1e-12 && candidate < best_assignment)
                {
                    best_q = q;
                    best_assignment = candidate;
                }
            }
            assignments[node] = best_assignment;
            if best_assignment != current {
                moved = true;
                base_q = best_q;
            }
        }

        if !moved {
            break;
        }
    }

    let mut remap = BTreeMap::new();
    let mut next = 0usize;
    for assignment in &assignments {
        if !remap.contains_key(assignment) {
            remap.insert(*assignment, next);
            next += 1;
        }
    }
    for assignment in &mut assignments {
        if let Some(mapped) = remap.get(assignment) {
            *assignment = *mapped;
        }
    }

    let score = modularity(&assignments, undirected_adj, &degrees, m2);
    (assignments, score, passes)
}

pub fn label_propagation(
    nodes: &[String],
    undirected_adj: &[Vec<(usize, f64)>],
    max_iter: usize,
) -> (Vec<String>, usize) {
    let n = nodes.len();
    if n == 0 {
        return (Vec::new(), 0);
    }
    let mut labels = nodes.to_vec();
    let mut rounds = 0usize;

    for _ in 0..max_iter.max(1) {
        rounds += 1;
        let mut changed = false;

        for node in 0..n {
            let mut counts = BTreeMap::<String, f64>::new();
            for (neighbor, weight) in &undirected_adj[node] {
                let entry = counts.entry(labels[*neighbor].clone()).or_insert(0.0);
                *entry += (*weight).max(1.0);
            }
            if counts.is_empty() {
                continue;
            }
            let mut best_label = labels[node].clone();
            let mut best_score = -1.0f64;
            for (label, score) in counts {
                if (score - best_score) > 1e-12
                    || ((score - best_score).abs() <= 1e-12 && label < best_label)
                {
                    best_score = score;
                    best_label = label;
                }
            }
            if best_label != labels[node] {
                labels[node] = best_label;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    (labels, rounds)
}

pub fn neighbor_sets(undirected_adj: &[Vec<(usize, f64)>]) -> Vec<BTreeSet<usize>> {
    undirected_adj
        .iter()
        .map(|neighbors| {
            neighbors
                .iter()
                .map(|(idx, _)| *idx)
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>()
}

pub fn jaccard_score(neighbors: &[BTreeSet<usize>], a: usize, b: usize) -> (f64, usize, usize) {
    if a >= neighbors.len() || b >= neighbors.len() {
        return (0.0, 0, 0);
    }
    let (inter, union, score) = overlap_stats(&neighbors[a], &neighbors[b]);
    (score, inter, union)
}

fn overlap_stats(left: &BTreeSet<usize>, right: &BTreeSet<usize>) -> (usize, usize, f64) {
    let inter = left.intersection(right).count();
    let union = left.union(right).count();
    let score = if union == 0 {
        0.0
    } else {
        (inter as f64) / (union as f64)
    };
    (inter, union, score)
}

fn bounded_top_k(top_k: usize) -> usize {
    top_k.max(1)
}

pub fn top_jaccard_pairs(
    neighbors: &[BTreeSet<usize>],
    top_k: usize,
) -> Vec<(usize, usize, f64, usize, usize)> {
    let mut out = Vec::new();
    for a in 0..neighbors.len() {
        for b in (a + 1)..neighbors.len() {
            let (score, inter, union) = jaccard_score(neighbors, a, b);
            out.push((a, b, score, inter, union));
        }
    }
    out.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    out.truncate(bounded_top_k(top_k));
    out
}

pub fn betweenness_centrality(undirected_adj: &[Vec<(usize, f64)>], normalize: bool) -> Vec<f64> {
    let n = undirected_adj.len();
    if n == 0 {
        return Vec::new();
    }
    let mut scores = vec![0.0f64; n];
    let neighbors = undirected_adj
        .iter()
        .map(|adj| adj.iter().map(|(idx, _)| *idx).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    for source in 0..n {
        let mut stack = Vec::new();
        let mut pred = vec![Vec::<usize>::new(); n];
        let mut sigma = vec![0.0f64; n];
        let mut dist = vec![-1i64; n];
        let mut queue = VecDeque::new();
        sigma[source] = 1.0;
        dist[source] = 0;
        queue.push_back(source);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for w in &neighbors[v] {
                if dist[*w] < 0 {
                    dist[*w] = dist[v] + 1;
                    queue.push_back(*w);
                }
                if dist[*w] == dist[v] + 1 {
                    sigma[*w] += sigma[v];
                    pred[*w].push(v);
                }
            }
        }

        let mut delta = vec![0.0f64; n];
        while let Some(w) = stack.pop() {
            for v in &pred[w] {
                if sigma[w] > 0.0 {
                    delta[*v] += (sigma[*v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != source {
                scores[w] += delta[w];
            }
        }
    }

    for value in &mut scores {
        *value /= 2.0;
    }

    if normalize && n > 2 {
        let scale = ((n as f64 - 1.0) * (n as f64 - 2.0)) / 2.0;
        if scale > 0.0 {
            for value in &mut scores {
                *value /= scale;
            }
        }
    }

    scores
}

pub fn community_groups(assignments: &[usize]) -> BTreeMap<usize, Vec<usize>> {
    let mut out = BTreeMap::<usize, Vec<usize>>::new();
    for (idx, assignment) in assignments.iter().enumerate() {
        out.entry(*assignment).or_default().push(idx);
    }
    out
}

pub fn label_groups(labels: &[String]) -> BTreeMap<String, Vec<usize>> {
    let mut out = BTreeMap::<String, Vec<usize>>::new();
    for (idx, label) in labels.iter().enumerate() {
        out.entry(label.clone()).or_default().push(idx);
    }
    out
}

pub fn predict_links(
    undirected_adj: &[Vec<(usize, f64)>],
    existing_edges: &BTreeSet<(usize, usize)>,
    pagerank_scores: &[f64],
    top_k: usize,
) -> Vec<LinkPrediction> {
    let neighbors = neighbor_sets(undirected_adj);
    let mut out = Vec::<LinkPrediction>::new();

    for a in 0..neighbors.len() {
        for b in (a + 1)..neighbors.len() {
            if existing_edges.contains(&(a, b)) {
                continue;
            }
            let (jaccard, common_neighbors, _) = jaccard_score(&neighbors, a, b);
            let preferential_attachment = (neighbors[a].len() * neighbors[b].len()) as f64;
            let pagerank_pair_sum = pagerank_scores.get(a).copied().unwrap_or(0.0)
                + pagerank_scores.get(b).copied().unwrap_or(0.0);
            let score = common_neighbors as f64
                + (0.10 * preferential_attachment)
                + (0.50 * jaccard)
                + (0.20 * pagerank_pair_sum);

            out.push(LinkPrediction {
                a,
                b,
                common_neighbors,
                preferential_attachment,
                jaccard,
                pagerank_pair_sum,
                score,
            });
        }
    }

    out.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.common_neighbors.cmp(&left.common_neighbors))
            .then_with(|| left.a.cmp(&right.a))
            .then_with(|| left.b.cmp(&right.b))
    });
    out.truncate(bounded_top_k(top_k));
    out
}
