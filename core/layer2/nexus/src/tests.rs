use crate::main_nexus::{DeliveryAuthorizationInput, LeaseIssueRequest, MAIN_NEXUS_ID};
use crate::policy::{DefaultNexusPolicy, NexusFeatureFlags, TrustClass, VerityClass};
use crate::registry::{ModuleKind, ModuleLifecycleState, SubNexusRegistration};
use crate::{ConnectionTemplate, RevocationCause};
use crate::{MainNexusControlPlane, NexusReceiptKind};

fn test_plane() -> MainNexusControlPlane {
    MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        DefaultNexusPolicy::default(),
    )
}

#[test]
fn registration_creates_control_plane_receipt() {
    let mut nexus = test_plane();
    let receipt = nexus
        .register_sub_nexus(
            "tester",
            SubNexusRegistration::new(
                "module_a",
                ModuleKind::Other,
                TrustClass::InterModuleData,
                VerityClass::Standard,
            ),
        )
        .expect("registration");
    assert_eq!(receipt.kind, NexusReceiptKind::Registration);
    assert_eq!(receipt.target.as_deref(), Some(MAIN_NEXUS_ID));
    assert!(nexus.registry().contains("module_a"));
}

#[test]
fn local_first_routing_bypasses_cross_module_lease() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let auth = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: None,
            source: "context_stacks".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "read".to_string(),
            offered_verity: VerityClass::High,
            now_ms: None,
        },
    );
    assert!(auth.allowed);
    assert!(auth.local_resolution);
    assert_eq!(nexus.metrics().local_resolution_count, 1);
}

#[test]
fn lease_issue_and_direct_delivery_authorization_succeeds() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let lease = nexus
        .issue_route_lease(
            "tester",
            LeaseIssueRequest {
                source: "client_ingress".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["read".to_string()],
                required_verity: VerityClass::Standard,
                trust_class: TrustClass::ClientIngressBoundary,
                requested_ttl_ms: 60_000,
                template_id: None,
                template_version: None,
            },
        )
        .expect("lease");
    let auth = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id.clone()),
            source: "client_ingress".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "read".to_string(),
            offered_verity: VerityClass::High,
            now_ms: None,
        },
    );
    assert!(auth.allowed);
    assert!(!auth.local_resolution);
    assert!(auth.conduit_link_id.is_some());
}

#[test]
fn draining_lifecycle_blocks_new_leases_but_not_existing_in_flight() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let first_lease = nexus
        .issue_route_lease(
            "tester",
            LeaseIssueRequest {
                source: "stomach".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["write".to_string()],
                required_verity: VerityClass::High,
                trust_class: TrustClass::InterModuleData,
                requested_ttl_ms: 120_000,
                template_id: None,
                template_version: None,
            },
        )
        .expect("lease before drain");
    nexus
        .set_module_lifecycle(
            "tester",
            "context_stacks",
            ModuleLifecycleState::Draining {
                drain_deadline_ms: first_lease.expires_at_ms.saturating_add(5_000),
            },
        )
        .expect("lifecycle");

    let second_attempt = nexus.issue_route_lease(
        "tester",
        LeaseIssueRequest {
            source: "stomach".to_string(),
            target: "context_stacks".to_string(),
            schema_ids: vec!["module.context".to_string()],
            verbs: vec!["write".to_string()],
            required_verity: VerityClass::High,
            trust_class: TrustClass::InterModuleData,
            requested_ttl_ms: 60_000,
            template_id: None,
            template_version: None,
        },
    );
    assert!(second_attempt.is_err());
    let auth = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: Some(first_lease.lease_id.clone()),
            source: "stomach".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "write".to_string(),
            offered_verity: VerityClass::Critical,
            now_ms: Some(first_lease.issued_at_ms.saturating_add(10)),
        },
    );
    assert!(auth.allowed);
}

