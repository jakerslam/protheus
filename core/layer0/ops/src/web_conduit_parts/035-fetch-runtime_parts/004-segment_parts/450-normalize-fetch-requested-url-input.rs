fn normalize_fetch_requested_url_input(raw_requested_url: &str) -> String {
    let mut out = fetch_strip_invisible_unicode(&clean_text(raw_requested_url, 2_400))
        .trim()
        .to_string();
    for _ in 0..3 {
        if out.starts_with('<') && out.ends_with('>') && out.len() > 2 {
            out = out[1..out.len() - 1].trim().to_string();
        }
        if ((out.starts_with('"') && out.ends_with('"'))
            || (out.starts_with('\'') && out.ends_with('\''))
            || (out.starts_with('`') && out.ends_with('`')))
            && out.len() > 1
        {
            out = out[1..out.len() - 1].trim().to_string();
        }
        if out.starts_with('[') && out.contains("](") && out.ends_with(')') {
            if let Some(start) = out.find("](") {
                let candidate = out[start + 2..out.len() - 1].trim().to_string();
                if !candidate.is_empty() {
                    out = candidate;
                }
            }
        }
        if out.starts_with('(') && out.ends_with(')') && out.len() > 2 {
            let candidate = out[1..out.len() - 1].trim().to_string();
            if candidate.starts_with("http://") || candidate.starts_with("https://") {
                out = candidate;
            }
        }
    }
    out = out.replace("&amp;", "&");
    while out.ends_with('.')
        || out.ends_with(',')
        || out.ends_with('!')
        || out.ends_with('?')
        || out.ends_with(';')
        || out.ends_with(':')
        || out.ends_with(')')
        || out.ends_with(']')
    {
        out.pop();
    }
    if out.starts_with("//") && out.len() > 2 {
        out = format!("https:{}", out);
    }
    let lowered = out.to_ascii_lowercase();
    if !lowered.starts_with("http://")
        && !lowered.starts_with("https://")
        && fetch_url_looks_like_bare_domain(&out)
    {
        out = format!("https://{}", out);
    }
    clean_text(&out, 2_200)
}
