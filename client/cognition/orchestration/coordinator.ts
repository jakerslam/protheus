#!/usr/bin/env node
'use strict';

const { stableHash } = require('./cli_shared.ts');
const { STATUS_ORDER, SEVERITY_ORDER, validateFinding, normalizeFinding } = require('./schema_runtime.ts');
const { appendFinding, loadScratchpad, writeScratchpad } = require('./scratchpad.ts');
const { maybeCheckpoint, handleTimeout } = require('./checkpoint.ts');
const { detectScopeOverlaps, classifyFindingsByScope } = require('./scope.ts');
const { ensureTaskGroup } = require('./taskgroup.ts');
const { trackBatchCompletion } = require('./completion.ts');
const { runCoordinatorCli } = require('./coordinator_cli.ts');

function partitionWork(items, agentCount = 1) {
  const normalized = Array.isArray(items) ? items.slice() : [];
  const count = Math.max(1, Number.isFinite(Number(agentCount)) ? Number(agentCount) : 1);
  const partitions = Array.from({ length: count }, (_, idx) => ({
    agent_id: `agent-${idx + 1}`,
    items: []
  }));

  normalized.forEach((item, index) => {
    partitions[index % count].items.push(item);
  });
  return partitions;
}

function mergeEvidence(rows) {
  const seen = new Set();
  const merged = [];
  for (const row of rows) {
    if (!row || typeof row !== 'object') continue;
    const key = `${row.type || ''}:${row.value || ''}:${row.source || ''}`;
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(row);
  }
  return merged;
}

function mergeFindings(findings) {
  const input = Array.isArray(findings) ? findings : [];
  const buckets = new Map();
  const dropped = [];

  for (const raw of input) {
    const finding = normalizeFinding(raw);
    const validation = validateFinding(finding);
    if (!validation.ok) {
      dropped.push({
        reason_code: validation.reason_code,
        finding
      });
      continue;
    }

    const key = finding.item_id;
    const existing = buckets.get(key);
    if (!existing) {
      buckets.set(key, Object.assign({}, finding));
      continue;
    }

    if ((SEVERITY_ORDER[finding.severity] || 0) > (SEVERITY_ORDER[existing.severity] || 0)) {
      existing.severity = finding.severity;
    }
    if ((STATUS_ORDER[finding.status] || 0) > (STATUS_ORDER[existing.status] || 0)) {
      existing.status = finding.status;
    }
    existing.evidence = mergeEvidence([].concat(existing.evidence || [], finding.evidence || []));
    existing.timestamp = new Date(Math.max(Date.parse(existing.timestamp), Date.parse(finding.timestamp))).toISOString();
    existing.summary = [existing.summary, finding.summary].filter(Boolean).join(' | ').trim() || undefined;
  }

  const merged = Array.from(buckets.values()).sort((left, right) => {
    const severityDelta = (SEVERITY_ORDER[right.severity] || 0) - (SEVERITY_ORDER[left.severity] || 0);
    if (severityDelta !== 0) return severityDelta;
    return String(left.item_id).localeCompare(String(right.item_id));
  });

  return {
    merged,
    dropped,
    deduped_count: input.length - merged.length - dropped.length
  };
}

function assignScopesToPartitions(partitions, normalizedScopes = []) {
  const out = [];
  const scopes = Array.isArray(normalizedScopes) ? normalizedScopes : [];
  for (let index = 0; index < partitions.length; index += 1) {
    const partition = partitions[index];
    const scope = scopes.length > 0 ? scopes[index % scopes.length] : null;
    out.push(Object.assign({}, partition, {
      scope
    }));
  }
  return out;
}

function scopeMapByAgent(partitions = []) {
  const map = new Map();
  for (const partition of partitions) {
    if (!partition || typeof partition !== 'object') continue;
    if (!partition.agent_id || !partition.scope) continue;
    map.set(String(partition.agent_id), partition.scope);
  }
  return map;
}

function applyScopeFiltering(findings = [], scopeByAgent = new Map()) {
  const kept = [];
  const violations = [];

  for (const raw of Array.isArray(findings) ? findings : []) {
    const finding = normalizeFinding(raw);
    const agentId = String(finding.agent_id || (finding.metadata && finding.metadata.agent_id) || '').trim();
    if (!agentId || !scopeByAgent.has(agentId)) {
      kept.push(finding);
      continue;
    }

    const scope = scopeByAgent.get(agentId);
    const classified = classifyFindingsByScope([finding], scope, agentId);
    if (!classified.ok) {
      violations.push({
        reason_code: 'scope_classification_failed',
        agent_id: agentId,
        item_id: finding.item_id,
        location: finding.location
      });
      continue;
    }

    if (classified.in_scope.length > 0) kept.push(classified.in_scope[0]);
    if (classified.violations.length > 0) violations.push(...classified.violations);
  }

  return {
    kept,
    violations
  };
}

