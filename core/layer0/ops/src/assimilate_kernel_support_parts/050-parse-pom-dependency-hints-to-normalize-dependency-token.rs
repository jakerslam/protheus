
fn parse_pom_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut search = raw;
    while let Some(start) = search.find("<artifactId>") {
        let rest = &search[start + "<artifactId>".len()..];
        let Some(end) = rest.find("</artifactId>") else {
            break;
        };
        let dep = rest[..end].trim();
        if !dep.is_empty() && dep != "${project.artifactId}" {
            out.insert(dep.to_string());
        }
        search = &rest[end + "</artifactId>".len()..];
    }
    out
}

fn parse_gradle_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("implementation")
            && !trimmed.contains("api ")
            && !trimmed.contains("testImplementation")
        {
            continue;
        }
        if let Some(idx) = trimmed.find('\'') {
            let rhs = &trimmed[idx + 1..];
            if let Some(end) = rhs.find('\'') {
                let dep = rhs[..end]
                    .split(':')
                    .nth(1)
                    .unwrap_or(rhs[..end].trim())
                    .trim();
                if !dep.is_empty() {
                    out.insert(dep.to_string());
                }
            }
        }
    }
    out
}

fn add_framework_markers(raw: &str, out: &mut BTreeSet<String>) {
    let lower = raw.to_ascii_lowercase();
    let markers = [
        "langgraph",
        "openai-agents",
        "openai_agents",
        "openai",
        "crewai",
        "mastra",
        "llamaindex",
        "llama-index",
        "haystack",
        "dspy",
        "pydantic-ai",
        "pydantic_ai",
        "camel-ai",
        "camel",
        "google-adk",
        "google_adk",
    ];
    for marker in markers {
        if lower.contains(marker) {
            out.insert(marker.to_string());
        }
    }
}

fn dependency_hints(kind: &str, raw: &str) -> Vec<String> {
    let mut out = match kind {
        "cargo_toml" => parse_cargo_dependency_hints(raw),
        "package_json" => parse_package_json_dependency_hints(raw),
        "pyproject_toml" => parse_pyproject_dependency_hints(raw),
        "requirements_txt" => parse_requirements_dependency_hints(raw),
        "go_mod" => parse_go_mod_dependency_hints(raw),
        "pom_xml" => parse_pom_dependency_hints(raw),
        "gradle" => parse_gradle_dependency_hints(raw),
        _ => BTreeSet::<String>::new(),
    };
    add_framework_markers(raw, &mut out);
    out.into_iter().take(120).collect::<Vec<_>>()
}

fn parse_cargo_package_name(raw: &str) -> Option<String> {
    let mut in_package = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((left, right)) = trimmed.split_once('=') {
            if left.trim() == "name" {
                let value = right
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_pyproject_package_name(raw: &str) -> Option<String> {
    let mut in_project = false;
    let mut in_poetry = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_project = trimmed == "[project]";
            in_poetry = trimmed == "[tool.poetry]";
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_project || in_poetry {
            if let Some((left, right)) = trimmed.split_once('=') {
                if left.trim() == "name" {
                    let value = right
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

fn parse_go_module_name(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("module ") {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_pom_package_name(raw: &str) -> Option<String> {
    if let Some(start) = raw.find("<artifactId>") {
        let rest = &raw[start + "<artifactId>".len()..];
        if let Some(end) = rest.find("</artifactId>") {
            let value = rest[..end].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_gradle_package_name(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rhs) = trimmed.strip_prefix("rootProject.name") {
            let value = rhs
                .split('=')
                .nth(1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn parse_manifest_package_name(kind: &str, raw: &str) -> Option<String> {
    match kind {
        "cargo_toml" => parse_cargo_package_name(raw),
        "package_json" => serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|v| v.get("name").and_then(Value::as_str).map(|s| s.to_string())),
        "pyproject_toml" => parse_pyproject_package_name(raw),
        "go_mod" => parse_go_module_name(raw),
        "pom_xml" => parse_pom_package_name(raw),
        "gradle" => parse_gradle_package_name(raw),
        _ => None,
    }
}

fn normalize_dependency_token(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
        .replace('_', "-")
}
