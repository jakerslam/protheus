fn classify_builder_risk(goal: &str, explicit: Option<&String>) -> String {
    if let Some(raw) = explicit {
        let normalized = raw.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "low" | "medium" | "high") {
            return normalized;
        }
    }
    let lower = goal.to_ascii_lowercase();
    let high_terms = [
        "delete",
        "drop table",
        "production",
        "payment",
        "security",
        "auth bypass",
    ];
    if high_terms.iter().any(|term| lower.contains(term)) {
        return "high".to_string();
    }
    let medium_terms = [
        "deploy",
        "migration",
        "schema",
        "customer data",
        "live traffic",
    ];
    if medium_terms.iter().any(|term| lower.contains(term)) {
        return "medium".to_string();
    }
    "low".to_string()
}
