#!/usr/bin/env node
'use strict';

const crypto = require('node:crypto');
const { STATUS_ORDER, SEVERITY_ORDER, validateFinding, normalizeFinding } = require('./schema_runtime.ts');
const { appendFinding, loadScratchpad, writeScratchpad } = require('./scratchpad.ts');
const { maybeCheckpoint, handleTimeout } = require('./checkpoint.ts');

function parseArgs(argv = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) flags[body.slice(0, eq)] = body.slice(eq + 1);
      else flags[body] = '1';
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function stableHash(input) {
  return crypto.createHash('sha256').update(String(input || '')).digest('hex').slice(0, 12);
}

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
  const partitions = partitionWork(items, agentCount);
  const merged = mergeFindings(findings);

  const scratchpad = loadScratchpad(taskId, scratchpadOptions).scratchpad;
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

  return {
    ok: true,
    type: 'orchestration_coordinator',
    task_id: taskId,
    audit_id: auditId,
    partition_count: partitions.length,
    partitions,
    findings_total: findings.length,
    findings_merged: merged.merged.length,
    findings_deduped: merged.deduped_count,
    findings_dropped: merged.dropped.length,
    report: {
      findings: merged.merged,
      dropped: merged.dropped
    }
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'run').trim().toLowerCase();

  if (command === 'run') {
    const taskId = String(parsed.flags['task-id'] || parsed.flags.task_id || parsed.positional[1] || '').trim();
    const auditId = String(parsed.flags['audit-id'] || parsed.flags.audit_id || '').trim();
    const rootDir = String(parsed.flags['root-dir'] || parsed.flags.root_dir || '').trim();
    const agentCount = Number(parsed.flags['agent-count'] || parsed.flags.agent_count || 1);

    let items = [];
    let findings = [];
    try {
      items = JSON.parse(String(parsed.flags['items-json'] || parsed.flags.items_json || '[]'));
    } catch {
      return {
        ok: false,
        type: 'orchestration_coordinator',
        reason_code: 'invalid_items_json'
      };
    }
    try {
      findings = JSON.parse(String(parsed.flags['findings-json'] || parsed.flags.findings_json || '[]'));
    } catch {
      return {
        ok: false,
        type: 'orchestration_coordinator',
        reason_code: 'invalid_findings_json'
      };
    }

    const timeout = String(parsed.flags.timeout || '0').trim() === '1';
    if (timeout) {
      return handleTimeout(taskId, {
        processed_count: Number(parsed.flags.processed || 0),
        total_count: Array.isArray(items) ? items.length : 0,
        partial_results: Array.isArray(findings) ? findings : [],
        retry_count: Number(parsed.flags['retry-count'] || parsed.flags.retry_count || 0),
        now_ms: Date.now()
      }, rootDir ? { rootDir } : {});
    }

    return runCoordinator({
      task_id: taskId,
      audit_id: auditId,
      agent_count: agentCount,
      items,
      findings,
      root_dir: rootDir || undefined
    });
  }

  if (command === 'partition') {
    let items = [];
    try {
      items = JSON.parse(String(parsed.flags['items-json'] || parsed.flags.items_json || '[]'));
    } catch {
      return {
        ok: false,
        type: 'orchestration_partition',
        reason_code: 'invalid_items_json'
      };
    }
    const agentCount = Number(parsed.flags['agent-count'] || parsed.flags.agent_count || 1);
    return {
      ok: true,
      type: 'orchestration_partition',
      partitions: partitionWork(items, agentCount)
    };
  }

  return {
    ok: false,
    type: 'orchestration_coordinator',
    reason_code: `unsupported_command:${command}`,
    commands: ['run', 'partition']
  };
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
