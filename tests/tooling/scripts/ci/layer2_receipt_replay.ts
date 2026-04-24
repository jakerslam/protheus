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
  graph_expectations?: Record<
    string,
    {
      required_edges?: string[];
      allow_unexpected_edges?: boolean;
    }
  >;
  events?: ReplayEvent[];
};

type Layer2ParityLane = {
  lane_id?: string;
  id?: string;
  status?: string;
  requested_action?: string;
  expected_receipt_type?: string;
  replay_fixture_path?: string;
  replay_artifact_path?: string;
};

type Layer2ParityManifest = {
  version?: number;
  lanes?: Layer2ParityLane[];
};

type ReplayState = {
  submitted: number;
  routed: number;
  adapter_invoked: number;
  failed: number;
  completed: number;
  canceled: number;
  timed_out: number;
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
    diffOutPath: cleanText(
      readFlag(argv, 'out-diff') || 'core/local/artifacts/layer2_replay_diff_current.json',
      400,
    ),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/layer2_parity_matrix.json',
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

function loadLayer2ParityManifest(root: string, relPath: string): Layer2ParityManifest {
  const abs = path.resolve(root, relPath);
  return JSON.parse(fs.readFileSync(abs, 'utf8')) as Layer2ParityManifest;
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

const REQUIRED_GRAPH_EDGES = [
  'task_submission->queue_routing',
  'queue_routing->adapter_invocation_envelope',
  'adapter_invocation_envelope->failure_classification',
  'failure_classification->queue_backpressure_action',
  'queue_backpressure_action->queue_backpressure_action',
  'queue_backpressure_action->conduit_outage_detected',
  'conduit_outage_detected->conduit_recovery_started',
  'conduit_recovery_started->conduit_recovery_completed',
  'conduit_recovery_completed->task_completed',
] as const;

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
  let manifest: Layer2ParityManifest;
  try {
    manifest = loadLayer2ParityManifest(root, args.manifestPath);
  } catch (err) {
    const payload = {
      ok: false,
      type: 'layer2_receipt_replay',
      error: 'layer2_parity_manifest_read_failed',
      detail: cleanText(String(err), 400),
      manifest_path: args.manifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const events = Array.isArray(bundle.events) ? bundle.events : [];
  const parityLanes = Array.isArray(manifest.lanes) ? manifest.lanes : [];
  const state: ReplayState = {
    submitted: 0,
    routed: 0,
    adapter_invoked: 0,
    failed: 0,
    completed: 0,
    canceled: 0,
    timed_out: 0,
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
    } else if (stage === 'task_cancellation') {
      if (state.submitted <= 0) {
        divergences.push({ id: 'cancellation_without_submission', detail: `event=${eventId}` });
      }
      state.canceled += 1;
    } else if (stage === 'task_timeout') {
      if (state.submitted <= 0) {
        divergences.push({ id: 'timeout_without_submission', detail: `event=${eventId}` });
      }
      state.timed_out += 1;
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

  const laneTransitions = new Map<string, any[]>();
  const actualStages = new Set<string>();
  const actualReceiptTypes = new Set<string>();
  for (const transition of transitions) {
    const laneId = cleanText(transition?.lane_id || '', 120) || 'lane_unknown';
    const stage = cleanText(transition?.stage || '', 120);
    if (stage) actualStages.add(stage);
    const bucket = laneTransitions.get(laneId) || [];
    bucket.push(transition);
    laneTransitions.set(laneId, bucket);
  }
  for (const event of events) {
    const receiptType = cleanText(((event.payload || {}) as any).receipt_type || '', 160);
    if (receiptType) actualReceiptTypes.add(receiptType);
  }
  const laneGraphDiff: Record<
    string,
    {
      required_edges: string[];
      actual_edges: string[];
      edge_counts: Record<string, number>;
      missing_required_edges: string[];
      unexpected_edges: string[];
      pass: boolean;
    }
  > = {};
  const globalRequiredEdgeSet = new Set<string>();
  const globalActualEdgeCounts: Record<string, number> = {};
  const globalMissingEdgeSet = new Set<string>();
  const globalUnexpectedEdgeSet = new Set<string>();

  for (const [laneId, laneRows] of laneTransitions.entries()) {
    const laneEdgeCounts: Record<string, number> = {};
    for (let i = 1; i < laneRows.length; i += 1) {
      const previousStage = cleanText(laneRows[i - 1]?.stage || '', 120);
      const currentStage = cleanText(laneRows[i]?.stage || '', 120);
      if (!previousStage || !currentStage) continue;
      const edge = `${previousStage}->${currentStage}`;
      laneEdgeCounts[edge] = (laneEdgeCounts[edge] || 0) + 1;
      globalActualEdgeCounts[edge] = (globalActualEdgeCounts[edge] || 0) + 1;
    }

    const laneExpectation = (bundle.graph_expectations && bundle.graph_expectations[laneId]) || {};
    const expectedEdgesRaw =
      Array.isArray(laneExpectation.required_edges) && laneExpectation.required_edges.length > 0
        ? laneExpectation.required_edges
        : Array.from(REQUIRED_GRAPH_EDGES);
    const requiredEdges = Array.from(
      new Set(
        expectedEdgesRaw
          .map((edge) => cleanText(edge || '', 200).toLowerCase())
          .filter(Boolean),
      ),
    ).sort();
    for (const edge of requiredEdges) globalRequiredEdgeSet.add(edge);
    const requiredEdgeSet = new Set(requiredEdges);
    const laneActualEdges = Object.keys(laneEdgeCounts).sort();
    const laneMissingRequiredEdges = requiredEdges.filter((edge) => !laneEdgeCounts[edge]);
    const allowUnexpectedEdges = laneExpectation.allow_unexpected_edges === true;
    const laneUnexpectedEdges = allowUnexpectedEdges
      ? []
      : laneActualEdges.filter((edge) => !requiredEdgeSet.has(edge));

    for (const edge of laneMissingRequiredEdges) globalMissingEdgeSet.add(edge);
    for (const edge of laneUnexpectedEdges) globalUnexpectedEdgeSet.add(edge);

    if (laneMissingRequiredEdges.length > 0) {
      divergences.push({
        id: 'lane_receipt_graph_missing_required_edge',
        detail: `lane=${laneId};edges=${laneMissingRequiredEdges.join(',')}`,
      });
    }
    if (laneUnexpectedEdges.length > 0) {
      divergences.push({
        id: 'lane_receipt_graph_unexpected_edge',
        detail: `lane=${laneId};edges=${laneUnexpectedEdges.join(',')}`,
      });
    }

    laneGraphDiff[laneId] = {
      required_edges: requiredEdges,
      actual_edges: laneActualEdges,
      edge_counts: laneEdgeCounts,
      missing_required_edges: laneMissingRequiredEdges,
      unexpected_edges: laneUnexpectedEdges,
      pass: laneMissingRequiredEdges.length === 0 && laneUnexpectedEdges.length === 0,
    };
  }

  const actualEdges = Object.keys(globalActualEdgeCounts).sort();
  const missingRequiredEdges = Array.from(globalMissingEdgeSet).sort();
  const unexpectedEdges = Array.from(globalUnexpectedEdgeSet).sort();
  const matrixLaneReplayRows = parityLanes
    .map((lane) => {
      const laneId = cleanText(lane.lane_id || lane.id || '', 120);
      const status = cleanText(lane.status || '', 40).toLowerCase();
      const requestedAction = cleanText(lane.requested_action || '', 160).toLowerCase();
      const expectedReceiptType = cleanText(lane.expected_receipt_type || '', 160);
      const expectedBundleMatches = cleanText(lane.replay_fixture_path || '', 400) === args.bundlePath;
      const stageCovered = !!requestedAction && actualStages.has(requestedAction);
      const receiptCovered = !!expectedReceiptType && actualReceiptTypes.has(expectedReceiptType);
      const replayRequired = status !== 'experimental' && status !== 'blocked';
      const pass = !replayRequired || (expectedBundleMatches && stageCovered && receiptCovered);
      return {
        lane_id: laneId,
        status,
        requested_action: requestedAction,
        expected_receipt_type: expectedReceiptType,
        replay_required: replayRequired,
        expected_bundle_matches: expectedBundleMatches,
        stage_covered: stageCovered,
        receipt_covered: receiptCovered,
        pass,
      };
    })
    .filter((row) => row.replay_required);
  const matrixLaneReplayFailures = matrixLaneReplayRows.filter((row) => !row.pass);
  for (const row of matrixLaneReplayFailures) {
    divergences.push({
      id: 'matrix_lane_replay_diff_failed',
      detail: `lane=${row.lane_id};bundle=${row.expected_bundle_matches};stage=${row.stage_covered};receipt=${row.receipt_covered}`,
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
    manifest_path: args.manifestPath,
    markdown_path: args.markdownOutPath,
    summary: {
      event_count: events.length,
      divergence_count: divergences.length,
      matrix_lane_count: matrixLaneReplayRows.length,
      matrix_lane_replay_failure_count: matrixLaneReplayFailures.length,
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
    receipt_graph_diff: {
      required_edges: Array.from(globalRequiredEdgeSet).sort(),
      actual_edges: actualEdges,
      edge_counts: globalActualEdgeCounts,
      missing_required_edges: missingRequiredEdges,
      unexpected_edges: unexpectedEdges,
      pass: missingRequiredEdges.length === 0 && unexpectedEdges.length === 0,
    },
    matrix_lane_replay_diff: {
      lane_count: matrixLaneReplayRows.length,
      failing_lane_count: matrixLaneReplayFailures.length,
      lanes: matrixLaneReplayRows,
      pass: matrixLaneReplayFailures.length === 0,
    },
    lane_graph_diff: laneGraphDiff,
    transitions,
    divergences,
    failures: divergences,
    artifact_paths: [args.markdownOutPath, args.diffOutPath],
  };

  const diffOutAbs = path.resolve(root, args.diffOutPath);
  fs.mkdirSync(path.dirname(diffOutAbs), { recursive: true });
  fs.writeFileSync(
    diffOutAbs,
    `${JSON.stringify(
      {
        ok: report.receipt_graph_diff.pass,
        type: 'layer2_replay_diff',
        generated_at: report.generated_at,
        revision: report.revision,
        bundle_id: report.bundle_id,
        bundle_path: report.bundle_path,
        manifest_path: report.manifest_path,
        summary: {
          lane_count: Object.keys(report.lane_graph_diff || {}).length,
          matrix_lane_count: report.matrix_lane_replay_diff.lane_count,
          matrix_lane_replay_failure_count: report.matrix_lane_replay_diff.failing_lane_count,
          required_edge_count: Array.isArray(report.receipt_graph_diff?.required_edges)
            ? report.receipt_graph_diff.required_edges.length
            : 0,
          actual_edge_count: actualEdges.length,
          missing_required_edge_count: missingRequiredEdges.length,
          unexpected_edge_count: unexpectedEdges.length,
          pass: report.receipt_graph_diff.pass,
        },
        receipt_graph_diff: report.receipt_graph_diff,
        matrix_lane_replay_diff: report.matrix_lane_replay_diff,
        lane_graph_diff: report.lane_graph_diff,
      },
      null,
      2,
    )}\n`,
    'utf8',
  );
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
