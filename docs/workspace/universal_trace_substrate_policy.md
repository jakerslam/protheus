# Universal Trace Substrate Policy

Owner: Assurance / Observability
Status: canonical architecture policy
Applies to: runtime traces, decision traces, workflow phase traces, Sentinel evidence traces, Validation run traces, Gateway boundary traces, Shell projection traces, RSI/self-study traces, external-assimilation probe traces

## Purpose

InfRing needs one trace substrate that can explain behavior across domains.

Local telemetry is not enough. A subsystem-local trace can tell us what one component believed happened, but RSI, assimilation, incident synthesis, and architecture repair need causal stories that cross Kernel, Orchestration, Gateway, Shell, Validation, Governance, and Observability boundaries.

The universal trace substrate is the Observability-owned causal fabric for that work.

## External Framing

`Machine Learning Systems, Volume 1` Chapter 5, `AI Workflow`, frames production AI systems as workflow-level systems with feedback loops, distributed monitoring, degradation prevention, and system-level behavior. The same principle applies here: a trace system that cannot connect workflow decisions, runtime receipts, health, evals, and user-visible symptoms cannot support reliable self-understanding.

Reference: https://mlsysbook.ai/vol1/assets/downloads/Machine-Learning-Systems-Vol1.pdf

## Anti-Pattern: Fragmented Observability

Fragmented observability is a negative architecture state where each subsystem emits useful local facts, but no universal trace substrate ties those facts into a single causal graph.

Symptoms:

- Orchestration can explain a planner decision, but Sentinel cannot correlate it with Kernel receipts or Shell symptoms.
- Kernel can emit receipts, but workflow or eval context must be reconstructed by hand.
- Shell can show an operator-visible failure, but the trace does not preserve the Kernel/Gateway/Orchestration path that produced it.
- Validation can fail a controlled check, but the failure cannot be attached to the live runtime span or source-health context that made it relevant.
- Assimilation and RSI worksheets must reread scattered files because trace references do not share IDs, timestamps, authority classes, or subject identity.

Fragmented observability is not just messy reporting. It is a source-of-truth and diagnosis risk because it encourages local symptom patching instead of cross-domain causal repair.

## Ownership Rule

Observability owns the trace envelope and correlation fabric.

Domains own typed payload extensions.

Governance consumes trace-derived evidence to create verdicts, scorecards, gates, and issue-candidate routing.

Kernel remains authoritative for truth and permission. Orchestration remains responsible for coordination. Validation owns controlled checks. Shell projects traces only through bounded detail routes.

## Single Trace ID Rule

Each user request mints exactly one canonical `trace_id`.

This rule has no exceptions.

That same `trace_id` must flow unchanged through:

- the initial user request;
- intake and normalization;
- workflow selection and workflow phases;
- Orchestration planning, decomposition, sequencing, recovery, and result shaping;
- every tool call and tool result;
- Gateway and Conduit boundary spans;
- Kernel receipts, state transitions, capability checks, and policy decisions;
- Validation/eval/check spans triggered by the request;
- Shell projection spans and detail-route views;
- Sentinel observation, diagnostic, finding, incident, and issue-candidate spans.
- final response assembly and visible response delivery.

Subsystems may mint local `span_id` values, but they must not mint competing root trace IDs for the same causal story.

If a component receives work without a `trace_id`, it must fail closed for trace completeness: emit a diagnostic `fragmented_observability` failure, refuse to present the output as part of a complete causal story, and require the caller to reattach the work to the initial request trace before continuing.

The only allowed `parent_span_id: null` span is the root span created for the initial user request. External target probes, background diagnostics, Sentinel follow-ups, and automated validation runs must still be caused by a user-request trace or an explicitly scheduled operator request trace; they do not get a hidden exception.

## Canonical Trace Envelope

The canonical machine-readable seed is `observability/traces/trace_envelope.schema.json`.

Every first-class trace row should be projectable into that envelope, including:

