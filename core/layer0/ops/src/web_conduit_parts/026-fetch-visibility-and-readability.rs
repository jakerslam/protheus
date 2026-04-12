fn regex_html_comments() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<!--.*?-->").expect("regex"))
}

fn regex_article() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<article[^>]*>(.*?)</article>").expect("regex"))
}

fn regex_main() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<main[^>]*>(.*?)</main>").expect("regex"))
}

fn regex_class_attr() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?is)\bclass\s*=\s*["']([^"']*)["']"#).expect("regex"))
}

fn regex_style_attr() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?is)\bstyle\s*=\s*["']([^"']*)["']"#).expect("regex"))
}

fn regex_type_attr() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?is)\btype\s*=\s*["']([^"']*)["']"#).expect("regex"))
}

fn regex_hidden_attr() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)\bhidden(?:\s|>|/)").expect("regex"))
}

fn regex_style_color_rgba_zero() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bcolor\s*:\s*rgba\s*\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*0(?:\.0+)?\s*\)")
            .expect("regex")
    })
}

fn regex_style_color_hsla_zero() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bcolor\s*:\s*hsla\s*\(\s*[\d.]+\s*,\s*[\d.]+%?\s*,\s*[\d.]+%?\s*,\s*0(?:\.0+)?\s*\)")
            .expect("regex")
    })
}

fn regex_style_clip_path_hidden() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bclip-path\s*:\s*inset\s*\(\s*(?:0*\.\d+|[1-9]\d*(?:\.\d+)?)%")
            .expect("regex")
    })
}

fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            let value = *ch;
            !matches!(
                value,
                '\u{200B}'..='\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2060}'..='\u{2064}'
                    | '\u{206A}'..='\u{206F}'
                    | '\u{FEFF}'
                    | '\u{E0000}'..='\u{E007F}'
            ) && (!value.is_control() || matches!(value, '\n' | '\r' | '\t'))
        })
        .collect::<String>()
}

fn html_hidden_class_tokens(value: &str) -> bool {
    value.split_whitespace().any(|token| {
        matches!(
            token.trim().to_ascii_lowercase().as_str(),
            "sr-only"
                | "visually-hidden"
                | "d-none"
                | "hidden"
                | "invisible"
                | "screen-reader-only"
                | "offscreen"
        )
    })
}

fn html_style_hides_content(style: &str) -> bool {
    let normalized = normalize_block_text(style).to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    normalized.contains("display:none")
        || normalized.contains("display: none")
        || normalized.contains("visibility:hidden")
        || normalized.contains("visibility: hidden")
        || normalized.contains("opacity:0")
        || normalized.contains("opacity: 0")
        || normalized.contains("font-size:0")
        || normalized.contains("font-size: 0")
        || normalized.contains("text-indent:-9999px")
        || normalized.contains("text-indent: -9999px")
        || normalized.contains("color:transparent")
        || normalized.contains("color: transparent")
        || normalized.contains("transform:scale(0)")
        || normalized.contains("transform: scale(0)")
        || normalized.contains("translatex(-9999px)")
        || normalized.contains("translatey(-9999px)")
        || normalized.contains("left:-9999px")
        || normalized.contains("left: -9999px")
        || normalized.contains("top:-9999px")
        || normalized.contains("top: -9999px")
        || (normalized.contains("width:0")
            && normalized.contains("height:0")
            && normalized.contains("overflow:hidden"))
        || (normalized.contains("width: 0")
            && normalized.contains("height: 0")
            && normalized.contains("overflow: hidden"))
        || regex_style_color_rgba_zero().is_match(&normalized)
        || regex_style_color_hsla_zero().is_match(&normalized)
        || regex_style_clip_path_hidden().is_match(&normalized)
}

fn html_tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start_matches('/')
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn html_tag_self_closing(tag: &str, tag_name: &str) -> bool {
    tag.trim_end().ends_with("/>")
        || matches!(
            tag_name,
            "meta" | "input" | "img" | "br" | "hr" | "link" | "source"
        )
}

fn html_should_remove_tag(tag_name: &str, tag_markup: &str) -> bool {
    if matches!(
        tag_name,
        "meta"
            | "template"
            | "svg"
            | "canvas"
            | "iframe"
            | "object"
            | "embed"
            | "nav"
            | "footer"
            | "aside"
    ) {
        return true;
    }
    let lowered = tag_markup.to_ascii_lowercase();
    if tag_name == "input" {
        if regex_type_attr()
            .captures(tag_markup)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_ascii_lowercase()))
            .as_deref()
            == Some("hidden")
        {
            return true;
        }
    }
    if lowered.contains("aria-hidden=\"true\"")
        || lowered.contains("aria-hidden='true'")
        || regex_hidden_attr().is_match(tag_markup)
    {
        return true;
    }
    if regex_class_attr()
        .captures(tag_markup)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .map(|value| html_hidden_class_tokens(&value))
        .unwrap_or(false)
    {
        return true;
    }
    if regex_style_attr()
        .captures(tag_markup)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .map(|value| html_style_hides_content(&value))
        .unwrap_or(false)
    {
        return true;
    }
    false
}

fn sanitize_html_visibility(raw_html: &str) -> String {
    let source = regex_html_comments().replace_all(raw_html, "").to_string();
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    let mut stack: Vec<(String, bool)> = Vec::new();
    let mut hidden_depth = 0usize;

    while idx < bytes.len() {
        if bytes[idx] != b'<' {
            if hidden_depth == 0 {
                out.push(bytes[idx] as char);
            }
            idx += 1;
            continue;
        }
        let Some(end_rel) = source[idx..].find('>') else {
            if hidden_depth == 0 {
                out.push_str(&source[idx..]);
            }
            break;
        };
        let end = idx + end_rel + 1;
        let tag = &source[idx..end];
        let closing = tag.trim_start().starts_with("</");
        let tag_name = html_tag_name(tag);
        if tag_name.is_empty() {
            if hidden_depth == 0 {
                out.push_str(tag);
            }
            idx = end;
            continue;
        }
        if closing {
            let mut matched_hidden = false;
            if let Some(position) = stack.iter().rposition(|(name, _)| *name == tag_name) {
                while stack.len() > position {
                    let (_, hidden) = stack.pop().expect("stack pop");
                    if hidden {
                        hidden_depth = hidden_depth.saturating_sub(1);
                    }
                    matched_hidden = hidden;
                }
            }
            if hidden_depth == 0 && !matched_hidden {
                out.push_str(tag);
            }
        } else {
            let hidden = html_should_remove_tag(&tag_name, tag);
            let self_closing = html_tag_self_closing(tag, &tag_name);
            if hidden {
                if !self_closing {
                    hidden_depth += 1;
                    stack.push((tag_name, true));
                }
            } else {
                if hidden_depth == 0 {
                    out.push_str(tag);
                }
                if !self_closing {
                    stack.push((tag_name, false));
                }
            }
        }
        idx = end;
    }
    out
}

fn select_readable_html_body(raw_html: &str) -> String {
    let sanitized = sanitize_html_visibility(raw_html);
    let article = regex_article()
        .captures(&sanitized)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default();
    if !normalize_block_text(&strip_tags_to_text(&article)).is_empty() {
        return article;
    }
    let main = regex_main()
        .captures(&sanitized)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default();
    if !normalize_block_text(&strip_tags_to_text(&main)).is_empty() {
        return main;
    }
    sanitized
}
