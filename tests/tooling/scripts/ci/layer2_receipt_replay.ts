#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ReplayEvent = {
  id?: string;
  lane_id?: string;
  stage?: string;
  payload?: Record<string, unknown>;
  expected_transition_hash?: string;
  expected_state_hash?: string;
};

type ReplayBundle = {
  version?: number;
  bundle_id?: string;
  events?: ReplayEvent[];
};

type ReplayState = {
  submitted: number;
  routed: number;
  adapter_invoked: number;
  failed: number;
  completed: number;
  classifications: Record<string, number>;
  queue_backpressure_actions: {
    defer_noncritical: number;
    shed_noncritical: number;
    quarantine_new_ingress: number;
  };
  conduit_outages_detected: number;
  conduit_recoveries_started: number;
  conduit_recoveries_completed: number;
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/layer2_receipt_replay_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    bundlePath: cleanText(
      readFlag(argv, 'bundle') || 'tests/tooling/fixtures/layer2_receipt_bundle_golden.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/LAYER2_RECEIPT_REPLAY_CURRENT.md',
      400,
    ),
  };
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((row) => stableStringify(row)).join(',')}]`;
  }
  const row = value as Record<string, unknown>;
  const keys = Object.keys(row).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(row[key])}`).join(',')}}`;
}

function sha256(input: string): string {
  return createHash('sha256').update(input).digest('hex');
}

function stateHash(state: ReplayState): string {
  return sha256(stableStringify(state));
}

function loadBundle(root: string, relPath: string): ReplayBundle {
  const abs = path.resolve(root, relPath);
  return JSON.parse(fs.readFileSync(abs, 'utf8')) as ReplayBundle;
}

function markdown(report: any): string {
  const lines = [
    '# Layer2 Receipt Replay',
    '',
    `- bundle: ${report.bundle_path}`,
    `- events: ${report.summary.event_count}`,
    `- divergence_count: ${report.summary.divergence_count}`,
    '',
    '| idx | event_id | stage | transition_hash | state_hash |',
    '| ---: | --- | --- | --- | --- |',
  ];
  for (const row of report.transitions) {
    lines.push(
      `| ${row.index} | ${row.event_id} | ${row.stage} | ${row.transition_hash} | ${row.state_hash} |`,
    );
  }
  lines.push('');
  lines.push('## Divergences');
  if (report.divergences.length === 0) {
    lines.push('- none');
  } else {
    for (const row of report.divergences) {
      lines.push(`- ${row.id}: ${row.detail}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  let bundle: ReplayBundle;
  try {
    bundle = loadBundle(root, args.bundlePath);
  } catch (err) {
    const payload = {
      ok: false,
      type: 'layer2_receipt_replay',
      error: 'layer2_replay_bundle_read_failed',
      detail: cleanText(String(err), 400),
      bundle_path: args.bundlePath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const events = Array.isArray(bundle.events) ? bundle.events : [];
  const state: ReplayState = {
    submitted: 0,
    routed: 0,
    adapter_invoked: 0,
    failed: 0,
    completed: 0,
    classifications: {},
    queue_backpressure_actions: {
      defer_noncritical: 0,
      shed_noncritical: 0,
      quarantine_new_ingress: 0,
    },
    conduit_outages_detected: 0,
    conduit_recoveries_started: 0,
    conduit_recoveries_completed: 0,
  };

  const divergences: { id: string; detail: string }[] = [];
  const transitions: any[] = [];
  let previousTransitionHash = sha256('layer2_replay_seed_v1');

  for (let i = 0; i < events.length; i += 1) {
    const ev = events[i] || {};
    const stage = cleanText(ev.stage || 'unknown', 120).toLowerCase();
    const lane = cleanText(ev.lane_id || '', 120);
    const eventId = cleanText(ev.id || `event_${i + 1}`, 120);

    if (stage === 'task_submission') {
      state.submitted += 1;
    } else if (stage === 'queue_routing') {
      if (state.submitted <= 0) {
        divergences.push({ id: 'queue_routing_without_submission', detail: `event=${eventId}` });
      }
      state.routed += 1;
    } else if (stage === 'adapter_invocation_envelope') {
      if (state.routed <= 0) {
        divergences.push({ id: 'adapter_invocation_without_route', detail: `event=${eventId}` });
      }
      state.adapter_invoked += 1;
    } else if (stage === 'failure_classification') {
      const classification = cleanText(((ev.payload as any)?.classification as string) || '', 120);
      if (!classification) {
        divergences.push({ id: 'failure_classification_missing_label', detail: `event=${eventId}` });
      } else {
        state.classifications[classification] = (state.classifications[classification] || 0) + 1;
      }
      state.failed += 1;
    } else if (stage === 'queue_backpressure_action') {
      if (state.routed <= 0) {
        divergences.push({ id: 'queue_backpressure_without_route', detail: `event=${eventId}` });
      }
      const action = cleanText(((ev.payload as any)?.action as string) || '', 120);
      if (
        action !== 'defer_noncritical' &&
        action !== 'shed_noncritical' &&
        action !== 'quarantine_new_ingress'
      ) {
        divergences.push({ id: 'queue_backpressure_unknown_action', detail: `event=${eventId};action=${action || 'missing'}` });
      } else {
        state.queue_backpressure_actions[action] += 1;
      }
    } else if (stage === 'conduit_outage_detected') {
      if (state.adapter_invoked <= 0) {
        divergences.push({ id: 'conduit_outage_without_adapter_invocation', detail: `event=${eventId}` });
      }
      state.conduit_outages_detected += 1;
    } else if (stage === 'conduit_recovery_started') {
      if (state.conduit_outages_detected <= state.conduit_recoveries_started) {
        divergences.push({ id: 'conduit_recovery_started_without_outage', detail: `event=${eventId}` });
      }
      state.conduit_recoveries_started += 1;
    } else if (stage === 'conduit_recovery_completed') {
      if (state.conduit_recoveries_started <= state.conduit_recoveries_completed) {
        divergences.push({ id: 'conduit_recovery_completed_without_start', detail: `event=${eventId}` });
      }
      state.conduit_recoveries_completed += 1;
    } else if (stage === 'task_completed') {
      if (state.adapter_invoked <= 0) {
        divergences.push({ id: 'completion_without_adapter_invocation', detail: `event=${eventId}` });
      }
      state.completed += 1;
    } else {
      divergences.push({ id: 'unknown_stage', detail: `event=${eventId},stage=${stage || 'empty'}` });
    }

    if (!lane) {
      divergences.push({ id: 'lane_id_missing', detail: `event=${eventId}` });
    }

    const transitionHash = sha256(
      `${previousTransitionHash}|${stableStringify({ id: eventId, stage, lane, payload: ev.payload || {} })}`,
    );
    previousTransitionHash = transitionHash;
    const currentStateHash = stateHash(state);

    const expectedTransitionHash = cleanText(ev.expected_transition_hash || '', 200);
    if (expectedTransitionHash && expectedTransitionHash !== transitionHash) {
      divergences.push({
        id: 'transition_hash_mismatch',
        detail: `event=${eventId},expected=${expectedTransitionHash},actual=${transitionHash}`,
      });
    }

    const expectedStateHash = cleanText(ev.expected_state_hash || '', 200);
    if (expectedStateHash && expectedStateHash !== currentStateHash) {
      divergences.push({
        id: 'state_hash_mismatch',
        detail: `event=${eventId},expected=${expectedStateHash},actual=${currentStateHash}`,
      });
    }

    transitions.push({
      index: i,
      event_id: eventId,
      stage,
      lane_id: lane,
      transition_hash: transitionHash,
      state_hash: currentStateHash,
    });
  }

  if (state.conduit_outages_detected <= 0) {
    divergences.push({ id: 'conduit_auto_heal_missing_outage_detection', detail: 'no conduit_outage_detected stage found' });
  }
  if (state.conduit_recoveries_started <= 0) {
    divergences.push({ id: 'conduit_auto_heal_missing_recovery_start', detail: 'no conduit_recovery_started stage found' });
  }
  if (state.conduit_recoveries_completed <= 0) {
    divergences.push({ id: 'conduit_auto_heal_missing_recovery_complete', detail: 'no conduit_recovery_completed stage found' });
  }
  if (state.conduit_recoveries_completed !== state.conduit_recoveries_started) {
    divergences.push({
      id: 'conduit_auto_heal_recovery_count_mismatch',
      detail: `started=${state.conduit_recoveries_started};completed=${state.conduit_recoveries_completed}`,
    });
  }
  if (state.conduit_recoveries_started > state.conduit_outages_detected) {
    divergences.push({
      id: 'conduit_auto_heal_recovery_exceeds_outage_count',
      detail: `outages=${state.conduit_outages_detected};started=${state.conduit_recoveries_started}`,
    });
  }
  for (const [action, count] of Object.entries(state.queue_backpressure_actions)) {
    if (count <= 0) {
      divergences.push({
        id: 'queue_backpressure_action_missing',
        detail: action,
      });
    }
  }

  const report = {
    ok: divergences.length === 0,
    type: 'layer2_receipt_replay',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    bundle_id: cleanText(bundle.bundle_id || '', 120),
    bundle_path: args.bundlePath,
    markdown_path: args.markdownOutPath,
    summary: {
      event_count: events.length,
      divergence_count: divergences.length,
      pass: divergences.length === 0,
    },
    final_state: state,
    final_transition_hash: previousTransitionHash,
    final_state_hash: stateHash(state),
    auto_heal: {
      outages_detected: state.conduit_outages_detected,
      recoveries_started: state.conduit_recoveries_started,
      recoveries_completed: state.conduit_recoveries_completed,
      complete:
        state.conduit_outages_detected > 0 &&
        state.conduit_recoveries_started > 0 &&
        state.conduit_recoveries_completed > 0 &&
        state.conduit_recoveries_completed === state.conduit_recoveries_started &&
        state.conduit_recoveries_started <= state.conduit_outages_detected,
    },
    queue_backpressure_actions: state.queue_backpressure_actions,
    transitions,
    divergences,
    failures: divergences,
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, markdown(report));

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
