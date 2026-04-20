fn normalize_search_query_url_candidate(raw: &str) -> String {
    let mut out = search_strip_invisible_unicode(&clean_text(raw, 2_200))
        .trim()
        .to_string();
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
    out = out.replace("&amp;", "&");
    clean_text(&out, 2_100)
}
