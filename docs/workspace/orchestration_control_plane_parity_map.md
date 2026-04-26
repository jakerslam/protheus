# Orchestration Control-Plane Parity Map

Status: active transition map
Owner: surface/orchestration
Purpose: keep the Infring control plane understandable against OpenHands and OpenFang while preventing coordination authority from scattering into Shell/Core.

## Boundary Rule

Core decides truth and permission.
Orchestration decides what should happen next.
Shell shows and collects interaction.

## OpenHands Parity

OpenHands separates the server/conversation manager, runtime manager, action execution server, event stream, and memory condenser.

Infring mapping:

- OpenHands EventStream -> Infring workflow/state streams under `response_workflow`, `local/state/ops/orchestration/**`, and eval trace artifacts.
- OpenHands action execution -> Infring typed tool-family contracts (`workspace`, `web`, `memory`, `agent`, `shell`, `browser`) with request, observation, and receipt binding.
- OpenHands runtime/sandbox -> Infring Core/Gateway execution authority, with Orchestration only recommending and sequencing tool requests.
- OpenHands memory condenser -> Infring memory/context surfaces, with Orchestration consuming snapshots and packaging result context.
- OpenHands UI event stream -> Infring UI harness streams (`workflow_state`, `agent_internal_notes`, `tool_trace`, `eval_trace`, `final_answer`).

Parity requirement:

- The control plane must be an event-sourced action/observation loop, not a chat-text workflow script.
- Every user turn should have a typed chain: user input, workflow decision, action request, observation, and final LLM response.

## OpenFang Parity

OpenFang separates Kernel, Runtime, Memory, Hands, API, Channels, Extensions, and Migration. Its Kernel owns workflows, RBAC, metering, scheduler, background execution, and capability gates.

Infring mapping:

- OpenFang workflow engine -> `surface/orchestration/src/control_plane/**` workflow selection, typed graph compilation, sequencing, and recovery.
- OpenFang capability manifests -> future Infring orchestration-managed agent capability manifests, while Core retains enforcement.
- OpenFang agent loop -> Infring final LLM stage plus tool observation loop.
- OpenFang scheduler/metering -> Core truth; Orchestration may propose budgets and termination limits but does not enforce canonical quotas.
- OpenFang Hands -> future Infring reusable orchestration templates with manifest-bound tools and lifecycle state.
- OpenFang audit trail -> Core receipts plus Orchestration correlation metadata.

Parity requirement:

- Workflow runs must have terminal states, run budgets, loop guards, retry/escalation policy, and telemetry streams.
- Tool access must be typed by family and bound to receipts.

## Infring Control-Plane Responsibilities

Canonical Rust home: `surface/orchestration/src/**`

Current required control-plane concerns:

- Decomposition
- Coordination
- Sequencing
- Recovery
- Result packaging
- Workflow graph validation
- Structured gate contracts
- Telemetry stream separation
- Tool-family request/observation contracts

Forbidden leakage:

- No system-authored fallback text in visible chat.
- No automatic task/info or workflow-route classifier deciding tool use before the LLM gate.
- No control-plane coordination authority in Shell wrappers.
- No canonical policy truth or execution admission in Orchestration.

## Guard

The active guard is:

`cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin workflow_contract_guard -- --strict=1`

It checks that workflow JSON compiles into typed graphs, structured gates expose only multiple-choice/text-input shapes, tool families have request/observation/receipt contracts, run budgets and terminal states exist, telemetry streams are separate, and visible chat remains LLM-final-only.
