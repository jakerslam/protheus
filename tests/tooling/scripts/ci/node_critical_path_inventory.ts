#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ScriptClass = 'rust_native' | 'node_typescript' | 'npm_wrapper' | 'unknown';

type BurnLane = {
  id: string;
  domain: string;
  owner: string;
  priority: number;
  target_classification: ScriptClass;
  target_date: string;
  migration_status: string;
  notes: string;
};

type BurnPlan = {
  schema_id: string;
  schema_version: string;
  required_domains: string[];
  operator_critical_domains?: string[];
  operator_critical_target_classification?: ScriptClass;
  operator_critical_priority_cutoff_date?: string;
  allowed_node_typescript_prefixes: string[];
  ordered_migration_queue?: string[];
  lanes: BurnLane[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/node_critical_path_inventory_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    baselinePath: cleanText(
      readFlag(argv, 'baseline') || 'core/local/artifacts/node_critical_path_inventory_baseline.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/NODE_CRITICAL_PATH_INVENTORY_CURRENT.md',
      400,
    ),
    packagePath: cleanText(readFlag(argv, 'package') || 'package.json', 400),
    burnPlanPath: cleanText(
      readFlag(argv, 'burn-plan') || 'client/runtime/config/node_critical_path_burndown_plan.json',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): { ok: boolean; payload: any; detail: string } {
  try {
    return {
      ok: true,
      payload: JSON.parse(fs.readFileSync(filePath, 'utf8')),
      detail: 'loaded',
    };
  } catch (error) {
    return {
      ok: false,
      payload: null,
      detail: cleanText((error as Error)?.message || 'json_unavailable', 240),
    };
  }
}

function classifyScriptCommand(command: string): ScriptClass {
  const normalized = cleanText(command, 2000).toLowerCase();
  if (!normalized) return 'unknown';
  if (normalized.includes('cargo run') || normalized.includes('cargo test')) return 'rust_native';
  if (normalized.includes('node client/runtime/lib/ts_entrypoint.ts')) return 'node_typescript';
  if (normalized.startsWith('npm run -s ')) return 'npm_wrapper';
  return 'unknown';
}

function parseTsEntrypointTarget(command: string): string {
  const match = /node\s+client\/runtime\/lib\/ts_entrypoint\.ts\s+([^\s]+)/i.exec(command);
  if (!match) return '';
  const raw = cleanText(match[1] || '', 500);
  if (!raw) return '';
  return cleanText(raw.replace(/^['"]|['"]$/g, ''), 500);
}

function markdown(payload: any): string {
  const lines = [
    '# Node Critical Path Inventory',
    '',
    `Generated: ${payload.generated_at}`,
    `Revision: ${payload.revision}`,
    `Pass: ${payload.ok}`,
    '',
    '## Summary',
    `- critical_scripts_total: ${payload.summary.critical_scripts_total}`,
    `- critical_scripts_missing: ${payload.summary.critical_scripts_missing}`,
    `- rust_native: ${payload.summary.rust_native_count}`,
    `- node_typescript: ${payload.summary.node_typescript_count}`,
    `- npm_wrapper: ${payload.summary.npm_wrapper_count}`,
    `- unknown: ${payload.summary.unknown_count}`,
    `- node_dependency_ratio: ${payload.summary.node_dependency_ratio}`,
    `- migration_outstanding: ${payload.summary.migration_outstanding_count}`,
    `- migration_overdue: ${payload.summary.migration_overdue_count}`,
    `- required_domains_missing: ${payload.summary.required_domains_missing_count}`,
    `- operator_critical_priority_one_missing_rust: ${payload.summary.operator_critical_priority_one_missing_rust_count}`,
    `- operator_critical_target_classification_violations: ${payload.summary.operator_critical_target_classification_violation_count}`,
    `- operator_critical_cutoff_violations: ${payload.summary.operator_critical_cutoff_violation_count}`,
    `- ordered_migration_queue_count: ${payload.summary.ordered_migration_queue_count}`,
    `- ordered_migration_queue_duplicate_id_count: ${payload.summary.ordered_migration_queue_duplicate_id_count}`,
    `- ordered_migration_queue_unknown_id_count: ${payload.summary.ordered_migration_queue_unknown_id_count}`,
    `- ordered_migration_queue_missing_operator_priority_one_count: ${payload.summary.ordered_migration_queue_missing_operator_priority_one_count}`,
    `- ts_confinement_violations: ${payload.summary.ts_confinement_violation_count}`,
    '',
    '| script | domain | owner | priority | class | target_class | target_date | status | ts_target |',
    '| --- | --- | --- | ---: | --- | --- | --- | --- | --- |',
  ];
  for (const row of payload.rows) {
    lines.push(
      `| ${row.id} | ${row.domain} | ${row.owner} | ${row.priority} | ${row.classification} | ${row.target_classification} | ${row.target_date} | ${row.migration_status} | ${row.ts_entrypoint_target || 'n/a'} |`,
    );
  }
  lines.push('');
  lines.push('## Failures');
  if (!payload.failures.length) lines.push('- none');
  else payload.failures.forEach((row: any) => lines.push(`- ${row.id}: ${row.detail}`));
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const packagePath = path.resolve(root, args.packagePath);
  const packageJson = readJsonBestEffort(packagePath);
  if (!packageJson.ok) {
    const payload = {
      ok: false,
      type: 'node_critical_path_inventory',
      error: 'package_json_unavailable',
      detail: packageJson.detail,
      package_path: args.packagePath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const burnPlanPath = path.resolve(root, args.burnPlanPath);
  const burnPlanJson = readJsonBestEffort(burnPlanPath);
  if (!burnPlanJson.ok) {
    const payload = {
      ok: false,
      type: 'node_critical_path_inventory',
      error: 'node_critical_path_burndown_plan_unavailable',
      detail: burnPlanJson.detail,
      package_path: args.packagePath,
      burn_plan_path: args.burnPlanPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const scripts = packageJson.payload?.scripts || {};
  const burnPlan = (burnPlanJson.payload || {}) as Partial<BurnPlan>;
  const lanes = Array.isArray(burnPlan.lanes)
    ? burnPlan.lanes.map((lane) => ({
        id: cleanText(lane.id || '', 200),
        domain: cleanText(lane.domain || '', 80),
        owner: cleanText(lane.owner || '', 120),
        priority: Number(lane.priority || 0),
        target_classification: cleanText(
          lane.target_classification || 'unknown',
          40,
        ) as ScriptClass,
        target_date: cleanText(lane.target_date || '', 40),
        migration_status: cleanText(lane.migration_status || '', 80),
        notes: cleanText(lane.notes || '', 240),
      }))
    : [];
  const laneMap = new Map<string, (typeof lanes)[number]>();
  const laneFailures: Array<{ id: string; detail: string }> = [];
  for (const lane of lanes) {
    if (!lane.id) laneFailures.push({ id: 'node_burndown_lane_missing_id', detail: lane.domain || 'unknown' });
    if (!lane.domain) laneFailures.push({ id: 'node_burndown_lane_missing_domain', detail: lane.id || 'unknown' });
    if (!lane.owner) laneFailures.push({ id: 'node_burndown_lane_missing_owner', detail: lane.id || 'unknown' });
    if (!Number.isFinite(lane.priority) || lane.priority < 1 || lane.priority > 3) {
      laneFailures.push({ id: 'node_burndown_lane_invalid_priority', detail: `${lane.id}:${lane.priority}` });
    }
    if (!Number.isFinite(Date.parse(lane.target_date))) {
      laneFailures.push({ id: 'node_burndown_lane_invalid_target_date', detail: `${lane.id}:${lane.target_date}` });
    }
    if (!lane.migration_status) {
      laneFailures.push({ id: 'node_burndown_lane_missing_status', detail: lane.id || 'unknown' });
    }
    if (!['rust_native', 'node_typescript', 'npm_wrapper', 'unknown'].includes(lane.target_classification)) {
      laneFailures.push({
        id: 'node_burndown_lane_invalid_target_classification',
        detail: `${lane.id}:${lane.target_classification}`,
      });
    }
    if (laneMap.has(lane.id)) {
      laneFailures.push({ id: 'node_burndown_lane_duplicate_id', detail: lane.id });
    } else if (lane.id) {
      laneMap.set(lane.id, lane);
    }
  }

  const requiredDomains =
    Array.isArray(burnPlan.required_domains) && burnPlan.required_domains.length > 0
      ? burnPlan.required_domains.map((value) => cleanText(String(value || ''), 80)).filter(Boolean)
      : ['release', 'repair', 'topology_truth', 'recovery', 'status'];
  const domainCoverage = new Set(lanes.map((lane) => lane.domain));
  const requiredDomainsMissing = requiredDomains.filter((domain) => !domainCoverage.has(domain));
  const requiredPriorityOneMissing = requiredDomains.filter(
    (domain) => !lanes.some((lane) => lane.domain === domain && lane.priority === 1),
  );
  const operatorCriticalDomains =
    Array.isArray(burnPlan.operator_critical_domains) && burnPlan.operator_critical_domains.length > 0
      ? burnPlan.operator_critical_domains.map((value) => cleanText(String(value || ''), 80)).filter(Boolean)
      : requiredDomains;
  const operatorCriticalTargetClassification = cleanText(
    String(burnPlan.operator_critical_target_classification || 'rust_native'),
    40,
  ) as ScriptClass;
  if (!['rust_native', 'node_typescript', 'npm_wrapper', 'unknown'].includes(operatorCriticalTargetClassification)) {
    laneFailures.push({
      id: 'node_burndown_operator_critical_target_classification_invalid',
      detail: operatorCriticalTargetClassification || 'missing',
    });
  }
  const operatorCriticalPriorityCutoffDate = cleanText(
    String(burnPlan.operator_critical_priority_cutoff_date || ''),
    40,
  );
  const operatorCriticalPriorityCutoffEpoch = Number.isFinite(Date.parse(operatorCriticalPriorityCutoffDate))
    ? Date.parse(operatorCriticalPriorityCutoffDate)
    : Number.NaN;
  if (!Number.isFinite(operatorCriticalPriorityCutoffEpoch)) {
    laneFailures.push({
      id: 'node_burndown_operator_critical_priority_cutoff_date_invalid',
      detail: operatorCriticalPriorityCutoffDate || 'missing',
    });
  }
  const operatorCriticalPriorityOneMissingRust = operatorCriticalDomains.filter(
    (domain) =>
      !lanes.some(
        (lane) =>
          lane.domain === domain &&
          lane.priority === 1 &&
          lane.target_classification === operatorCriticalTargetClassification,
      ),
  );
  const operatorCriticalTargetClassificationViolations = lanes.filter(
    (lane) =>
      operatorCriticalDomains.includes(lane.domain) &&
      lane.priority === 1 &&
      lane.target_classification !== operatorCriticalTargetClassification,
  );
  const operatorCriticalCutoffViolations = lanes.filter((lane) => {
    if (!operatorCriticalDomains.includes(lane.domain)) return false;
    if (lane.priority !== 1) return false;
    const laneTargetEpoch = Number.isFinite(Date.parse(lane.target_date)) ? Date.parse(lane.target_date) : Number.NaN;
    if (!Number.isFinite(laneTargetEpoch) || !Number.isFinite(operatorCriticalPriorityCutoffEpoch)) return false;
    return laneTargetEpoch > operatorCriticalPriorityCutoffEpoch;
  });

  const allowedNodeTypescriptPrefixes =
    Array.isArray(burnPlan.allowed_node_typescript_prefixes) &&
    burnPlan.allowed_node_typescript_prefixes.length > 0
      ? burnPlan.allowed_node_typescript_prefixes.map((value) => cleanText(String(value || ''), 300)).filter(Boolean)
      : ['tests/tooling/scripts/', 'client/runtime/systems/ui/', 'client/runtime/systems/extensions/'];

  const orderedMigrationQueue =
    Array.isArray(burnPlan.ordered_migration_queue) && burnPlan.ordered_migration_queue.length > 0
      ? burnPlan.ordered_migration_queue.map((value) => cleanText(String(value || ''), 200)).filter(Boolean)
      : [];
  if (orderedMigrationQueue.length === 0) {
    laneFailures.push({
      id: 'node_burndown_ordered_migration_queue_missing',
      detail: 'ordered_migration_queue',
    });
  }
  const orderedQueueSeen = new Set<string>();
  const orderedQueueDuplicateIds: string[] = [];
  const orderedQueueUnknownIds: string[] = [];
  for (const laneId of orderedMigrationQueue) {
    if (orderedQueueSeen.has(laneId)) {
      orderedQueueDuplicateIds.push(laneId);
      continue;
    }
    orderedQueueSeen.add(laneId);
    if (!laneMap.has(laneId)) {
      orderedQueueUnknownIds.push(laneId);
    }
  }
  const operatorCriticalPriorityOneLaneIds = lanes
    .filter((lane) => operatorCriticalDomains.includes(lane.domain) && lane.priority === 1)
    .map((lane) => lane.id)
    .filter(Boolean);
  const orderedQueueMissingOperatorCriticalPriorityOne = operatorCriticalPriorityOneLaneIds.filter(
    (laneId) => !orderedQueueSeen.has(laneId),
  );

  const criticalScriptIds = Array.from(laneMap.keys());
  const nowEpoch = Date.now();

  const rows = criticalScriptIds.map((id) => {
    const lane = laneMap.get(id);
    const command = cleanText(scripts?.[id] || '', 2000);
    const classification = classifyScriptCommand(command);
    const tsEntrypointTarget = classification === 'node_typescript' ? parseTsEntrypointTarget(command) : '';
    const tsConfinementAllowed =
      classification !== 'node_typescript' ||
      (!!tsEntrypointTarget &&
        allowedNodeTypescriptPrefixes.some((prefix) => tsEntrypointTarget.startsWith(prefix)));
    const targetEpoch = Number.isFinite(Date.parse(lane?.target_date || ''))
      ? Date.parse(lane?.target_date || '')
      : Number.NaN;
    const migrationOutstanding =
      !!lane &&
      lane.target_classification !== 'unknown' &&
      classification !== lane.target_classification;
    const migrationOverdue = migrationOutstanding && Number.isFinite(targetEpoch) && nowEpoch > targetEpoch;
    return {
      id,
      command,
      classification,
      domain: lane?.domain || 'unknown',
      owner: lane?.owner || '',
      priority: lane?.priority ?? 0,
      target_classification: lane?.target_classification || 'unknown',
      target_date: lane?.target_date || '',
      migration_status: lane?.migration_status || '',
      notes: lane?.notes || '',
      ts_entrypoint_target: tsEntrypointTarget,
      ts_confinement_allowed: tsConfinementAllowed,
      migration_outstanding: migrationOutstanding,
      migration_overdue: migrationOverdue,
      exists: command.length > 0,
    };
  });

  const missing = rows.filter((row) => !row.exists);
  const rustNativeCount = rows.filter((row) => row.classification === 'rust_native').length;
  const nodeTypescriptCount = rows.filter((row) => row.classification === 'node_typescript').length;
  const npmWrapperCount = rows.filter((row) => row.classification === 'npm_wrapper').length;
  const unknownCount = rows.filter((row) => row.classification === 'unknown').length;
  const nodeDependentCount = nodeTypescriptCount + npmWrapperCount;
  const total = rows.length || 1;
  const nodeDependencyRatio = Number((nodeDependentCount / total).toFixed(4));

  const baseline = readJsonBestEffort(path.resolve(root, args.baselinePath));
  const baselineNodeRatio = Number(baseline.payload?.summary?.node_dependency_ratio ?? Number.NaN);
  const baselineAvailable = baseline.ok;
  const nodeDependencyRegression =
    baselineAvailable && Number.isFinite(baselineNodeRatio) && nodeDependencyRatio > baselineNodeRatio;

  const failures = []
    .concat(laneFailures)
    .concat(
      requiredDomainsMissing.map((domain) => ({
        id: 'node_burndown_required_domain_missing',
        detail: domain,
      })),
    )
    .concat(
      requiredPriorityOneMissing.map((domain) => ({
        id: 'node_burndown_required_priority_one_missing',
        detail: domain,
      })),
    )
    .concat(
      operatorCriticalPriorityOneMissingRust.map((domain) => ({
        id: 'node_burndown_operator_critical_priority_one_missing_rust_target',
        detail: domain,
      })),
    )
    .concat(
      operatorCriticalTargetClassificationViolations.map((lane) => ({
        id: 'node_burndown_operator_critical_target_classification_violation',
        detail: `${lane.id}:domain=${lane.domain};priority=${lane.priority};target=${lane.target_classification};required=${operatorCriticalTargetClassification}`,
      })),
    )
    .concat(
      operatorCriticalCutoffViolations.map((lane) => ({
        id: 'node_burndown_operator_critical_cutoff_violation',
        detail: `${lane.id}:domain=${lane.domain};target_date=${lane.target_date};cutoff=${operatorCriticalPriorityCutoffDate}`,
      })),
    )
    .concat(
      orderedQueueDuplicateIds.map((laneId) => ({
        id: 'node_burndown_ordered_migration_queue_duplicate_id',
        detail: laneId,
      })),
    )
    .concat(
      orderedQueueUnknownIds.map((laneId) => ({
        id: 'node_burndown_ordered_migration_queue_unknown_id',
        detail: laneId,
      })),
    )
    .concat(
      orderedQueueMissingOperatorCriticalPriorityOne.map((laneId) => ({
        id: 'node_burndown_ordered_migration_queue_missing_operator_priority_one_lane',
        detail: laneId,
      })),
    )
    .concat(
      missing.map((row) => ({
        id: 'critical_script_missing',
        detail: row.id,
      })),
    )
    .concat(
      rows
        .filter((row) => row.classification === 'node_typescript' && !row.ts_confinement_allowed)
        .map((row) => ({
          id: 'node_typescript_confinement_violation',
          detail: `${row.id}:${row.ts_entrypoint_target || 'missing_target_path'}`,
        })),
    )
    .concat(
      rows
        .filter((row) => row.migration_overdue)
        .map((row) => ({
          id: 'node_burndown_target_overdue',
          detail: `${row.id}:target_date=${row.target_date};target=${row.target_classification};actual=${row.classification}`,
        })),
    )
    .concat(
      nodeDependencyRegression
        ? [
            {
              id: 'node_dependency_ratio_regression',
              detail: `current=${nodeDependencyRatio};baseline=${baselineNodeRatio}`,
            },
          ]
        : [],
    );

  const payload = {
    ok: failures.length === 0,
    type: 'node_critical_path_inventory',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    package_path: args.packagePath,
    baseline_path: args.baselinePath,
    summary: {
      critical_scripts_total: rows.length,
      critical_scripts_missing: missing.length,
      rust_native_count: rustNativeCount,
      node_typescript_count: nodeTypescriptCount,
      npm_wrapper_count: npmWrapperCount,
      unknown_count: unknownCount,
      node_dependency_ratio: nodeDependencyRatio,
      migration_outstanding_count: rows.filter((row) => row.migration_outstanding).length,
      migration_overdue_count: rows.filter((row) => row.migration_overdue).length,
      required_domains_missing_count: requiredDomainsMissing.length,
      required_priority_one_missing_count: requiredPriorityOneMissing.length,
      operator_critical_priority_one_missing_rust_count: operatorCriticalPriorityOneMissingRust.length,
      operator_critical_target_classification_violation_count: operatorCriticalTargetClassificationViolations.length,
      operator_critical_cutoff_violation_count: operatorCriticalCutoffViolations.length,
      ordered_migration_queue_count: orderedMigrationQueue.length,
      ordered_migration_queue_duplicate_id_count: orderedQueueDuplicateIds.length,
      ordered_migration_queue_unknown_id_count: orderedQueueUnknownIds.length,
      ordered_migration_queue_missing_operator_priority_one_count: orderedQueueMissingOperatorCriticalPriorityOne.length,
      ts_confinement_violation_count: rows.filter(
        (row) => row.classification === 'node_typescript' && !row.ts_confinement_allowed,
      ).length,
      baseline_available: baselineAvailable,
      baseline_node_dependency_ratio: baselineAvailable && Number.isFinite(baselineNodeRatio) ? baselineNodeRatio : null,
      node_dependency_ratio_regression: nodeDependencyRegression,
    },
    burn_down_plan: {
      schema_id: cleanText((burnPlan.schema_id as string) || '', 120),
      schema_version: cleanText((burnPlan.schema_version as string) || '', 40),
      plan_path: args.burnPlanPath,
      lane_count: lanes.length,
      required_domains: requiredDomains,
      operator_critical_domains: operatorCriticalDomains,
      operator_critical_target_classification: operatorCriticalTargetClassification,
      operator_critical_priority_cutoff_date: operatorCriticalPriorityCutoffDate,
      allowed_node_typescript_prefixes: allowedNodeTypescriptPrefixes,
      ordered_migration_queue: orderedMigrationQueue,
    },
    rows,
    failures,
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
