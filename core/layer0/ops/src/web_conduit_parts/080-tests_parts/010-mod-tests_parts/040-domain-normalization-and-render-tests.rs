
    #[test]
    fn normalize_allowed_domains_supports_comma_string() {
        let domains =
            normalize_allowed_domains(&json!("https://www.github.com, docs.rs *.example.com"));
        assert_eq!(
            domains,
            vec![
                "github.com".to_string(),
                "docs.rs".to_string(),
                "example.com".to_string()
            ]
        );
    }

    #[test]
    fn domain_allowed_scope_respects_exact_domain_mode() {
        let filters = vec!["example.com".to_string()];
        assert!(domain_allowed_for_scope(
            "https://example.com/docs",
            &filters,
            true
        ));
        assert!(!domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            true
        ));
        assert!(domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            false
        ));
    }

    #[test]
    fn render_serper_payload_filters_domains_and_builds_content() {
        let body = serde_json::to_string(&json!({
            "organic": [
                {
                    "title": "Main",
                    "link": "https://example.com/main",
                    "snippet": "Main domain snippet"
                },
                {
                    "title": "Subdomain",
                    "link": "https://blog.example.com/post",
                    "snippet": "Subdomain snippet"
                },
                {
                    "title": "Other",
                    "link": "https://other.com/page",
                    "snippet": "Other domain snippet"
                }
            ]
        }))
        .expect("encode");
        let rendered =
            render_serper_payload(&body, &vec!["example.com".to_string()], true, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            rendered.get("provider_raw_count").and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            rendered
                .get("provider_filtered_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        let links = rendered
            .get("links")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), Some("https://example.com/main"));
    }

    #[test]
    fn render_serper_payload_handles_invalid_json() {
        let rendered = render_serper_payload("not-json", &[], false, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            rendered.get("error").and_then(Value::as_str),
            Some("serper_decode_failed")
        );
    }
