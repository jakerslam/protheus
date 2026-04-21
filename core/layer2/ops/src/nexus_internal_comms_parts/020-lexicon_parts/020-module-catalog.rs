
fn module_catalog() -> Vec<&'static str> {
    module_specs().into_iter().map(|spec| spec.name).collect()
}

fn module_entries(module: &str) -> Vec<(&'static str, &'static str)> {
    let normalized = normalize_module_name(module);
    module_specs()
        .into_iter()
        .find(|spec| spec.name == normalized.as_str())
        .map(|spec| spec.entries.to_vec())
        .unwrap_or_default()
}

fn module_catalog_manifest() -> Value {
    Value::Array(
        module_specs()
            .into_iter()
            .map(|spec| {
                json!({
                    "name": spec.name,
                    "symbol_count": spec.entries.len(),
                    "entries": spec.entries
                        .iter()
                        .map(|(code, phrase)| json!({"code": code, "phrase": phrase}))
                        .collect::<Vec<_>>(),
                    "task_keywords": spec.task_keywords,
                    "role_keywords": spec.role_keywords,
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn normalize_module_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn known_modules() -> BTreeSet<&'static str> {
    module_catalog().into_iter().collect::<BTreeSet<_>>()
}

fn module_limit_error(got: usize) -> String {
    format!("module_limit_exceeded:max={MAX_MODULES_PER_AGENT}:got={got}")
}

fn validated_module_token(
    raw_module: &str,
    known: &BTreeSet<&'static str>,
) -> Result<Option<String>, String> {
    let module = normalize_module_name(raw_module);
    if module.is_empty() {
        return Ok(None);
    }
    if !known.contains(module.as_str()) {
        return Err(format!("unknown_module:{module}"));
    }
    Ok(Some(module))
}

fn parse_modules(argv: &[String]) -> Result<Vec<String>, String> {
    let raw = parse_flag(argv, "modules")
        .or_else(|| parse_flag(argv, "module"))
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let known = known_modules();
    let mut modules = Vec::<String>::new();
    let mut seen = BTreeSet::<String>::new();
    for raw_item in raw.split(',') {
        if let Some(module) = validated_module_token(raw_item, &known)? {
            if seen.insert(module.clone()) {
                modules.push(module);
            }
        }
    }
    if modules.len() > MAX_MODULES_PER_AGENT {
        return Err(module_limit_error(modules.len()));
    }
    Ok(modules)
}

fn module_context_scores(
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Vec<(String, u64)> {
    let task_norm = task.map(normalize_text_atom).unwrap_or_default();
    let role_norm = role.map(normalize_text_atom).unwrap_or_default();
    let extra_norm = extra_text.map(normalize_text_atom).unwrap_or_default();
    if task_norm.is_empty() && role_norm.is_empty() && extra_norm.is_empty() {
        return Vec::new();
    }
    let mut scored = Vec::<(String, u64)>::new();
    for spec in module_specs() {
        let mut score = 0u64;
        for raw_kw in spec.task_keywords {
            let kw = normalize_text_atom(raw_kw);
            if kw.is_empty() {
                continue;
            }
            if task_norm.contains(kw.as_str()) {
                score += 3;
            }
            if extra_norm.contains(kw.as_str()) {
                score += 1;
            }
        }
        for raw_kw in spec.role_keywords {
            let kw = normalize_text_atom(raw_kw);
            if !kw.is_empty() && role_norm.contains(kw.as_str()) {
                score += 4;
            }
        }
        if !task_norm.is_empty() && task_norm.contains(spec.name) {
            score += 2;
        }
        if !role_norm.is_empty() && role_norm.contains(spec.name) {
            score += 2;
        }
        if score > 0 {
            scored.push((spec.name.to_string(), score));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored
}

fn infer_modules_for_task(
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Vec<String> {
    module_context_scores(task, role, extra_text)
        .into_iter()
        .map(|(module, _)| module)
        .take(MAX_MODULES_PER_AGENT)
        .collect()
}

fn resolve_modules_for_context(
    argv: &[String],
    seeded_modules: &[String],
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut modules = parse_modules(argv)?;
    let known = known_modules();
    let mut seen = modules.iter().cloned().collect::<BTreeSet<_>>();

    for raw_seed in seeded_modules {
        let Some(seeded) = validated_module_token(raw_seed, &known)? else {
            continue;
        };
        if seen.contains(&seeded) {
            continue;
        }
        if modules.len() >= MAX_MODULES_PER_AGENT {
            return Err(module_limit_error(modules.len() + 1));
        }
        seen.insert(seeded.clone());
        modules.push(seeded);
    }

    if modules.is_empty() {
        for inferred in infer_modules_for_task(task, role, extra_text) {
            if modules.len() >= MAX_MODULES_PER_AGENT {
                break;
            }
            if seen.insert(inferred.clone()) {
                modules.push(inferred);
            }
        }
    }
    Ok(modules)
}

fn active_lexicon(modules: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::<String, String>::new();
    for (code, phrase) in core_lexicon_entries() {
        out.insert(code.to_string(), phrase.to_string());
    }
    let core_codes = out.keys().cloned().collect::<BTreeSet<_>>();
    for module in modules {
        for (code, phrase) in module_entries(module) {
            if core_codes.contains(code) {
                return Err(format!("module_redefines_core_symbol:{module}:{code}"));
            }
            if out.contains_key(code) {
                return Err(format!("module_symbol_collision:{module}:{code}"));
            }
            out.insert(code.to_string(), phrase.to_string());
        }
    }
    Ok(out)
}

fn reverse_lexicon(lexicon: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::<String, String>::new();
    for (code, phrase) in lexicon {
        out.insert(normalize_text_atom(phrase), code.clone());
    }
    out
}