- `trace_id`
- `span_id`
- `parent_span_id`
- `timestamp`
- `source_domain`
- `producer`
- `authority_class`
- `event_kind`
- `subject`
- `correlation`
- `payload`
- `payload_schema`
- `evidence_refs`
- `receipt_refs`
- `severity`
- `confidence`
- `schema_version`

The root envelope must stay generic. Domain-specific detail belongs in an extension payload and must be declared in `observability/traces/domain_trace_extension_registry.json`.

## Trace/Evidence/Finding Separation

Trace:

- answers what happened, when, where, and through which causal parent.
- preserves cross-domain correlation.
- can be high-volume and diagnostic.

Evidence:

- answers what should be trusted for proof, gate, or investigation.
- may be projected from traces but includes authority and freshness semantics.
- must remain compatible with the Assurance evidence envelope.

Finding:

- is synthesized interpretation from traces and evidence.
- belongs to Sentinel, Governance, or a controlled Validation report depending on authority class.
- must preserve trace and evidence refs rather than replacing them with prose.

## Required Source Domains

The universal trace substrate must support these source domains:

- `kernel`
- `orchestration`
- `gateway`
- `shell`
- `validation`
- `observability`
- `governance`
- `conduit`
- `external_target`

## Required Event Kinds

Minimum event kinds:

- `decision`
- `state_transition`
- `workflow_phase`
- `tool_call`
- `gateway_boundary`
- `receipt`
- `health`
- `telemetry`
- `validation_run`
- `eval`
- `benchmark`
- `gate`
- `runtime_finding`
- `sentinel_observation`
- `diagnostic_probe`
- `error`
- `recovery`

## Correlation Requirements

Every cross-domain trace must preserve:

- one stable `trace_id` minted at the initial user request and never reminted, replaced, dropped, or forked for child work;
- a `span_id` for the local event;
- `parent_span_id` when a span is caused by a previous span;
- subject identity such as session, request, workflow, plan, task, gateway, or artifact ID;
- authority class;
- evidence or receipt refs when the event claims truth, proof, failure, or recovery.

## Domain Extension Rule

Domains may add typed extension schemas, but they must not invent alternate root trace formats.

Allowed examples:

- Orchestration decision trace extension.
- Kernel Sentinel evidence trace extension.
- Validation run trace extension.
- Gateway boundary trace extension.
- Shell projection trace extension.

Forbidden examples:

- a second root `decision_trace` shape that cannot project into the universal envelope;
- a Shell-local full workflow trace cache;
- a Validation-only trace format that drops runtime correlation;
- a Sentinel evidence row that cannot preserve the original producer artifact.

## Relationship To Assurance

Observability owns the universal trace substrate.

Validation can attach controlled check output to trace spans.

Governance can derive verdicts, gates, scorecards, issue candidates, and release blockers from trace-linked evidence.

Sentinel uses universal traces as big-picture observation input. Sentinel findings should prefer trace clusters that cross domains over isolated local symptoms.

## Relationship To Assimilation And RSI

Assimilation and RSI both depend on system understanding.

For external assimilation, probe traces from the target system should use the same envelope with `source_domain: external_target`, then map behavior, architecture, capability, and failure model into the System Understanding Dossier.

For internal RSI, InfRing traces should feed the self-dossier so Sentinel can recognize multi-level failures before local patching starts.

The trace substrate is therefore shared infrastructure for both directions: understand another system, or understand ourselves.

## Implementation Direction

1. Standardize the trace envelope and extension registry under `observability/traces/**`.
2. Add projection adapters for existing Orchestration decision traces and Kernel Sentinel evidence streams.
3. Add cross-domain correlation IDs at Kernel, Orchestration, Gateway, Shell, Validation, and Governance boundaries.
4. Add a trace substrate guard that fails when new root trace schemas appear outside Observability.
5. Add query/index tooling so Sentinel, RSI, and operators can inspect causal stories without hand-reading scattered artifacts.

## One-Line Rule

Fragmented observability is a system-understanding failure. Universal traces are the Observability-owned causal spine.