#[test]
fn unauthorized_delivery_without_lease_fails_closed() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let auth = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: None,
            source: "stomach".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "read".to_string(),
            offered_verity: VerityClass::High,
            now_ms: None,
        },
    );
    assert!(!auth.allowed);
    assert_eq!(auth.reason, "cross_module_delivery_requires_lease");
}

#[test]
fn expired_lease_is_denied_and_revoked() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let lease = nexus
        .issue_route_lease(
            "tester",
            LeaseIssueRequest {
                source: "stomach".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["read".to_string()],
                required_verity: VerityClass::Standard,
                trust_class: TrustClass::InterModuleData,
                requested_ttl_ms: 5,
                template_id: None,
                template_version: None,
            },
        )
        .expect("lease");
    let auth = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id.clone()),
            source: "stomach".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "read".to_string(),
            offered_verity: VerityClass::High,
            now_ms: Some(lease.expires_at_ms.saturating_add(1)),
        },
    );
    assert!(!auth.allowed);
    assert_eq!(auth.reason, "lease_expired");
    let revoked = nexus
        .active_leases()
        .into_iter()
        .find(|row| row.lease_id == lease.lease_id)
        .expect("lease present");
    assert_eq!(revoked.revocation_cause, Some(RevocationCause::Expired));
}

#[test]
fn lifecycle_detach_revokes_related_leases() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let lease = nexus
        .issue_route_lease(
            "tester",
            LeaseIssueRequest {
                source: "stomach".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["write".to_string()],
                required_verity: VerityClass::High,
                trust_class: TrustClass::InterModuleData,
                requested_ttl_ms: 120_000,
                template_id: None,
                template_version: None,
            },
        )
        .expect("lease");
    nexus
        .set_module_lifecycle("tester", "context_stacks", ModuleLifecycleState::Detached)
        .expect("detach");
    let current = nexus
        .active_leases()
        .into_iter()
        .find(|row| row.lease_id == lease.lease_id)
        .expect("lease");
    assert_eq!(
        current.revocation_cause,
        Some(RevocationCause::TargetDetached)
    );
}

#[test]
fn lazy_conduit_creation_and_idle_teardown() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    let baseline = nexus.active_conduits();
    let lease = nexus
        .issue_route_lease(
            "tester",
            LeaseIssueRequest {
                source: "stomach".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["read".to_string()],
                required_verity: VerityClass::Standard,
                trust_class: TrustClass::InterModuleData,
                requested_ttl_ms: 60_000,
                template_id: None,
                template_version: None,
            },
        )
        .expect("lease");
    assert!(nexus.active_conduits() > baseline);
    let _ = nexus.authorize_direct_delivery(
        "tester",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id),
            source: "stomach".to_string(),
            target: "context_stacks".to_string(),
            schema_id: "module.context".to_string(),
            verb: "read".to_string(),
            offered_verity: VerityClass::High,
            now_ms: Some(1),
        },
    );
    let receipts = nexus.teardown_idle_conduits("tester", u64::MAX);
    assert!(!receipts.is_empty());
}

#[test]
fn template_instantiation_is_receipted_and_issues_lease() {
    let mut nexus = test_plane();
    nexus.register_v1_adapters("tester").expect("adapters");
    nexus
        .upsert_template(
            "tester",
            ConnectionTemplate {
                template_id: "ctx_sync".to_string(),
                version: 1,
                source: "stomach".to_string(),
                target: "context_stacks".to_string(),
                schema_ids: vec!["module.context".to_string()],
                verbs: vec!["read".to_string()],
                required_verity: VerityClass::Standard,
                trust_class: TrustClass::InterModuleData,
                default_ttl_ms: 50_000,
            },
        )
        .expect("upsert");
    let lease = nexus
        .issue_route_lease_from_template("tester", "ctx_sync", 1, None)
        .expect("template lease");
    assert_eq!(lease.template_id.as_deref(), Some("ctx_sync"));
    assert!(nexus
        .receipts()
        .iter()
        .any(|row| row.kind == NexusReceiptKind::TemplateInstantiation));
}
