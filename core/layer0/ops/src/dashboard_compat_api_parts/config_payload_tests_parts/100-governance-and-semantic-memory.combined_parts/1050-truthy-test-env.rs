fn truthy_test_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(ref value) if value == "1" || value == "true" || value == "yes"
    )
}

#[test]
