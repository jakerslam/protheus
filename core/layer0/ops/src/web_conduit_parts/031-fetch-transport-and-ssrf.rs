const FETCH_MARKDOWN_ACCEPT_HEADER: &str = "text/markdown, text/html;q=0.9, */*;q=0.1";

fn normalize_fetch_content_type(raw: &str) -> String {
    clean_text(raw, 120)
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn fetch_response_is_markdown(content_type: &str) -> bool {
    matches!(
        normalize_fetch_content_type(content_type).as_str(),
        "text/markdown" | "text/x-markdown" | "application/markdown"
    )
}

fn fetch_url_scheme(raw_url: &str) -> String {
    let cleaned = clean_text(raw_url, 2200).to_ascii_lowercase();
    if cleaned.starts_with("https://") {
        "https".to_string()
    } else if cleaned.starts_with("http://") {
        "http".to_string()
    } else {
        String::new()
    }
}

fn fetch_url_authority(raw_url: &str) -> String {
    let cleaned = clean_text(raw_url, 2200);
    let without_scheme = cleaned
        .strip_prefix("https://")
        .or_else(|| cleaned.strip_prefix("http://"))
        .unwrap_or(cleaned.as_str());
    clean_text(
        without_scheme
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .trim(),
        320,
    )
}

fn fetch_url_host(raw_url: &str) -> String {
    let authority = fetch_url_authority(raw_url);
    let host_port = authority
        .split('@')
        .next_back()
        .unwrap_or_default()
        .trim_matches('.');
    if let Some(rest) = host_port.strip_prefix('[') {
        return clean_text(rest.split(']').next().unwrap_or_default(), 220).to_ascii_lowercase();
    }
    clean_text(
        host_port
            .split(':')
            .next()
            .unwrap_or_default()
            .trim_matches('.'),
        220,
    )
    .to_ascii_lowercase()
}

fn fetch_url_origin(raw_url: &str) -> String {
    let scheme = fetch_url_scheme(raw_url);
    let authority = fetch_url_authority(raw_url);
    if scheme.is_empty() || authority.is_empty() {
        String::new()
    } else {
        format!("{scheme}://{authority}")
    }
}

fn resolve_fetch_redirect_url(current_url: &str, location: &str) -> Option<String> {
    let cleaned = clean_text(location, 2200);
    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        return Some(cleaned);
    }
    let scheme = fetch_url_scheme(current_url);
    let origin = fetch_url_origin(current_url);
    if scheme.is_empty() || origin.is_empty() {
        return None;
    }
    if cleaned.starts_with("//") {
        return Some(format!("{scheme}:{cleaned}"));
    }
    if cleaned.starts_with('/') {
        return Some(format!("{origin}{cleaned}"));
    }
    let base = clean_text(current_url, 2200);
    let base_no_fragment = base.split('#').next().unwrap_or(base.as_str());
    let base_no_query = base_no_fragment
        .split('?')
        .next()
        .unwrap_or(base_no_fragment);
    let base_dir = if base_no_query.ends_with('/') {
        base_no_query.to_string()
    } else if let Some((left, _)) = base_no_query.rsplit_once('/') {
        format!("{left}/")
    } else {
        format!("{origin}/")
    };
    Some(format!("{base_dir}{cleaned}"))
}

fn fetch_host_is_local_name(host: &str) -> bool {
    let lowered = clean_text(host, 220).to_ascii_lowercase();
    lowered == "localhost" || lowered.ends_with(".localhost")
}

fn fetch_host_ip_literal(host: &str) -> Option<std::net::IpAddr> {
    clean_text(host, 220).trim_matches(['[', ']']).parse().ok()
}

fn ipv4_is_rfc2544_benchmark_range(addr: std::net::Ipv4Addr) -> bool {
    let octets = addr.octets();
    octets[0] == 198 && matches!(octets[1], 18 | 19)
}

fn ipv4_is_restricted_target(
    addr: std::net::Ipv4Addr,
    allow_rfc2544_benchmark_range: bool,
) -> bool {
    let octets = addr.octets();
    addr.is_private()
        || addr.is_loopback()
        || addr.is_link_local()
        || addr.is_multicast()
        || addr.is_unspecified()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (!allow_rfc2544_benchmark_range && ipv4_is_rfc2544_benchmark_range(addr))
}

