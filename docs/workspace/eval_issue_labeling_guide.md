# Eval Issue Labeling Guide (v1)

Purpose: label chat/tool failures consistently so eval quality gates can measure precision, recall, and actionability.

## Required label fields

Each labeled row must include:

1. `issue_class`
2. `severity`
3. `is_failure` (boolean)
4. `expected_fix`
5. `source_event_id`

## Severity rubric

1. `critical`: repeated user-visible breakage, unsafe behavior, or routing collapse.
2. `high`: clear user-facing failure that blocks or strongly degrades task completion.
3. `medium`: bounded defect with workable fallback.
4. `low`: minor quality issue with no major task impact.
5. `info`: not a defect; useful observation.

## Issue classes with examples

### `hallucination`

- Positive example: assistant claims a policy/tool state that is absent from trace/state evidence.
- Negative example: assistant says "I infer X" and explicitly marks it as inference.

### `wrong_tool_selection`

- Positive example: user asks for local file tooling and assistant routes to web search.
- Negative example: user explicitly asks for web lookup and assistant selects `web_search`.

### `no_response`

- Positive example: assistant returns fallback template ("final reply did not render") without answering the user prompt.
- Negative example: assistant gives concise degraded diagnosis plus actionable next step.

### `response_loop`

- Positive example: same or near-identical failure template repeated for 3+ turns.
- Negative example: one fallback message followed by a distinct corrective response.

### `tool_output_misdirection`

- Positive example: response contains unrelated external card/snippet content for a local-workspace request.
- Negative example: mixed evidence synthesis where each source is relevant and attributed.

### `policy_block_misframing`

- Positive example: policy block is presented as generic outage or with no remediation guidance.
- Negative example: assistant states policy block clearly and provides bounded remediation steps.

## Labeling rules

1. Prefer one primary class per row.
2. Use `critical` or `high` when user cannot make progress without retry/patch.
3. If evidence is ambiguous, set lower severity and include uncertainty in `expected_fix`.
4. Do not label user-quoted failure text as assistant failure unless assistant endorses it as current truth.

