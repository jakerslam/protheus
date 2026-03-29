
#[test]
fn core_shortcut_routes_parse_export_to_parse_plane() {
    let route = resolve_core_shortcuts(
        "parse",
        &[
            "export".to_string(),
            "core/local/state/ops/parse_plane/flatten/latest.json".to_string(),
            "core/local/artifacts/parse/export.json".to_string(),
            "--format=json".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://parse-plane");
    assert_eq!(
        route.args,
        vec![
            "export",
            "--from-path=core/local/state/ops/parse_plane/flatten/latest.json",
            "--output-path=core/local/artifacts/parse/export.json",
            "--format=json"
        ]
    );
}

#[test]
fn core_shortcut_routes_parse_visualize_to_parse_plane() {
    let route = resolve_core_shortcuts(
        "parse",
        &[
            "visualize".to_string(),
            "core/local/state/ops/parse_plane/parse_doc/latest.json".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://parse-plane");
    assert_eq!(
        route.args,
        vec![
            "visualize",
            "--from-path=core/local/state/ops/parse_plane/parse_doc/latest.json",
            "--strict=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_mcp_status_to_mcp_plane() {
    let route = resolve_core_shortcuts("mcp", &[]).expect("route");
    assert_eq!(route.script_rel, "core://mcp-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_mcp_expose_to_mcp_plane() {
    let route = resolve_core_shortcuts(
        "mcp",
        &[
            "expose".to_string(),
            "research-agent".to_string(),
            "--tools=fetch,extract".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://mcp-plane");
    assert_eq!(
        route.args,
        vec!["expose", "--agent=research-agent", "--tools=fetch,extract"]
    );
}

#[test]
fn core_shortcut_routes_flow_compile_to_flow_plane() {
    let route = resolve_core_shortcuts(
        "flow",
        &[
            "compile".to_string(),
            "core/local/artifacts/flow/canvas.json".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://flow-plane");
    assert_eq!(
        route.args,
        vec![
            "compile",
            "--canvas-path=core/local/artifacts/flow/canvas.json",
            "--strict=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_flow_run_to_flow_plane() {
    let route = resolve_core_shortcuts(
        "flow",
        &[
            "run".to_string(),
            "--run-id=batch29-flow".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://flow-plane");
    assert_eq!(
        route.args,
        vec![
            "playground",
            "--op=play",
            "--run-id=batch29-flow",
            "--strict=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_flow_install_to_flow_plane() {
    let route = resolve_core_shortcuts(
        "flow",
        &[
            "install".to_string(),
            "--manifest=planes/contracts/flow/template_pack_manifest_v1.json".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://flow-plane");
    assert_eq!(
        route.args,
        vec![
            "install",
            "--manifest=planes/contracts/flow/template_pack_manifest_v1.json",
            "--strict=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_blobs_to_binary_blob_runtime() {
    let route = resolve_core_shortcuts("blobs", &["migrate".to_string(), "--apply=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://binary-blob-runtime");
    assert_eq!(route.args, vec!["migrate", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_directives_migrate_to_directive_kernel() {
    let route = resolve_core_shortcuts(
        "directives",
        &["migrate".to_string(), "--apply=1".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://directive-kernel");
    assert_eq!(route.args, vec!["migrate", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_directives_dashboard_to_directive_kernel() {
    let route = resolve_core_shortcuts("directives", &["dashboard".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://directive-kernel");
    assert_eq!(route.args, vec!["dashboard"]);
}

#[test]
fn core_shortcut_routes_prime_sign_to_directive_kernel() {
    let route = resolve_core_shortcuts(
        "prime",
        &["sign".to_string(), "--directive=Always safe".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://directive-kernel");
    assert_eq!(route.args, vec!["prime-sign", "--directive=Always safe"]);
}

#[test]
fn core_shortcut_routes_organism_ignite_to_organism_layer() {
    let route =
        resolve_core_shortcuts("organism", &["ignite".to_string(), "--apply=1".to_string()])
            .expect("route");
    assert_eq!(route.script_rel, "core://organism-layer");
    assert_eq!(route.args, vec!["ignite", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_rsi_ignite_to_rsi_ignition() {
    let route = resolve_core_shortcuts("rsi", &["ignite".to_string(), "--apply=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://rsi-ignition");
    assert_eq!(route.args, vec!["ignite", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_veto_to_directive_kernel() {
    let route =
        resolve_core_shortcuts("veto", &["--action=rsi_proposal".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://directive-kernel");
    assert_eq!(
        route.args,
        vec![
            "compliance-check",
            "--action=veto",
            "--allow=0",
            "--action=rsi_proposal"
        ]
    );
}

#[test]
fn core_shortcut_routes_model_buy_credits_to_intelligence_nexus() {
    let route = resolve_core_shortcuts(
        "model",
        &[
            "buy".to_string(),
            "credits".to_string(),
            "--provider=openai".to_string(),
            "--amount=250".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://intelligence-nexus");
    assert_eq!(
        route.args,
        vec!["buy-credits", "--provider=openai", "--amount=250"]
    );
}

#[test]
fn core_shortcut_routes_compute_share_to_network_compute_proof() {
    let route = resolve_core_shortcuts("compute", &["share".to_string(), "--gpu=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://p2p-gossip-seed");
    assert_eq!(route.args, vec!["compute-proof", "--share=1", "--gpu=1"]);
}

#[test]
fn core_shortcut_routes_skills_enable_to_assimilation_controller() {
    let route = resolve_core_shortcuts(
        "skills",
        &[
            "enable".to_string(),
            "perplexity-mode".to_string(),
            "--apply=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://assimilation-controller");
    assert_eq!(
        route.args,
        vec!["skills-enable", "perplexity-mode", "--apply=1"]
    );
}

#[test]
fn core_shortcut_routes_skills_dashboard_to_skills_plane() {
    let route = resolve_core_shortcuts("skills", &["dashboard".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://skills-plane");
    assert_eq!(route.args, vec!["dashboard"]);
}

#[test]
fn core_shortcut_routes_skills_spawn_to_assimilation_controller() {
    let route = resolve_core_shortcuts(
        "skills",
        &[
            "spawn".to_string(),
            "--task=launch".to_string(),
            "--roles=researcher,executor".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://assimilation-controller");
    assert_eq!(
        route.args,
        vec![
            "skills-spawn-subagents",
            "--task=launch",
            "--roles=researcher,executor"
        ]
    );
}

#[test]
fn core_shortcut_routes_skills_computer_use_to_assimilation_controller() {
    let route = resolve_core_shortcuts(
        "skills",
        &[
            "computer-use".to_string(),
            "--action=open browser".to_string(),
            "--target=desktop".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://assimilation-controller");
    assert_eq!(
        route.args,
        vec![
            "skills-computer-use",
            "--action=open browser",
            "--target=desktop"
        ]
    );
}

#[test]
fn core_shortcut_routes_skills_status_to_skills_plane() {
    let route = resolve_core_shortcuts("skills", &[]).expect("route");
    assert_eq!(route.script_rel, "core://skills-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_skill_create_to_skills_plane() {
    let route = resolve_core_shortcuts(
        "skill",
        &[
            "create".to_string(),
            "weekly".to_string(),
            "growth".to_string(),
            "report".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://skills-plane");
    assert_eq!(route.args, vec!["create", "--name=weekly growth report"]);
}

#[test]
fn core_shortcut_routes_skill_run_to_skills_plane() {
    let route = resolve_core_shortcuts(
        "skill",
        &[
            "run".to_string(),
            "--skill=researcher".to_string(),
            "--input=check".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://skills-plane");
    assert_eq!(
        route.args,
        vec!["run", "--skill=researcher", "--input=check"]
    );
}

#[test]
fn core_shortcut_routes_skill_list_to_skills_plane() {
    let route = resolve_core_shortcuts("skill", &["list".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://skills-plane");
    assert_eq!(route.args, vec!["list"]);
}

#[test]
fn core_shortcut_routes_binary_vuln_to_core_lane() {
    let route = resolve_core_shortcuts(
        "binary-vuln",
        &["scan".to_string(), "--input=a.bin".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://binary-vuln-plane");
    assert_eq!(route.args, vec!["scan", "--input=a.bin"]);
}

#[test]
fn core_shortcut_routes_business_to_business_plane() {
    let route = resolve_core_shortcuts("business", &[]).expect("route");
    assert_eq!(route.script_rel, "core://business-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_canyon_to_canyon_plane() {
    let route = resolve_core_shortcuts("canyon", &[]).expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_canyon_benchmark_gate_to_canyon_plane() {
    let route = resolve_core_shortcuts(
        "canyon",
        &[
            "benchmark-gate".to_string(),
            "--op=run".to_string(),
            "--milestone=day90".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(
        route.args,
        vec!["benchmark-gate", "--op=run", "--milestone=day90"]
    );
}

#[test]
fn core_shortcut_routes_init_to_canyon_ecosystem_init() {
    let route = resolve_core_shortcuts(
        "init",
        &[
            "starter-web".to_string(),
            "--target-dir=/tmp/demo".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(
        route.args,
        vec![
            "ecosystem",
            "--op=init",
            "--template=starter-web",
            "--target-dir=/tmp/demo"
        ]
    );
}

#[test]
fn core_shortcut_routes_init_pure_to_canyon_ecosystem_init() {
    let route = resolve_core_shortcuts(
        "init",
        &[
            "--pure".to_string(),
            "--target-dir=/tmp/pure-demo".to_string(),
            "--dry-run=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(
        route.args,
        vec![
            "ecosystem",
            "--op=init",
            "--workspace-mode=pure",
            "--pure",
            "--target-dir=/tmp/pure-demo",
            "--dry-run=1"
        ]
    );
}

