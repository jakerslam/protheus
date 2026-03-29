
#[test]
fn core_shortcut_routes_init_tiny_max_forces_pure_workspace_mode() {
    let route = resolve_core_shortcuts(
        "init",
        &[
            "--tiny-max=1".to_string(),
            "--target-dir=/tmp/tiny-max-demo".to_string(),
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
            "--pure=1",
            "--tiny-max=1",
            "--target-dir=/tmp/tiny-max-demo",
            "--dry-run=1"
        ]
    );
}

#[test]
fn core_shortcut_routes_init_help_to_canyon_help() {
    let route = resolve_core_shortcuts("init", &["--help".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(route.args, vec!["help"]);
}

#[test]
fn core_shortcut_routes_marketplace_publish_to_canyon_ecosystem() {
    let route = resolve_core_shortcuts(
        "marketplace",
        &[
            "publish".to_string(),
            "--hand-id=starter".to_string(),
            "--receipt-file=/tmp/r.json".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://canyon-plane");
    assert_eq!(
        route.args,
        vec![
            "ecosystem",
            "--op=marketplace-publish",
            "--hand-id=starter",
            "--receipt-file=/tmp/r.json"
        ]
    );
}

#[test]
fn core_shortcut_routes_replay_to_enterprise_hardening() {
    let route =
        resolve_core_shortcuts("replay", &["--receipt-hash=abc123".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["replay", "--receipt-hash=abc123"]);
}

#[test]
fn core_shortcut_routes_ai_to_enterprise_hardening() {
    let route = resolve_core_shortcuts("ai", &["--model=ollama/llama3.2:latest".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["ai", "--model=ollama/llama3.2:latest"]);
}

#[test]
fn core_shortcut_routes_chaos_to_enterprise_hardening() {
    let route = resolve_core_shortcuts("chaos", &["run".to_string(), "--agents=16".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["chaos-run", "--agents=16"]);
}

#[test]
fn core_shortcut_routes_chaos_isolate_to_enterprise_hardening() {
    let route = resolve_core_shortcuts("chaos", &["isolate".to_string(), "--agents=4".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(
        route.args,
        vec!["chaos-run", "--suite=isolate", "--agents=4"]
    );
}

#[test]
fn core_shortcut_routes_assistant_to_enterprise_hardening() {
    let route =
        resolve_core_shortcuts("assistant", &["--topic=onboarding".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["assistant-mode", "--topic=onboarding"]);
}

#[test]
fn core_shortcut_routes_adaptive_default_to_adaptive_lane_status() {
    let route = resolve_core_shortcuts("adaptive", &[]).expect("route");
    assert_eq!(route.script_rel, "core://adaptive-intelligence");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_adaptive_propose_to_adaptive_lane() {
    let route = resolve_core_shortcuts(
        "adaptive-intelligence",
        &[
            "propose".to_string(),
            "--prompt=refactor scheduler".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://adaptive-intelligence");
    assert_eq!(route.args, vec!["propose", "--prompt=refactor scheduler"]);
}

#[test]
fn core_shortcut_routes_gov_alias_to_government_plane() {
    let route = resolve_core_shortcuts("gov", &["classification".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://government-plane");
    assert_eq!(route.args, vec!["classification"]);
}

#[test]
fn core_shortcut_routes_bank_alias_to_finance_plane() {
    let route = resolve_core_shortcuts("bank", &["transaction".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://finance-plane");
    assert_eq!(route.args, vec!["transaction"]);
}

#[test]
fn core_shortcut_routes_hospital_alias_to_healthcare_plane() {
    let route = resolve_core_shortcuts("hospital", &["cds".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://healthcare-plane");
    assert_eq!(route.args, vec!["cds"]);
}

#[test]
fn core_shortcut_routes_vertical_to_vertical_plane() {
    let route = resolve_core_shortcuts("vertical", &[]).expect("route");
    assert_eq!(route.script_rel, "core://vertical-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_nexus_to_nexus_plane() {
    let route = resolve_core_shortcuts("nexus", &[]).expect("route");
    assert_eq!(route.script_rel, "core://nexus-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_scan_binary_to_binary_vuln_lane() {
    let route = resolve_core_shortcuts("scan", &["binary".to_string(), "firmware.bin".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://binary-vuln-plane");
    assert_eq!(
        route.args,
        vec!["scan", "--dx-source=scan-binary", "--input=firmware.bin"]
    );
}

#[test]
fn core_shortcut_routes_shadow_discover_to_hermes_lane() {
    let route = resolve_core_shortcuts(
        "shadow",
        &["discover".to_string(), "--shadow=alpha".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://hermes-plane");
    assert_eq!(route.args, vec!["discover", "--shadow=alpha"]);
}

#[test]
fn core_shortcut_routes_top_to_hermes_cockpit() {
    let route = resolve_core_shortcuts("top", &[]).expect("route");
    assert_eq!(route.script_rel, "core://hermes-plane");
    assert_eq!(route.args, vec!["cockpit"]);
}

#[test]
fn core_shortcut_routes_status_dashboard_to_hermes_cockpit() {
    let route = resolve_core_shortcuts("status", &["--dashboard".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://hermes-plane");
    assert_eq!(route.args, vec!["cockpit"]);
}

#[test]
fn core_shortcut_routes_browser_to_vbrowser_plane() {
    let route = resolve_core_shortcuts(
        "browser",
        &["start".to_string(), "--url=https://example.com".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://vbrowser-plane");
    assert_eq!(
        route.args,
        vec!["session-start", "--url=https://example.com"]
    );
}

#[test]
fn core_shortcut_routes_agency_create_to_agency_plane() {
    let route = resolve_core_shortcuts(
        "agency",
        &[
            "create".to_string(),
            "--template=frontend-wizard".to_string(),
            "--name=ux-shadow".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://agency-plane");
    assert_eq!(
        route.args,
        vec![
            "create-shadow",
            "--template=frontend-wizard",
            "--name=ux-shadow"
        ]
    );
}

#[test]
fn core_shortcut_routes_shadow_browser_flag_to_vbrowser_plane() {
    let route = resolve_core_shortcuts(
        "shadow",
        &[
            "--browser".to_string(),
            "--session-id=live".to_string(),
            "--url=https://example.com".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://vbrowser-plane");
    assert_eq!(
        route.args,
        vec![
            "session-start",
            "--shadow=default-shadow",
            "--session-id=live",
            "--url=https://example.com"
        ]
    );
}

#[test]
fn core_shortcut_routes_shadow_delegate_to_hermes_plane() {
    let route = resolve_core_shortcuts(
        "shadow",
        &[
            "delegate".to_string(),
            "--task=triage".to_string(),
            "--parent=alpha".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://hermes-plane");
    assert_eq!(
        route.args,
        vec!["delegate", "--task=triage", "--parent=alpha"]
    );
}

#[test]
fn core_shortcut_routes_shadow_continuity_to_hermes_plane() {
    let route = resolve_core_shortcuts(
        "shadow",
        &[
            "continuity".to_string(),
            "--op=status".to_string(),
            "--session-id=s1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://hermes-plane");
    assert_eq!(
        route.args,
        vec!["continuity", "--op=status", "--session-id=s1"]
    );
}

#[test]
fn core_shortcut_routes_shadow_create_template_to_agency_plane() {
    let route = resolve_core_shortcuts(
        "shadow",
        &[
            "create".to_string(),
            "--template=security-engineer".to_string(),
            "--name=sec-shadow".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://agency-plane");
    assert_eq!(
        route.args,
        vec![
            "create-shadow",
            "--template=security-engineer",
            "--name=sec-shadow"
        ]
    );
}

#[test]
fn core_shortcut_routes_team_dashboard_to_collab_plane() {
    let route =
        resolve_core_shortcuts("team", &["dashboard".to_string(), "--team=ops".to_string()])
            .expect("route");
    assert_eq!(route.script_rel, "core://collab-plane");
    assert_eq!(route.args, vec!["dashboard", "--team=ops"]);
}

#[test]
fn core_shortcut_routes_team_schedule_to_collab_plane() {
    let route = resolve_core_shortcuts(
        "team",
        &[
            "schedule".to_string(),
            "--op=kickoff".to_string(),
            "--team=ops".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://collab-plane");
    assert_eq!(route.args, vec!["schedule", "--op=kickoff", "--team=ops"]);
}

#[test]
fn core_shortcut_routes_company_budget_to_company_plane() {
    let route = resolve_core_shortcuts(
        "company",
        &[
            "budget".to_string(),
            "--agent=alpha".to_string(),
            "--tokens=100".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(
        route.args,
        vec!["budget-enforce", "--agent=alpha", "--tokens=100"]
    );
}

#[test]
fn core_shortcut_routes_company_ticket_to_company_plane() {
    let route = resolve_core_shortcuts(
        "company",
        &[
            "ticket".to_string(),
            "--op=create".to_string(),
            "--title=Fix ingestion".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(
        route.args,
        vec!["ticket", "--op=create", "--title=Fix ingestion"]
    );
}

#[test]
fn core_shortcut_routes_company_heartbeat_to_company_plane() {
    let route = resolve_core_shortcuts(
        "company",
        &[
            "heartbeat".to_string(),
            "--op=tick".to_string(),
            "--team=ops".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(route.args, vec!["heartbeat", "--op=tick", "--team=ops"]);
}

#[test]
fn core_shortcut_routes_top_level_ticket_to_company_plane() {
    let route = resolve_core_shortcuts(
        "ticket",
        &[
            "--op=create".to_string(),
            "--title=Stability hotfix".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(
        route.args,
        vec!["ticket", "--op=create", "--title=Stability hotfix"]
    );
}

#[test]
fn core_shortcut_routes_top_level_heartbeat_to_company_plane() {
    let route = resolve_core_shortcuts(
        "heartbeat",
        &["--op=tick".to_string(), "--team=platform".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://company-plane");
    assert_eq!(
        route.args,
        vec!["heartbeat", "--op=tick", "--team=platform"]
    );
}

#[test]
fn core_shortcut_routes_substrate_capture_to_substrate_plane() {
    let route = resolve_core_shortcuts(
        "substrate",
        &[
            "capture".to_string(),
            "--adapter=wifi-csi-esp32".to_string(),
            "--strict=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://substrate-plane");
    assert_eq!(
        route.args,
        vec!["csi-capture", "--adapter=wifi-csi-esp32", "--strict=1"]
    );
}

#[test]
fn core_shortcut_routes_eye_enable_wifi_to_substrate_plane() {
    let route =
        resolve_core_shortcuts("eye", &["enable".to_string(), "wifi".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://substrate-plane");
    assert_eq!(route.args, vec!["eye-bind", "--op=enable", "--source=wifi"]);
}

#[test]
fn core_shortcut_routes_substrate_enable_biological_to_substrate_plane() {
    let route = resolve_core_shortcuts(
        "substrate",
        &[
            "enable".to_string(),
            "biological".to_string(),
            "--persona=neural-watch".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://substrate-plane");
    assert_eq!(
        route.args,
        vec!["bio-enable", "--mode=biological", "--persona=neural-watch"]
    );
}

