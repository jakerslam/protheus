# Incident Response Workflow

## 1. Intake and Classification

1. Capture trigger source, first observed timestamp, and affected surfaces.
2. Classify severity using the policy taxonomy (`P0`-`P3`).
3. Record an explicit reason code and initial containment hypothesis.

## 2. Ownership and Escalation

1. Assign required roles: incident commander, lane owner, and reporting owner.
2. Start severity-based SLA timers for acknowledgement and escalation.
3. Escalate if SLA breach is reached or if blast radius expands.

## 3. Containment and Rollback Decision

1. Attempt containment in fail-closed mode first.
2. Evaluate rollback criteria against policy triggers.
3. If rollback is triggered, announce rollback and execute the documented rollback plan.

## 4. Communication Cadence

1. Emit `initial_alert` template.
2. Emit periodic `status_update` on SLA cadence.
3. Emit `mitigation_started` and `rollback_notice` when applicable.
4. Emit `resolved` only after verification checks pass.

## 5. Resolution and Verification

1. Verify restoration of expected runtime health and contracts.
2. Verify no unresolved critical regressions remain.
3. Capture final residual risk assessment and owner acknowledgement.

## 6. Post-Incident Closure

1. Produce required artifact set:
   - `incident_timeline`
   - `impact_assessment`
   - `root_cause`
   - `corrective_actions`
   - `evidence_bundle`
   - `owner_signoff`
2. Validate artifact fields against [`client/runtime/config/post_incident_artifact_schema.json`](/Users/jay/.openclaw/workspace/client/runtime/config/post_incident_artifact_schema.json).
3. Emit `postmortem_ready` template.
4. Close only when artifacts are complete and linked.

## 7. Waiver Handling

1. Waivers are temporary and explicit; they are not a bypass policy.
2. Every waiver must include:
   - `waiver_id`
   - `check_ids`
   - `reason`
   - `approver`
   - `expires_at`
3. Expired waivers are invalid and fail governance checks.
4. Waivers are tracked in [`client/runtime/config/incident_operations_governance_waivers.json`](/Users/jay/.openclaw/workspace/client/runtime/config/incident_operations_governance_waivers.json).