fn ipv6_is_restricted_target(
    addr: std::net::Ipv6Addr,
    allow_rfc2544_benchmark_range: bool,
) -> bool {
    if let Some(mapped) = addr.to_ipv4_mapped() {
        return ipv4_is_restricted_target(mapped, allow_rfc2544_benchmark_range);
    }
    addr.is_loopback()
        || addr.is_unspecified()
        || addr.is_unique_local()
        || addr.is_unicast_link_local()
        || addr.is_multicast()
}

fn ip_addr_is_restricted_target(
    addr: std::net::IpAddr,
    allow_rfc2544_benchmark_range: bool,
) -> bool {
    match addr {
        std::net::IpAddr::V4(v4) => {
            ipv4_is_restricted_target(v4, allow_rfc2544_benchmark_range)
        }
        std::net::IpAddr::V6(v6) => {
            ipv6_is_restricted_target(v6, allow_rfc2544_benchmark_range)
        }
    }
}

fn resolve_fetch_host_ip_addrs(raw_url: &str) -> Vec<std::net::IpAddr> {
    let host = fetch_url_host(raw_url);
    if host.is_empty() || fetch_host_ip_literal(&host).is_some() {
        return Vec::new();
    }
    let port = if fetch_url_scheme(raw_url) == "https" {
        443
    } else {
        80
    };
    std::net::ToSocketAddrs::to_socket_addrs(&(host.as_str(), port))
        .map(|iter| {
            iter.map(|addr| addr.ip())
                .fold(Vec::<std::net::IpAddr>::new(), |mut acc, ip| {
                    if !acc.iter().any(|existing| existing == &ip) && acc.len() < 8 {
                        acc.push(ip);
                    }
                    acc
                })
        })
        .unwrap_or_default()
}

fn evaluate_fetch_ssrf_guard(
    raw_url: &str,
    allow_rfc2544_benchmark_range: bool,
    resolved_override: Option<&[std::net::IpAddr]>,
) -> Value {
    let cleaned = clean_text(raw_url, 2200);
    let scheme = fetch_url_scheme(&cleaned);
    let host = fetch_url_host(&cleaned);
    let invalid = cleaned.is_empty()
        || host.is_empty()
        || !matches!(scheme.as_str(), "http" | "https")
        || fetch_url_origin(&cleaned).is_empty();
    if invalid {
        return json!({
            "ok": false,
            "error": "invalid_fetch_url",
            "url": cleaned,
            "host": host,
            "resolved_ip_addrs": []
        });
    }
    if fetch_host_is_local_name(&host) {
        return json!({
            "ok": false,
            "error": "blocked_hostname",
            "url": cleaned,
            "host": host,
            "resolved_ip_addrs": []
        });
    }
    if let Some(literal) = fetch_host_ip_literal(&host) {
        if ip_addr_is_restricted_target(literal, allow_rfc2544_benchmark_range) {
            return json!({
                "ok": false,
                "error": "blocked_private_network_target",
                "url": cleaned,
                "host": host,
                "resolved_ip_addrs": [literal.to_string()]
            });
        }
        return json!({
            "ok": true,
            "url": cleaned,
            "host": host,
            "resolved_ip_addrs": [literal.to_string()]
        });
    }
    let resolved = resolved_override
        .map(|rows| rows.to_vec())
        .unwrap_or_else(|| resolve_fetch_host_ip_addrs(&cleaned));
    if resolved
        .iter()
        .copied()
        .any(|ip| ip_addr_is_restricted_target(ip, allow_rfc2544_benchmark_range))
    {
        return json!({
            "ok": false,
            "error": "blocked_private_network_target",
            "url": cleaned,
            "host": host,
            "resolved_ip_addrs": resolved.iter().map(|row| row.to_string()).collect::<Vec<_>>()
        });
    }
    json!({
        "ok": true,
        "url": cleaned,
        "host": host,
        "resolved_ip_addrs": resolved.iter().map(|row| row.to_string()).collect::<Vec<_>>()
    })
}

fn curl_fetch_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        suffix
    ))
}

fn extract_last_header_block(raw_headers: &str) -> String {
    raw_headers
        .replace("\r\n", "\n")
        .split("\n\n")
        .filter(|block| block.trim_start().starts_with("HTTP/"))
        .last()
        .map(|row| row.trim().to_string())
        .unwrap_or_default()
}

