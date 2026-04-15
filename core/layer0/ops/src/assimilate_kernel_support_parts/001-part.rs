fn infer_target_root(target: &str) -> Option<PathBuf> {
    let candidate = target.trim();
    if candidate.is_empty() {
        return None;
    }
    if candidate.starts_with("http://")
        || candidate.starts_with("https://")
        || candidate.contains("://")
    {
        return None;
    }
    let path = PathBuf::from(candidate);
    if path.is_absolute() && path.exists() {
        return fs::canonicalize(path).ok();
    }
    let root = repo_root_from_env_or_cwd();
    let joined = root.join(path);
    if joined.exists() {
        return fs::canonicalize(joined).ok();
    }
    None
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(normalized_path_text)
        .unwrap_or_else(|| normalized_path_text(path))
}

fn sha256_hex(raw: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw);
    format!("sha256:{:x}", hasher.finalize())
}

fn manifest_kind(path: &Path) -> Option<&'static str> {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match name.as_str() {
        "cargo.toml" => Some("cargo_toml"),
        "package.json" => Some("package_json"),
        "pyproject.toml" => Some("pyproject_toml"),
        "requirements.txt" => Some("requirements_txt"),
        "go.mod" => Some("go_mod"),
        "pom.xml" => Some("pom_xml"),
        "build.gradle" | "build.gradle.kts" => Some("gradle"),
        _ => None,
    }
}

fn parse_cargo_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_dependency_table = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dependency_table = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[workspace.dependencies]";
            continue;
        }
        if !in_dependency_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let dep = name.trim();
            if !dep.is_empty() {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

fn parse_package_json_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let parsed = serde_json::from_str::<Value>(raw).ok();
    let Some(payload) = parsed else {
        return out;
    };
    let keys = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];
    for key in keys {
        let Some(row) = payload.get(key).and_then(Value::as_object) else {
            continue;
        };
        for dep in row.keys() {
            out.insert(dep.to_string());
        }
    }
    out
}

fn parse_requirements_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let dep = trimmed
            .split(['=', '>', '<', '!', '~', ';', '['])
            .next()
            .unwrap_or("")
            .trim();
        if !dep.is_empty() {
            out.insert(dep.to_string());
        }
    }
    out
}

fn parse_pyproject_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_dependencies = false;
    let mut in_optional_dependencies = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dependencies = trimmed == "[project]" || trimmed == "[tool.poetry.dependencies]";
            in_optional_dependencies = trimmed.starts_with("[project.optional-dependencies.");
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_dependencies && trimmed.starts_with("dependencies") && trimmed.contains('[') {
            if let Some((_, rhs)) = trimmed.split_once('=') {
                for token in rhs.split([',', '[', ']', '"', '\'']) {
                    let dep = token.trim();
                    if dep.is_empty() || dep.contains(' ') || dep.contains('=') {
                        continue;
                    }
                    out.insert(dep.to_string());
                }
            }
            continue;
        }
        if in_optional_dependencies {
            for token in trimmed.split([',', '[', ']', '"', '\'']) {
                let dep = token.trim();
                if dep.is_empty() || dep.contains(' ') || dep.contains('=') {
                    continue;
                }
                out.insert(dep.to_string());
            }
            continue;
        }
        if trimmed.contains('=') && (in_dependencies || trimmed.starts_with("name")) {
            let dep = trimmed
                .split('=')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !dep.is_empty() && dep != "python" && dep != "name" {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

fn parse_go_mod_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_require_block = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }
        if in_require_block || trimmed.starts_with("require ") {
            let dep = trimmed
                .trim_start_matches("require")
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if !dep.is_empty() {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

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

