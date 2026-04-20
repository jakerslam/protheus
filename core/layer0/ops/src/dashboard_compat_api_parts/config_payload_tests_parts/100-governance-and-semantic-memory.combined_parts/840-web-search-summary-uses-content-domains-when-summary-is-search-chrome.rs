fn web_search_summary_uses_content_domains_when_summary_is_search_chrome() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "latest technology news today",
            "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
            "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
            "content": "latest technology news today at DuckDuckGo All Regions Any Time Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology: Latest Tech News Articles Today | AP News apnews.com/technology Don't miss an update on the latest tech news. The Latest News in Technology | PCMag www.pcmag.com/news Get the latest technology news and in-depth analysis."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("key web findings"),
        "unexpected summary: {summary}"
    );
    assert!(lowered.contains("reuters.com"));
    assert!(!lowered.contains("couldn't extract usable findings"));
    assert!(!lowered.contains("search response came from"));
}

#[test]
