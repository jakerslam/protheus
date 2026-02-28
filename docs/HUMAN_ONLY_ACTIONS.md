# Human-Only Actions (Non-Executable Backlog Inputs)

Purpose: capture high-impact tasks that cannot be executed autonomously from backlog automation because they require human legal authority, identity, money movement approval, physical presence, or relationship management.

## How To Use

- Treat each action as a prerequisite artifact producer.
- After completion, attach evidence to `state/ops/evidence/` and reference it in the dependent backlog item receipt.
- Do not mark dependent backlog items done until the evidence artifact exists and is linked.

## Human-Only Task List

| ID | Human Action | Why This Is Human-Only | Evidence Artifact (suggested) | Backlog Dependencies |
|---|---|---|---|---|
| HMAN-001 | Select and contract independent security assessor(s) | Requires commercial negotiation, legal acceptance, and payment authority | `state/ops/evidence/external_security_contract_<date>.pdf` | `V2-012`, `V3-ENT-003`, `V3-SEC-004` |
| HMAN-002 | Approve coordinated disclosure policy and legal response workflow | Requires legal counsel and liability/risk sign-off | `state/ops/evidence/coordinated_disclosure_policy_signed_<date>.pdf` | `V3-SEC-004`, `V3-DOC-006` |
| HMAN-003 | Approve official reliability SLO policy targets and incident severity definitions | Requires executive risk acceptance and business tradeoff decisions | `state/ops/evidence/reliability_policy_approval_<date>.md` | `V3-REL-001`, `V3-OPS-001` |
| HMAN-004 | Approve benchmark publication policy (what can be public vs private) | Requires strategic disclosure decision and competitive/legal judgment | `state/ops/evidence/benchmark_publication_policy_<date>.md` | `V3-BENCH-001`, `V3-BENCH-002` |
| HMAN-005 | Fund provider accounts and authorize billing/payment rails | Requires custody of funds and account ownership authority | `state/ops/evidence/provider_funding_authorization_<date>.md` | `V3-BUD-001`, `V3-ECO-001`, `V3-BLK-001` |
| HMAN-006 | Approve and sign soul-token/root identity ceremonies | Requires identity proof and private-key consent that cannot be delegated | `state/ops/evidence/soul_token_ceremony_<date>.json` | `V2-058`, `V3-BLK-001`, `V3-CPY-001` |
| HMAN-007 | Complete hardware key ceremonies for trusted devices | Requires physical device possession and trusted environment controls | `state/ops/evidence/hardware_attestation_ceremony_<date>.json` | `V3-021`, `V3-CPY-001`, `V3-VENOM-001` |
| HMAN-008 | Execute legal/compliance filings (if enterprise launch path chosen) | Requires licensed legal/accounting actors and official filings | `state/ops/evidence/compliance_filing_bundle_<date>.zip` | `V2-013`, `V3-ENT-002`, `V3-DOC-005` |
| HMAN-009 | Approve risk thresholds for autonomous spend and action tiers | Requires personal/business risk appetite decisions | `state/ops/evidence/risk_tier_threshold_approval_<date>.md` | `V3-ACT-001`, `V3-AEX-001`, `V3-BUD-001` |
| HMAN-010 | Run periodic human governance review board (you + designated approvers) | Requires human accountability and judgment on governance drift | `state/ops/evidence/governance_review_minutes_<date>.md` | `V3-GOV-001`, `V3-DOC-002`, `V3-DOC-006` |
| HMAN-011 | Conduct live operator UX acceptance test with real operators | Requires real user interviews and qualitative acceptance sign-off | `state/ops/evidence/operator_uat_report_<date>.md` | `V3-USE-001`, `V3-USE-003`, `V3-OPS-004` |
| HMAN-012 | Approve incident communication templates for legal/public response | Requires legal/brand authority and escalation ownership | `state/ops/evidence/incident_comms_approval_<date>.md` | `V3-DOC-006`, `V3-ENT-003`, `V3-SEC-004` |

## Non-Negotiable Constraint

These tasks are intentionally not auto-executable. They anchor sovereignty, legal control, and accountability in the human root.