function runCoordinator(input = {}) {
  const taskId = String(input.task_id || '').trim();
  if (!taskId) {
    return {
      ok: false,
      type: 'orchestration_coordinator',
      reason_code: 'missing_task_id'
    };
  }

  const auditId = String(input.audit_id || `audit-${stableHash(taskId)}`);
  const items = Array.isArray(input.items) ? input.items : [];
  const findings = Array.isArray(input.findings) ? input.findings : [];
  const agentCount = Math.max(1, Number.isFinite(Number(input.agent_count)) ? Number(input.agent_count) : 1);
  const scratchpadOptions = input.root_dir ? { rootDir: String(input.root_dir) } : {};

  const scopeCheck = detectScopeOverlaps(Array.isArray(input.scopes) ? input.scopes : []);
  if (!scopeCheck.ok) {
    return {
      ok: false,
      type: 'orchestration_coordinator',
      reason_code: scopeCheck.reason_code,
      overlaps: scopeCheck.overlaps,
      scope_id: scopeCheck.scope_id || null
    };
  }

  const partitions = assignScopesToPartitions(partitionWork(items, agentCount), scopeCheck.normalized_scopes || []);
  const scopeByAgent = scopeMapByAgent(partitions);

  const taskGroup = ensureTaskGroup({
    task_group_id: input.task_group_id,
    task_type: input.task_type || 'audit',
    coordinator_session: input.coordinator_session || null,
    agent_count: partitions.length,
    agents: partitions.map((partition) => ({
      agent_id: partition.agent_id,
      status: 'running',
      details: partition.scope ? { scope_id: partition.scope.scope_id } : {}
    }))
  }, scratchpadOptions);
  if (!taskGroup.ok) {
    return Object.assign({}, taskGroup, {
      ok: false,
      type: 'orchestration_coordinator',
      reason_code: taskGroup.reason_code || 'task_group_creation_failed'
    });
  }

  const findingsWithAudit = findings.map((finding) => {
    if (!finding || typeof finding !== 'object' || Array.isArray(finding)) {
      return { audit_id: auditId };
    }
    return Object.assign({ audit_id: auditId }, finding);
  });
  const filtered = applyScopeFiltering(findingsWithAudit, scopeByAgent);
  const merged = mergeFindings(filtered.kept);

  const updatedProgress = {
    processed: merged.merged.length,
    total: items.length
  };

  writeScratchpad(taskId, {
    progress: updatedProgress
  }, scratchpadOptions);

  for (const finding of merged.merged) {
    appendFinding(taskId, Object.assign({ audit_id: auditId }, finding), scratchpadOptions);
  }

  maybeCheckpoint(taskId, {
    processed_count: updatedProgress.processed,
    total_count: updatedProgress.total,
    now_ms: Date.now()
  }, scratchpadOptions);

  const completion = trackBatchCompletion(
    taskGroup.task_group.task_group_id,
    partitions.map((partition) => ({
      agent_id: partition.agent_id,
      status: 'done',
      details: {
        processed_count: partition.items.length,
        scope_id: partition.scope ? partition.scope.scope_id : undefined
      }
    })),
    scratchpadOptions
  );

  if (!completion.ok) {
    return {
      ok: false,
      type: 'orchestration_coordinator',
      reason_code: completion.reason_code || 'completion_tracking_failed',
      task_id: taskId,
      audit_id: auditId
    };
  }

  return {
    ok: true,
    type: 'orchestration_coordinator',
    task_id: taskId,
    audit_id: auditId,
    task_group_id: taskGroup.task_group.task_group_id,
    partition_count: partitions.length,
    partitions,
    findings_total: findings.length,
    findings_in_scope: filtered.kept.length,
    findings_merged: merged.merged.length,
    findings_deduped: merged.deduped_count,
    findings_dropped: merged.dropped.length,
    scope_violation_count: filtered.violations.length,
    scope_violations: filtered.violations,
    completion_summary: completion.summary,
    notification: completion.notification,
    report: {
      findings: merged.merged,
      dropped: merged.dropped
    }
  };
}

function run(argv = process.argv.slice(2)) {
  return runCoordinatorCli(argv, {
    runCoordinator,
    partitionWork,
    assignScopesToPartitions,
    detectScopeOverlaps,
    loadScratchpad,
    handleTimeout
  });
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  partitionWork,
  mergeFindings,
  runCoordinator,
  run
};
