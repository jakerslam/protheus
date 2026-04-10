// SPDX-License-Identifier: Apache-2.0
use persona_dispatch_security_gate::evaluate_persona_dispatch_gate;

const SCRIPT_REL: &str = "client/runtime/systems/ops/protheus_control_plane.js";
const LENSES: &[&str] = &["guardian", "operator"];

fn decision(requested_lens: Option<&str>, blocked_paths: &[&str]) -> persona_dispatch_security_gate::PersonaDispatchGateDecision {
    evaluate_persona_dispatch_gate(
        SCRIPT_REL,
        requested_lens,
        LENSES,
        blocked_paths,
        false,
        false,
    )
}

#[test]
fn blocked_dispatch_path_fails_closed() {
    let decision = decision(Some("guardian"), &[SCRIPT_REL]);

    assert!(!decision.ok);
    assert_eq!(decision.code, "blocked_dispatch_path");
}

#[test]
fn invalid_requested_lens_uses_valid_fallback() {
    let decision = decision(Some("nonexistent"), &[]);

    assert!(decision.ok);
    assert_eq!(decision.selected_lens.as_deref(), Some("guardian"));
    assert!(decision.envelope.fallback_used);
}

#[test]
fn deterministic_error_envelope_is_stable() {
    let first = decision(Some("guardian"), &[SCRIPT_REL]);
    let second = decision(Some("guardian"), &[SCRIPT_REL]);

    assert!(!first.ok);
    assert_eq!(first.envelope, second.envelope);
    assert_eq!(
        first.envelope.deterministic_key,
        second.envelope.deterministic_key
    );
}
