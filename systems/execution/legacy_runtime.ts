#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');

type AnyObj = Record<string, any>;

type WorkflowStep = {
  id: string;
  kind: string;
  action: string;
  command: string;
  pause_after: boolean;
  params: Record<string, string>;
};

type ExecutionState = {
  cursor: number;
  paused: boolean;
  completed: boolean;
  last_step_id: string | null;
  processed_step_ids: string[];
  processed_events: number;
  digest: string;
};

type ExecutionReceipt = {
  workflow_id: string;
  status: string;
  deterministic: boolean;
  replayable: boolean;
  processed_steps: number;
  pause_reason: string | null;
  event_digest: string;
  events: string[];
  state: ExecutionState;
  metadata: Record<string, string>;
  warnings: string[];
};

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function stableHash(lines: string[]) {
  const h = crypto.createHash('sha256');
  for (let i = 0; i < lines.length; i += 1) {
    h.update(`${i}:${lines[i]}|`, 'utf8');
  }
  return h.digest('hex');
}

function normalizeStep(raw: AnyObj, idx: number): WorkflowStep {
  const params: Record<string, string> = {};
  const rawParams = raw && typeof raw.params === 'object' ? raw.params : {};
  for (const key of Object.keys(rawParams).sort()) {
    params[cleanText(key, 120)] = cleanText(rawParams[key], 200);
  }
  return {
    id: cleanText(raw && raw.id, 120),
    kind: cleanText(raw && raw.kind, 80),
    action: cleanText(raw && raw.action, 120),
    command: cleanText(raw && raw.command, 260),
    pause_after: Boolean(raw && raw.pause_after),
    params
  };
}

function normalizeStepId(step: WorkflowStep, idx: number) {
  return step.id || `step_${String(idx + 1).padStart(3, '0')}`;
}

function normalizeState(raw: AnyObj): ExecutionState {
  const cursorNum = Number(raw && raw.cursor);
  return {
    cursor: Number.isFinite(cursorNum) && cursorNum >= 0 ? Math.floor(cursorNum) : 0,
    paused: Boolean(raw && raw.paused),
    completed: Boolean(raw && raw.completed),
    last_step_id: raw && raw.last_step_id ? cleanText(raw.last_step_id, 120) : null,
    processed_step_ids: Array.isArray(raw && raw.processed_step_ids)
      ? raw.processed_step_ids.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
      : [],
    processed_events: Number.isFinite(Number(raw && raw.processed_events))
      ? Math.max(0, Math.floor(Number(raw && raw.processed_events)))
      : 0,
    digest: cleanText(raw && raw.digest, 128)
  };
}

function normalizeDefinition(raw: AnyObj) {
  const stepsRaw = Array.isArray(raw && raw.steps) ? raw.steps : [];
  const steps = stepsRaw.map((row: AnyObj, idx: number) => normalizeStep(row, idx));
  const metadata: Record<string, string> = {};
  const rawMeta = raw && typeof raw.metadata === 'object' ? raw.metadata : {};
  for (const key of Object.keys(rawMeta).sort()) {
    metadata[cleanText(key, 120)] = cleanText(rawMeta[key], 200);
  }
  return {
    workflow_id: cleanText(raw && raw.workflow_id, 160),
    deterministic_seed: cleanText(raw && raw.deterministic_seed, 160),
    pause_after_step: raw && raw.pause_after_step ? cleanText(raw.pause_after_step, 120) : null,
    resume: raw && raw.resume && typeof raw.resume === 'object' ? normalizeState(raw.resume) : null,
    steps,
    metadata
  };
}

function stepFingerprint(workflowId: string, seed: string, idx: number, stepId: string, step: WorkflowStep) {
  const parts = [workflowId, seed, String(idx), stepId, step.kind, step.action, step.command];
  for (const key of Object.keys(step.params).sort()) {
    parts.push(`${key}=${step.params[key]}`);
  }
  return stableHash(parts);
}

function failureReceipt(workflowId: string, reason: string): ExecutionReceipt {
  const digest = stableHash([workflowId, reason, 'failed']);
  return {
    workflow_id: workflowId,
    status: 'failed',
    deterministic: true,
    replayable: false,
    processed_steps: 0,
    pause_reason: reason,
    event_digest: digest,
    events: [`error:${reason}`],
    state: {
      cursor: 0,
      paused: false,
      completed: false,
      last_step_id: null,
      processed_step_ids: [],
      processed_events: 0,
      digest
    },
    metadata: {},
    warnings: [reason]
  };
}

function runWorkflowLegacySpec(specRaw: AnyObj): ExecutionReceipt {
  const spec = normalizeDefinition(specRaw && typeof specRaw === 'object' ? specRaw : {});
  const workflowId = spec.workflow_id
    || `wf_${stableHash([String(spec.steps.length), spec.deterministic_seed, JSON.stringify(spec.metadata)]).slice(0, 12)}`;

  const warnings: string[] = [];
  const state = spec.resume ? normalizeState(spec.resume) : normalizeState({});
  if (state.cursor > spec.steps.length) {
    warnings.push('resume_cursor_clamped');
    state.cursor = spec.steps.length;
  }

  const events: string[] = [];
  if (state.cursor > 0) {
    events.push(`resume:${state.cursor}`);
  }

  let pauseReason: string | null = null;
  for (let idx = state.cursor; idx < spec.steps.length; idx += 1) {
    const step = spec.steps[idx];
    const stepId = normalizeStepId(step, idx);
    const fp = stepFingerprint(workflowId, spec.deterministic_seed, idx, stepId, step);
    events.push(`exec:${stepId}:${fp}`);

    state.cursor = idx + 1;
    state.last_step_id = stepId;
    state.processed_step_ids.push(stepId);

    const shouldPause = Boolean(step.pause_after || (spec.pause_after_step && spec.pause_after_step === stepId));
    if (shouldPause) {
      state.paused = true;
      state.completed = false;
      pauseReason = `paused_after:${stepId}`;
      break;
    }
  }

  if (state.cursor >= spec.steps.length) {
    state.completed = true;
    state.paused = false;
    pauseReason = null;
  }

  const digestInput = [
    workflowId,
    spec.deterministic_seed,
    String(state.cursor),
    String(state.paused),
    String(state.completed),
    ...events
  ];
  for (const key of Object.keys(spec.metadata).sort()) {
    digestInput.push(`${key}=${spec.metadata[key]}`);
  }
  const digest = stableHash(digestInput);
  state.processed_events = events.length;
  state.digest = digest;

  return {
    workflow_id: workflowId,
    status: state.paused ? 'paused' : (state.completed ? 'completed' : 'running'),
    deterministic: true,
    replayable: true,
    processed_steps: state.cursor,
    pause_reason: pauseReason,
    event_digest: digest,
    events,
    state,
    metadata: spec.metadata,
    warnings
  };
}

function runWorkflowLegacyYaml(yaml: string): ExecutionReceipt {
  try {
    const parsed = JSON.parse(String(yaml || '{}'));
    return runWorkflowLegacySpec(parsed && typeof parsed === 'object' ? parsed : {});
  } catch (err) {
    return failureReceipt('invalid_workflow', `yaml_parse_failed:${cleanText(err && (err as any).message, 200)}`);
  }
}

module.exports = {
  runWorkflowLegacySpec,
  runWorkflowLegacyYaml,
  stableHash
};
