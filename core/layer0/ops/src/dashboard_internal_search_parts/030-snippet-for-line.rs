
fn snippet_for_line(line: &str, terms: &[String]) -> String {
    let value = clean_text(line, 260);
    if value.is_empty() {
        return String::new();
    }
    let lower = value.to_ascii_lowercase();
    let mut focus_start = None::<usize>;
    for term in terms {
        if term.is_empty() {
            continue;
        }
        if let Some(idx) = lower.find(term) {
            focus_start = Some(idx);
            break;
        }
    }
    let (start, end) = if let Some(idx) = focus_start {
        let mut left = idx.saturating_sub(42);
        let mut right = (idx + 78).min(value.len());
        while left > 0 && !value.is_char_boundary(left) {
            left -= 1;
        }
        while right < value.len() && !value.is_char_boundary(right) {
            right += 1;
        }
        (left, right.min(value.len()))
    } else {
        (0, value.len().min(120))
    };
    let excerpt = value
        .get(start..end)
        .map(|row| clean_text(row, 180))
        .unwrap_or_else(|| clean_text(&value, 180));
    if excerpt.is_empty() {
        return String::new();
    }
    let words = excerpt.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return String::new();
    }
    let first_term_hit = words.iter().position(|word| {
        let lw = word.to_ascii_lowercase();
        terms
            .iter()
            .any(|term| !term.is_empty() && lw.contains(term))
    });
    let first_meaningful = first_term_hit.unwrap_or_else(|| {
        words
            .iter()
            .position(|word| !is_stop_word(&word.to_ascii_lowercase()))
            .unwrap_or(0)
    });
    let compact = words[first_meaningful..].join(" ");
    let clipped = clean_text(&compact, 176);
    if clipped.is_empty() {
        return String::new();
    }
    format!("...[{}]...", clipped)
}

pub fn search_conversations(root: &Path, query: &str, limit: usize) -> Value {
    let cleaned_query = clean_text(query, 260);
    let terms = tokenize_for_search(&cleaned_query);
    if cleaned_query.is_empty() || terms.is_empty() {
        return json!({
            "ok": true,
            "type": "dashboard_conversation_search",
            "query": cleaned_query,
            "results": []
        });
    }

    let mut scored = Vec::<(i64, String, Value)>::new();
    for doc in collect_documents(root) {
        let name_lc = doc.name.to_ascii_lowercase();
        let mut score: i64 = 0;
        if name_lc == cleaned_query.to_ascii_lowercase() {
            score += 180;
        }
        if name_lc.starts_with(&cleaned_query.to_ascii_lowercase()) {
            score += 120;
        }
        if name_lc.contains(&cleaned_query.to_ascii_lowercase()) {
            score += 84;
        }
        let mut best_line_score = 0i64;
        let mut best_line = String::new();
        for line in &doc.lines {
            let line_lc = line.to_ascii_lowercase();
            let mut line_score = 0i64;
            if line_lc.contains(&cleaned_query.to_ascii_lowercase()) {
                line_score += 42;
            }
            for term in &terms {
                if name_lc.contains(term) {
                    score += 16;
                }
                if line_lc.contains(term) {
                    line_score += 10;
                }
            }
            if line_score > best_line_score {
                best_line_score = line_score;
                best_line = line.clone();
            }
        }
        score += best_line_score;
        if score <= 0 {
            continue;
        }
        let snippet = if !best_line.is_empty() {
            snippet_for_line(&best_line, &terms)
        } else {
            format!("...[{}]...", clean_text(&doc.name, 96))
        };
        let payload = json!({
            "agent_id": doc.agent_id,
            "name": doc.name,
            "snippet": snippet,
            "score": score,
            "archived": doc.archived,
            "state": doc.state,
            "avatar_url": doc.avatar_url,
            "emoji": doc.emoji,
            "updated_at": doc.updated_at
        });
        scored.push((
            score,
            payload
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            payload,
        ));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    let capped = limit.clamp(1, 120);
    let results = scored
        .into_iter()
        .take(capped)
        .map(|(_, _, payload)| payload)
        .collect::<Vec<_>>();
    json!({
        "ok": true,
        "type": "dashboard_conversation_search",
        "query": cleaned_query,
        "terms": terms,
        "results": results
    })
}

#[cfg(test)]
include!("../dashboard_internal_search_tests.rs");
