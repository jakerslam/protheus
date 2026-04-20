    fn extract_mode_literals(text: &str, call_name: &str) -> std::collections::BTreeSet<String> {
        let pattern = format!(r#"{}\s*\(\s*['"`]([^'"`]+)['"`]"#, regex::escape(call_name));
        let re = Regex::new(&pattern).expect("valid call regex");
        let static_mode_re =
            Regex::new(r"^[a-zA-Z0-9_-]+$").expect("valid static mode token regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty() && static_mode_re.is_match(mode))
            .collect()
    }

    fn extract_bridge_modes(text: &str, fn_name: &str) -> std::collections::BTreeSet<String> {
        let section_re = Regex::new(&format!(
            r#"(?s)function {}\s*\([^)]*\)\s*\{{.*?const fieldByMode:\s*AnyObj\s*=\s*\{{(.*?)\}}\s*(?:;|\r?\n)?"#,
            regex::escape(fn_name)
        ))
        .expect("valid section regex");
        let keys_re = Regex::new(r#"(?m)^\s*(?:([a-zA-Z0-9_]+)|['"]([^'"]+)['"])\s*:"#)
            .expect("valid key regex");
        let Some(section) = section_re
            .captures(text)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        else {
            return std::collections::BTreeSet::new();
        };
        keys_re
            .captures_iter(section)
            .filter_map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().trim().to_string())
            })
            .filter(|key| !key.is_empty())
            .collect()
    }

    fn extract_dispatch_modes(text: &str) -> std::collections::BTreeSet<String> {
        let re = Regex::new(r#"(?m)^\s*(?:if|else if) mode == "([^"]+)""#)
            .expect("valid dispatch regex");
        let block_comment_re = Regex::new(r"(?s)/\*.*?\*/").expect("valid block comment regex");
        let line_comment_re = Regex::new(r"(?m)//.*$").expect("valid line comment regex");
        let without_block = block_comment_re.replace_all(text, "");
        let cleaned = line_comment_re.replace_all(&without_block, "");
        re.captures_iter(cleaned.as_ref())
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .filter(|mode| !mode.is_empty())
            .collect()
    }

    #[test]
    fn extract_mode_literals_accepts_all_quote_styles() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive('beta', {});
const c = runInversionPrimitive(`gamma`, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha", "beta", "gamma"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_accepts_quoted_and_unquoted_keys() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    "beta-mode": "payload_beta",
    'gamma_mode': "payload_gamma"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_allows_non_string_values() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: payloadAlpha,
    "beta-mode": payloadBeta,
    'gamma_mode': payloadGamma
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta-mode", "gamma_mode"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_selects_requested_function_section() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha"
  };
}
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed_inversion = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected_inversion = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_inversion, expected_inversion);

        let parsed_other = extract_bridge_modes(bridge, "runOtherPrimitive");
        let expected_other = ["rogue"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed_other, expected_other);
    }

    #[test]
    fn extract_bridge_modes_allows_missing_trailing_semicolon() {
        let bridge = r#"
function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    alpha: "payload_alpha",
    beta: "payload_beta"
  }
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_bridge_modes_returns_empty_when_function_missing() {
        let bridge = r#"
function runOtherPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {
  const fieldByMode: AnyObj = {
    rogue: "payload_rogue"
  };
}
"#;
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        assert!(parsed.is_empty());
    }

    #[test]
    fn extract_bridge_modes_supports_crlf_lines() {
        let bridge = "function runInversionPrimitive(mode: string, data: AnyObj = {}, opts: AnyObj = {}) {\r\n  const fieldByMode: AnyObj = {\r\n    alpha: \"payload_alpha\",\r\n    beta: \"payload_beta\"\r\n  }\r\n}\r\n";
        let parsed = extract_bridge_modes(bridge, "runInversionPrimitive");
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

