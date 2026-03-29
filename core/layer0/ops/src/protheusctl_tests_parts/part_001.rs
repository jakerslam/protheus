
#[test]
fn core_shortcut_routes_enterprise_moat_license_to_core_lane() {
    let route = resolve_core_shortcuts(
        "enterprise",
        &[
            "moat".to_string(),
            "license".to_string(),
            "--primitives=conduit,binary_blob".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(
        route.args,
        vec!["moat-license", "--primitives=conduit,binary_blob"]
    );
}

#[test]
fn core_shortcut_routes_genesis_truth_gate_to_core_lane() {
    let route = resolve_core_shortcuts(
        "genesis",
        &[
            "truth-gate".to_string(),
            "--regression-pass=1".to_string(),
            "--dod-pass=1".to_string(),
            "--verify-pass=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(
        route.args,
        vec![
            "genesis-truth-gate",
            "--regression-pass=1",
            "--dod-pass=1",
            "--verify-pass=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_moat_launch_to_core_lane() {
    let route = resolve_core_shortcuts(
        "moat",
        &["launch-sim".to_string(), "--events=12000".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["moat-launch-sim", "--events=12000"]);
}

#[test]
fn core_shortcut_routes_seed_deploy_viral_to_seed_protocol() {
    let route = resolve_core_shortcuts(
        "seed",
        &[
            "deploy".to_string(),
            "viral".to_string(),
            "--targets=node-a,node-b".to_string(),
            "--apply=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://seed-protocol");
    assert_eq!(
        route.args,
        vec![
            "deploy",
            "--profile=viral",
            "--targets=node-a,node-b",
            "--apply=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_seed_ignite_viral_to_seed_protocol() {
    let route = resolve_core_shortcuts(
        "seed",
        &[
            "ignite".to_string(),
            "viral".to_string(),
            "--replication-cap=16".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://seed-protocol");
    assert_eq!(
        route.args,
        vec!["deploy", "--profile=viral", "--replication-cap=16"]
    );
}

#[test]
fn core_shortcut_routes_seed_defaults_to_status() {
    let route = resolve_core_shortcuts("seed", &[]).expect("route");
    assert_eq!(route.script_rel, "core://seed-protocol");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_keys_open_to_intelligence_nexus() {
    let route = resolve_core_shortcuts("keys", &["open".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://intelligence-nexus");
    assert_eq!(route.args, vec!["open"]);
}

#[test]
fn core_shortcut_routes_keys_add_alias_to_add_key() {
    let route = resolve_core_shortcuts(
        "keys",
        &["add".to_string(), "--provider=openai".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://intelligence-nexus");
    assert_eq!(route.args, vec!["add-key", "--provider=openai"]);
}

#[test]
fn core_shortcut_routes_keys_rotate_alias_to_rotate_key() {
    let route = resolve_core_shortcuts(
        "keys",
        &["rotate".to_string(), "--provider=openai".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://intelligence-nexus");
    assert_eq!(route.args, vec!["rotate-key", "--provider=openai"]);
}

#[test]
fn core_shortcut_routes_keys_revoke_alias_to_revoke_key() {
    let route = resolve_core_shortcuts(
        "keys",
        &["revoke".to_string(), "--provider=openai".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://intelligence-nexus");
    assert_eq!(route.args, vec!["revoke-key", "--provider=openai"]);
}

#[test]
fn core_shortcut_routes_graph_pagerank_to_graph_toolkit() {
    let route = resolve_core_shortcuts(
        "graph",
        &[
            "pagerank".to_string(),
            "--dataset=memory-vault".to_string(),
            "--iterations=32".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://graph-toolkit");
    assert_eq!(
        route.args,
        vec!["pagerank", "--dataset=memory-vault", "--iterations=32"]
    );
}

#[test]
fn core_shortcut_routes_graph_defaults_to_status() {
    let route = resolve_core_shortcuts("graph", &[]).expect("route");
    assert_eq!(route.script_rel, "core://graph-toolkit");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_research_stealth_flags_to_core_plane_fetch() {
    let route = resolve_core_shortcuts(
        "research",
        &[
            "--stealth".to_string(),
            "--url=https://example.com".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["fetch", "--url=https://example.com", "--mode=stealth"]
    );
}

#[test]
fn core_shortcut_routes_research_default_fetch_mode_to_auto() {
    let route = resolve_core_shortcuts(
        "research",
        &["fetch".to_string(), "--url=https://example.com".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["fetch", "--url=https://example.com", "--mode=auto"]
    );
}

#[test]
fn core_shortcut_routes_research_firmware_to_binary_vuln_lane() {
    let route = resolve_core_shortcuts(
        "research",
        &[
            "--firmware=fw.bin".to_string(),
            "--format=jsonl".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://binary-vuln-plane");
    assert_eq!(
        route.args,
        vec![
            "scan",
            "--dx-source=research-firmware",
            "--input=fw.bin",
            "--format=jsonl",
            "--strict=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_top_level_crawl_goal_to_research_plane() {
    let route = resolve_core_shortcuts(
        "crawl",
        &[
            "memory".to_string(),
            "coherence".to_string(),
            "--max-pages=4".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["goal-crawl", "--goal=memory coherence", "--max-pages=4"]
    );
}

#[test]
fn core_shortcut_routes_top_level_map_to_research_plane() {
    let route =
        resolve_core_shortcuts("map", &["example.com".to_string(), "--depth=3".to_string()])
            .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["map-site", "--domain=example.com", "--depth=3"]
    );
}

#[test]
fn core_shortcut_routes_top_level_monitor_to_research_plane() {
    let route = resolve_core_shortcuts(
        "monitor",
        &[
            "https://example.com/feed".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["monitor", "--url=https://example.com/feed", "--strict=1"]
    );
}

#[test]
fn core_shortcut_routes_assimilate_scrapy_core_to_research_plane() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["scrape://scrapy-core".to_string(), "--strict=1".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(route.args, vec!["template-governance", "--strict=1"]);
}

#[test]
fn core_shortcut_routes_assimilate_firecrawl_core_to_research_plane() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &[
            "scrape://firecrawl-core".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://research-plane");
    assert_eq!(
        route.args,
        vec!["firecrawl-template-governance", "--strict=1"]
    );
}

#[test]
fn core_shortcut_routes_assimilate_doc2dict_core_to_parse_plane() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &[
            "parse://doc2dict-core".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://parse-plane");
    assert_eq!(route.args, vec!["template-governance", "--strict=1"]);
}

#[test]
fn core_shortcut_routes_assimilate_llamaindex_to_llamaindex_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &[
            "llamaindex".to_string(),
            "--payload-base64=e30=".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://llamaindex-bridge");
    assert_eq!(
        route.args,
        vec!["register-connector", "--payload-base64=e30="]
    );
}

#[test]
fn core_shortcut_routes_assimilate_google_adk_to_google_adk_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &[
            "google-adk".to_string(),
            "--payload-base64=e30=".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://google-adk-bridge");
    assert_eq!(
        route.args,
        vec!["register-tool-manifest", "--payload-base64=e30="]
    );
}

#[test]
fn core_shortcut_routes_assimilate_camel_to_camel_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["camel".to_string(), "--payload-base64=e30=".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://camel-bridge");
    assert_eq!(route.args, vec!["import-dataset", "--payload-base64=e30="]);
}

#[test]
fn core_shortcut_routes_assimilate_haystack_to_haystack_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["haystack".to_string(), "--payload-base64=e30=".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://haystack-bridge");
    assert_eq!(
        route.args,
        vec!["register-pipeline", "--payload-base64=e30="]
    );
}

#[test]
fn core_shortcut_routes_assimilate_langchain_to_langchain_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["langchain".to_string(), "--payload-base64=e30=".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://langchain-bridge");
    assert_eq!(
        route.args,
        vec!["import-integration", "--payload-base64=e30="]
    );
}

#[test]
fn core_shortcut_routes_assimilate_pydantic_ai_to_pydantic_ai_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &[
            "pydantic-ai".to_string(),
            "--payload-base64=e30=".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://pydantic-ai-bridge");
    assert_eq!(route.args, vec!["register-agent", "--payload-base64=e30="]);
}

#[test]
fn core_shortcut_routes_assimilate_dspy_to_dspy_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["dspy".to_string(), "--payload-base64=e30=".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://dspy-bridge");
    assert_eq!(
        route.args,
        vec!["import-integration", "--payload-base64=e30="]
    );
}

#[test]
fn core_shortcut_routes_assimilate_mastra_to_mastra_bridge() {
    let route = resolve_core_shortcuts(
        "assimilate",
        &["mastra".to_string(), "--payload-base64=e30=".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://mastra-bridge");
    assert_eq!(route.args, vec!["register-graph", "--payload-base64=e30="]);
}

#[test]
fn core_shortcut_routes_parse_doc_to_parse_plane() {
    let route = resolve_core_shortcuts(
        "parse",
        &[
            "doc".to_string(),
            "fixtures/report.html".to_string(),
            "--mapping=default".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://parse-plane");
    assert_eq!(
        route.args,
        vec![
            "parse-doc",
            "--file=fixtures/report.html",
            "--mapping=default"
        ]
    );
}

