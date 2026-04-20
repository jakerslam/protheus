
fn rerank_score(query: &str, candidate: &Candidate) -> f64 {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let framework_catalog_intent = is_framework_catalog_intent(query);
    let query_tokens = query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let haystack = format!("{} {}", candidate.title, candidate.snippet).to_ascii_lowercase();
    let overlap = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count() as f64;
    let overlap_norm = if query_tokens.is_empty() {
        0.0
    } else {
        overlap / query_tokens.len() as f64
    };
    let locator_bonus = if candidate.locator.is_empty() {
        0.0
    } else {
        0.2
    };
    let status_bonus = if (200..400).contains(&candidate.status_code) {
        0.2
    } else {
        0.0
    };
    let metric_bonus = if benchmark_intent && looks_like_metric_rich_text(&candidate.snippet) {
        0.24
    } else {
        0.0
    };
    let framework_catalog_bonus = if framework_catalog_intent
        && looks_like_framework_catalog_text(&format!("{} {}", candidate.title, candidate.snippet))
    {
        0.18
    } else {
        0.0
    };
    let framework_catalog_source_bonus = if framework_catalog_intent {
        framework_catalog_source_adjustment(candidate)
    } else {
        0.0
    };
    let definition_penalty = if benchmark_intent && looks_like_definition_candidate(candidate) {
        0.72
    } else {
        0.0
    };
    let comparison_noise_penalty =
        if benchmark_intent && looks_like_comparison_noise_candidate(candidate) {
            0.65
        } else {
            0.0
        };
    let mut score = 0.6 * overlap_norm
        + locator_bonus
        + status_bonus
        + metric_bonus
        + framework_catalog_bonus
        + framework_catalog_source_bonus
        - definition_penalty
        - comparison_noise_penalty;
    if benchmark_intent && !looks_like_metric_rich_text(&candidate.snippet) {
        score -= 0.12;
    }
    score.clamp(0.0, 1.0)
}

fn minimum_synthesis_score(benchmark_intent: bool) -> f64 {
    if benchmark_intent {
        0.33
    } else {
        0.18
    }
}

fn retrieve_web_candidates_for_query_with_timeout(
    root: &Path,
    query: &str,
    timeout: Duration,
) -> Result<Vec<Candidate>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<Candidate>, String>>();
    let root_buf = root.to_path_buf();
    let query_buf = query.to_string();
    let spawned = thread::Builder::new()
        .name("batch-query-retrieve".to_string())
        .spawn(move || {
            let out = retrieve_web_candidates_for_query(&root_buf, &query_buf);
            let _ = tx.send(out);
        });
    if spawned.is_err() {
        return Err("query_worker_spawn_failed".to_string());
    }
    match rx.recv_timeout(timeout) {
        Ok(out) => out,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("query_timeout_ms_{}", timeout.as_millis()))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err("query_worker_disconnected".to_string())
        }
    }
}

fn is_benign_partial_failure(detail: &str) -> bool {
    let lowered = clean_text(detail, 320).to_ascii_lowercase();
    if lowered.contains("anti_bot_challenge") {
        return false;
    }
    lowered.contains("candidate_low_relevance")
        || lowered.contains("fetch_candidate_low_relevance")
        || lowered.contains("no_usable_summary")
        || lowered.contains("fixture_missing")
}
