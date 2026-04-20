
#[cfg(test)]
mod provider_http_interop_tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn extract_openai_text_preserves_multiline_list_layout() {
        let payload = json!({
            "choices": [{
                "message": {
                    "content": "1. First item\n2. Second item\n   - nested detail"
                }
            }]
        });
        let text = extract_openai_text(&payload);
        assert!(text.contains("1. First item\n2. Second item"));
        assert!(text.contains("\n   - nested detail"));
    }

    #[test]
    fn parse_ollama_list_models_reads_name_column() {
        let raw = "\
NAME                             ID              SIZE      MODIFIED
qwen3:8b                         500a1f067a9f    5.2 GB    7 weeks ago
kimi-k2.5:cloud                  6d1c3246c608    -         7 weeks ago
";
        let rows = parse_ollama_list_models(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "kimi-k2.5:cloud".to_string()]
        );
    }

    #[test]
    fn parse_ollama_list_models_json_reads_array_rows() {
        let raw = r#"
[
  {"name":"qwen3:8b","model":"qwen3:8b"},
  {"name":"smallthinker:latest","model":"smallthinker:latest"}
]
"#;
        let rows = parse_ollama_list_models_json(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "smallthinker:latest".to_string()]
        );
    }

    #[test]
    fn ollama_base_url_candidates_include_default_loopback() {
        let rows = ollama_base_url_candidates("127.0.0.1:11434");
        assert!(rows.iter().any(|row| row == "http://127.0.0.1:11434"));
        assert!(rows.iter().any(|row| row == "http://localhost:11434"));
    }

    #[test]
    fn provider_rows_marks_ollama_reachable_when_cli_lists_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let bin_dir = tempfile::tempdir().expect("tempdir");
        let ollama_path = bin_dir.path().join("ollama");
        let script = r#"#!/bin/sh
if [ "$1" = "list" ] && [ "$2" = "--json" ]; then
  printf '[{"name":"qwen3:4b","model":"qwen3:4b"},{"name":"smallthinker:latest","model":"smallthinker:latest"}]\n'
  exit 0
fi
if [ "$1" = "list" ]; then
  printf 'NAME ID SIZE MODIFIED\nqwen3:4b deadbeef 3.2GB now\n'
  exit 0
fi
exit 1
"#;
        fs::write(&ollama_path, script).expect("write ollama stub");
        #[cfg(unix)]
        {
            fs::set_permissions(&ollama_path, fs::Permissions::from_mode(0o755))
                .expect("chmod ollama stub");
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.path().display(), old_path);
        std::env::set_var("PATH", new_path);

        let rows = provider_rows(root.path(), &json!({}));
        let ollama = rows
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("ollama"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        std::env::set_var("PATH", old_path);

        assert_eq!(ollama.get("reachable").and_then(Value::as_bool), Some(true));
        assert!(ollama
            .get("detected_models")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .any(|row| row.as_str() == Some("qwen3:4b"))
            })
            .unwrap_or(false));
    }
}

