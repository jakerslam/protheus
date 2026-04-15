use std::collections::{BTreeMap, BTreeSet};

use super::graph_query_acceleration_state::relation_label;
use super::graph_query_acceleration_types::{GraphQueryTerm, KnowledgeTriplePattern};
use super::KnowledgeGraph;

impl KnowledgeGraph {
    pub(crate) fn estimate_pattern_cardinality(&self, pattern: &KnowledgeTriplePattern) -> usize {
        let relation_key = pattern.relation.as_ref().map(relation_label);
        let subject = term_bound_value(&pattern.subject, &BTreeMap::new());
        let object = term_bound_value(&pattern.object, &BTreeMap::new());
        match (subject.as_deref(), relation_key, object.as_deref()) {
            (Some(subject), Some(relation), Some(object)) => {
                if self
                    .acceleration
                    .triple_indexes
                    .has_edge(subject, relation, object)
                {
                    1
                } else {
                    0
                }
            }
            (Some(subject), Some(relation), None) => self
                .acceleration
                .triple_indexes
                .spo
                .get(&(subject.to_string(), relation.to_string()))
                .map(BTreeSet::len)
                .unwrap_or(0),
            (None, Some(relation), Some(object)) => self
                .acceleration
                .triple_indexes
                .pos
                .get(&(relation.to_string(), object.to_string()))
                .map(BTreeSet::len)
                .unwrap_or(0),
            (Some(subject), None, Some(object)) => self
                .acceleration
                .triple_indexes
                .sop
                .get(&(subject.to_string(), object.to_string()))
                .map(BTreeSet::len)
                .unwrap_or(0),
            (None, Some(relation), None) => self
                .acceleration
                .predicate_counts
                .get(relation)
                .copied()
                .unwrap_or(self.edges.len()),
            _ => self.edges.len(),
        }
    }

    pub(crate) fn build_variable_domains(
        &self,
        patterns: &[KnowledgeTriplePattern],
    ) -> BTreeMap<String, BTreeSet<String>> {
        let mut domains = BTreeMap::<String, Vec<BTreeSet<String>>>::new();
        for pattern in patterns {
            let Some(relation) = pattern.relation.as_ref().map(relation_label) else {
                continue;
            };
            match (&pattern.subject, &pattern.object) {
                (GraphQueryTerm::Var(subject_var), GraphQueryTerm::Node(object_node)) => {
                    if let Some(candidates) = self
                        .acceleration
                        .triple_indexes
                        .pos
                        .get(&(relation.to_string(), object_node.clone()))
                        .cloned()
                    {
                        domains
                            .entry(subject_var.clone())
                            .or_default()
                            .push(candidates);
                    }
                }
                (GraphQueryTerm::Node(subject_node), GraphQueryTerm::Var(object_var)) => {
                    if let Some(candidates) = self
                        .acceleration
                        .triple_indexes
                        .spo
                        .get(&(subject_node.clone(), relation.to_string()))
                        .cloned()
                    {
                        domains
                            .entry(object_var.clone())
                            .or_default()
                            .push(candidates);
                    }
                }
                _ => {}
            }
        }
        domains
            .into_iter()
            .map(|(var, sets)| (var, leapfrog_intersection(sets)))
            .collect::<BTreeMap<String, BTreeSet<String>>>()
    }