fn header_value_from_block(raw_headers: &str, header_name: &str) -> String {
    let lowered_name = header_name.to_ascii_lowercase();
    extract_last_header_block(raw_headers)
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            if name.trim().eq_ignore_ascii_case(&lowered_name) {
                Some(clean_text(value, 220))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn extract_markdown_fetch_content(
    raw_body: &str,
    extract_mode: &str,
    max_chars: usize,
) -> (String, Option<String>, bool) {
    let normalized = normalize_block_text(&strip_invisible_unicode(raw_body));
    let title = normalized
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .filter(|line| !line.is_empty());
    let rendered = if extract_mode == "markdown" {
        normalized
    } else {
        markdown_to_text_document(&normalized)
    };
    let (text, truncated) = truncate_chars(&rendered, max_chars);
    (text, title, truncated)
}

fn extract_fetch_content_with_extractor(
    raw_body: &str,
    content_type: &str,
    extract_mode: &str,
    max_chars: usize,
) -> (String, Option<String>, bool, String) {
    if fetch_response_is_markdown(content_type) {
        let (content, title, truncated) =
            extract_markdown_fetch_content(raw_body, extract_mode, max_chars);
        return (content, title, truncated, "cf-markdown".to_string());
    }
    let (content, title, truncated) =
        extract_fetch_content(raw_body, content_type, extract_mode, max_chars);
    let extractor = if looks_like_html_document(raw_body, content_type) {
        "readability"
    } else {
        "raw"
    };
    (content, title, truncated, extractor.to_string())
}

fn run_curl_fetch_once(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let header_path = curl_fetch_temp_path("web-conduit-fetch-header", ".tmp");
    let body_path = curl_fetch_temp_path("web-conduit-fetch-body", ".tmp");
    let output = Command::new("curl")
        .arg("-sS")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--max-redirs")
        .arg("0")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg(format!("Accept-Language: {DEFAULT_ACCEPT_LANGUAGE}"))
        .arg("-H")
        .arg(format!("Accept: {FETCH_MARKDOWN_ACCEPT_HEADER}"))
        .arg("-e")
        .arg(DEFAULT_REFERER)
        .arg("-D")
        .arg(&header_path)
        .arg("-o")
        .arg(&body_path)
        .arg("-w")
        .arg("__STATUS__:%{http_code}\n__CTYPE__:%{content_type}\n__EFFECTIVE_URL__:%{url_effective}")
        .arg(url)
        .output();

    match output {
        Ok(run) => {
            let header_raw = fs::read_to_string(&header_path).unwrap_or_default();
            let body_raw = fs::read(&body_path).unwrap_or_default();
            let _ = fs::remove_file(&header_path);
            let _ = fs::remove_file(&body_path);
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let mut status_code = 0i64;
            let mut content_type = String::new();
            let mut effective_url = clean_text(url, 2200);
            for line in stdout.lines() {
                if let Some(value) = line.strip_prefix("__STATUS__:") {
                    status_code = clean_text(value, 12).parse::<i64>().unwrap_or(0);
                } else if let Some(value) = line.strip_prefix("__CTYPE__:") {
                    content_type = normalize_fetch_content_type(value);
                } else if let Some(value) = line.strip_prefix("__EFFECTIVE_URL__:") {
                    let candidate = clean_text(value, 2200);
                    if !candidate.is_empty() {
                        effective_url = candidate;
                    }
                }
            }
            let body = clip_bytes(&String::from_utf8_lossy(&body_raw), max_response_bytes.max(256));
            let location = header_value_from_block(&header_raw, "location");
            let markdown_tokens = header_value_from_block(&header_raw, "x-markdown-tokens");
            let status_ok = (200..300).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260),
                "effective_url": effective_url,
                "location": if location.is_empty() { Value::Null } else { Value::String(location) },
                "x_markdown_tokens": if markdown_tokens.is_empty() { Value::Null } else { Value::String(markdown_tokens) },
                "accept_header": FETCH_MARKDOWN_ACCEPT_HEADER
            })
        }
        Err(err) => {
            let _ = fs::remove_file(&header_path);
            let _ = fs::remove_file(&body_path);
            json!({
                "ok": false,
                "status_code": 0,
                "content_type": "",
                "body": "",
                "stderr": format!("curl_spawn_failed:{err}"),
                "user_agent": clean_text(user_agent, 260),
                "effective_url": clean_text(url, 2200),
                "accept_header": FETCH_MARKDOWN_ACCEPT_HEADER
            })
        }
    }
}