    pub(crate) fn match_pattern(
        &self,
        pattern: &KnowledgeTriplePattern,
        binding: &BTreeMap<String, String>,
        variable_domains: &BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<(BTreeMap<String, String>, String)> {
        let subject = term_bound_value(&pattern.subject, binding);
        let object = term_bound_value(&pattern.object, binding);
        let relation = pattern.relation.as_ref().map(relation_label);

        let candidates = self
            .edges
            .iter()
            .filter(|edge| {
                if let Some(required) = relation {
                    if relation_label(&edge.relation) != required {
                        return false;
                    }
                    if let Some(source) = subject.as_deref() {
                        if !self
                            .acceleration
                            .relation_source_might_exist(required, source)
                        {
                            return false;
                        }
                    }
                }
                if let Some(source) = subject.as_deref() {
                    if edge.source_entity_id != source {
                        return false;
                    }
                }
                if let Some(target) = object.as_deref() {
                    if edge.target_entity_id != target {
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>();

        let mut out = Vec::<(BTreeMap<String, String>, String)>::new();
        for edge in candidates {
            let mut next = binding.clone();
            if !bind_term(
                &pattern.subject,
                &edge.source_entity_id,
                &mut next,
                variable_domains,
            ) {
                continue;
            }
            if !bind_term(
                &pattern.object,
                &edge.target_entity_id,
                &mut next,
                variable_domains,
            ) {
                continue;
            }
            out.push((next, edge.edge_id.clone()));
        }
        out
    }

    pub(crate) fn rebuild_entity_embeddings_if_needed(&mut self, dims: usize) {
        if !self.acceleration.entity_embeddings.is_empty() {
            return;
        }
        for node in self.nodes.values() {
            let neighbors = self
                .adjacency
                .get(&node.entity_id)
                .cloned()
                .unwrap_or_default()
                .join(" ");
            let text = format!("{} {} {}", node.label, node.aliases.join(" "), neighbors);
            let embedding = crate::vector_index::embed_text(text.as_str(), dims);
            let bucket = ann_bucket_key(&embedding);
            self.acceleration
                .ann_buckets
                .entry(bucket)
                .or_default()
                .push(node.entity_id.clone());
            self.acceleration
                .entity_embeddings
                .insert(node.entity_id.clone(), embedding);
        }
    }
}

fn term_bound_value(term: &GraphQueryTerm, binding: &BTreeMap<String, String>) -> Option<String> {
    match term {
        GraphQueryTerm::Node(value) => Some(value.clone()),
        GraphQueryTerm::Var(var) => binding.get(var).cloned(),
    }
}

fn bind_term(
    term: &GraphQueryTerm,
    value: &str,
    binding: &mut BTreeMap<String, String>,
    variable_domains: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    match term {
        GraphQueryTerm::Node(node) => node == value,
        GraphQueryTerm::Var(var) => {
            if let Some(domain) = variable_domains.get(var) {
                if !domain.contains(value) {
                    return false;
                }
            }
            if let Some(existing) = binding.get(var) {
                existing == value
            } else {
                binding.insert(var.clone(), value.to_string());
                true
            }
        }
    }
}

pub(crate) fn pattern_fingerprint(pattern: &KnowledgeTriplePattern) -> String {
    let relation = pattern
        .relation
        .as_ref()
        .map(relation_label)
        .unwrap_or("*")
        .to_string();
    let subject = match &pattern.subject {
        GraphQueryTerm::Node(node) => format!("node:{node}"),
        GraphQueryTerm::Var(var) => format!("var:{var}"),
    };
    let object = match &pattern.object {
        GraphQueryTerm::Node(node) => format!("node:{node}"),
        GraphQueryTerm::Var(var) => format!("var:{var}"),
    };
    format!("{subject}|{relation}|{object}")
}

fn ann_bucket_key(embedding: &[f32]) -> String {
    embedding
        .iter()
        .take(4)
        .map(|row| if *row >= 0.0 { "1" } else { "0" })
        .collect::<Vec<&str>>()
        .join("")
}

fn leapfrog_intersection(mut sets: Vec<BTreeSet<String>>) -> BTreeSet<String> {
    if sets.is_empty() {
        return BTreeSet::new();
    }
    if sets.len() == 1 {
        return sets.pop().unwrap_or_default();
    }
    let arrays = sets
        .into_iter()
        .map(|set| set.into_iter().collect::<Vec<String>>())
        .collect::<Vec<Vec<String>>>();
    if arrays.iter().any(Vec::is_empty) {
        return BTreeSet::new();
    }
    let mut pointers = vec![0usize; arrays.len()];
    let mut out = BTreeSet::<String>::new();
    loop {
        let mut current_max = arrays[0][pointers[0]].clone();
        for (idx, array) in arrays.iter().enumerate().skip(1) {
            let current = &array[pointers[idx]];
            if current > &current_max {
                current_max = current.clone();
            }
        }
        let mut advanced = false;
        let mut all_equal = true;
        for (idx, array) in arrays.iter().enumerate() {
            while pointers[idx] < array.len() && array[pointers[idx]] < current_max {
                pointers[idx] += 1;
                advanced = true;
            }
            if pointers[idx] >= array.len() {
                return out;
            }
            if array[pointers[idx]] != current_max {
                all_equal = false;
            }
        }
        if all_equal {
            out.insert(current_max.clone());
            for pointer in &mut pointers {
                *pointer += 1;
            }
            if pointers
                .iter()
                .enumerate()
                .any(|(idx, pointer)| *pointer >= arrays[idx].len())
            {
                return out;
            }
            continue;
        }
        if !advanced {
            for pointer in &mut pointers {
                *pointer += 1;
            }
            if pointers
                .iter()
                .enumerate()
                .any(|(idx, pointer)| *pointer >= arrays[idx].len())
            {
                return out;
            }
        }
    }
}
