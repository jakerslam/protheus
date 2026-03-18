#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const DEFAULT_POLICY_PATH = 'client/runtime/config/benchmark_sanity_policy.json';
const OUT_JSON = 'core/local/artifacts/benchmark_sanity_gate_current.json';
const OUT_MD = 'local/workspace/reports/BENCHMARK_SANITY_GATE_CURRENT.md';

function parseArgs(argv) {
  const out = { strict: false, policyPath: DEFAULT_POLICY_PATH };
  for (const raw of argv) {
    const arg = String(raw ?? '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(value);
      continue;
    }
    if (arg.startsWith('--policy=')) {
      out.policyPath = arg.slice('--policy='.length).trim() || DEFAULT_POLICY_PATH;
      continue;
    }
  }
  return out;
}

function readJson(relPath) {
  return JSON.parse(readFileSync(resolve(relPath), 'utf8'));
}

function ensureParent(path) {
  mkdirSync(dirname(resolve(path)), { recursive: true });
}

function asFiniteNumber(value) {
  const num = Number(value);
  if (!Number.isFinite(num)) return null;
  return num;
}

function normalizeProjects(report) {
  const rawProjects = report?.projects;
  if (rawProjects && typeof rawProjects === 'object') {
    const normalized = { ...rawProjects };
    if (!normalized['InfRing (rich)'] && normalized.OpenClaw) {
      normalized['InfRing (rich)'] = normalized.OpenClaw;
    }
    return normalized;
  }

  if (report?.type === 'competitive_benchmark_matrix_stabilized' && report?.medians) {
    const medians = report.medians;
    return {
      'InfRing (rich)': {
        cold_start_ms: asFiniteNumber(medians?.rich?.cold_start_ms),
        idle_memory_mb: asFiniteNumber(medians?.rich?.idle_memory_mb),
        install_size_mb: asFiniteNumber(medians?.rich?.install_size_mb),
        tasks_per_sec: asFiniteNumber(medians?.rich?.tasks_per_sec),
      },
      'InfRing (pure)': {
        cold_start_ms: asFiniteNumber(medians?.pure?.cold_start_ms),
        idle_memory_mb: asFiniteNumber(medians?.pure?.idle_memory_mb),
        install_size_mb: asFiniteNumber(medians?.pure?.install_size_mb),
        tasks_per_sec: asFiniteNumber(medians?.pure?.tasks_per_sec),
      },
      'InfRing (tiny-max)': {
        cold_start_ms: asFiniteNumber(medians?.tiny_max?.cold_start_ms),
        idle_memory_mb: asFiniteNumber(medians?.tiny_max?.idle_memory_mb),
        install_size_mb: asFiniteNumber(medians?.tiny_max?.install_size_mb),
        tasks_per_sec: asFiniteNumber(medians?.tiny_max?.tasks_per_sec),
      },
    };
  }

  return {};
}

function reportRuntimeSourcePath(report, reportPath, policy) {
  const override = String(policy?.runtime_source_report_path || '').trim();
  if (override) return override;
  const hinted = String(report?.latest_live_report || '').trim();
  if (hinted) return hinted;
  return reportPath;
}

function reportType(report) {
  return String(report?.type || 'competitive_benchmark_matrix_live');
}

function getProjectMetric(projects, projectName, metric) {
  const project = projects?.[projectName];
  if (!project || typeof project !== 'object') return null;
  return asFiniteNumber(project?.[metric]);
}

function metricRow(projects, projectName, metric) {
  return {
    project: projectName,
    metric,
    value: getProjectMetric(projects, projectName, metric),
  };
}

function sanitizeMeasured(projects, policy) {
  const rows = [];
  for (const projectName of policy.required_projects ?? []) {
    for (const metric of policy.required_metrics ?? []) {
      rows.push(metricRow(projects, projectName, metric));
    }
  }
  return rows;
}

function checkRequiredRows(rows) {
  const violations = [];
  for (const row of rows) {
    if (row.value == null) {
      violations.push(
        `missing_or_non_numeric_metric:${row.project}:${row.metric}`,
      );
    }
  }
  return violations;
}

function checkBounds(rows, policy) {
  const violations = [];
  for (const row of rows) {
    if (row.value == null) continue;
    const bounds = policy?.bounds?.[row.metric];
    if (!bounds || typeof bounds !== 'object') continue;
    const min = asFiniteNumber(bounds.min);
    const max = asFiniteNumber(bounds.max);
    if (min != null && row.value < min) {
      violations.push(`metric_below_min:${row.project}:${row.metric}:${row.value}<${min}`);
    }
    if (max != null && row.value > max) {
      violations.push(`metric_above_max:${row.project}:${row.metric}:${row.value}>${max}`);
    }
  }
  return violations;
}

function runtimeSourcePayload(report) {
  return (
    report?.projects?.OpenClaw?.runtime_metric_source ??
    report?.projects?.['InfRing (rich)']?.runtime_metric_source ??
    null
  );
}

function checkRuntimeSource(runtimeReport, policy) {
  const violations = [];
  const source = runtimeSourcePayload(runtimeReport);
  const required = policy?.openclaw_required_runtime_source_keys ?? [];
  for (const key of required) {
    const value = source?.[key];
    const missing =
      value == null || (typeof value === 'string' && value.trim().length === 0);
    if (missing) {
      violations.push(`openclaw_runtime_source_missing:${key}`);
    }
  }
  return violations;
}

function computeStepRatio(a, b) {
  if (a == null || b == null || a <= 0 || b <= 0) return null;
  const high = Math.max(a, b);
  const low = Math.min(a, b);
  return high / low;
}

function throughputSource(runtimeReport) {
  return runtimeSourcePayload(runtimeReport)?.tasks_source ?? null;
}

function throughputSourceChanged(runtimeReport, previous) {
  const current = throughputSource(runtimeReport);
  const prior = previous?.shared_throughput_source ?? null;
  if (!current) return false;
  if (!prior) return true;
  return current !== prior;
}

function reportSourceChanged(previous, reportPath, runtimeSourcePath, currentReportType) {
  const priorReportPath = String(previous?.source_report || '').trim();
  const priorRuntimeSourcePath = String(previous?.runtime_source_report || '').trim();
  const priorReportType = String(previous?.report_type || '').trim();
  return (
    (priorReportPath && priorReportPath !== reportPath) ||
    (priorRuntimeSourcePath && priorRuntimeSourcePath !== runtimeSourcePath) ||
    (priorReportType && priorReportType !== currentReportType)
  );
}

function checkStepChanges(rows, policy, previous, runtimeReport, reportPath, runtimeSourcePath, currentReportType) {
  const violations = [];
  if (!previous || typeof previous !== 'object') {
    return violations;
  }
  if (reportSourceChanged(previous, reportPath, runtimeSourcePath, currentReportType)) {
    return violations;
  }
  const exemptions = new Set(
    (policy?.step_change_exemptions ?? [])
      .map((row) => `${String(row?.project || '').trim()}::${String(row?.metric || '').trim()}`)
      .filter((row) => row !== '::'),
  );
  const projectMap = previous.projects ?? {};
  const multipliers = policy?.max_step_multiplier ?? {};
  for (const row of rows) {
    if (row.value == null) continue;
    if (exemptions.has(`${row.project}::${row.metric}`)) continue;
    if (row.metric === 'tasks_per_sec' && throughputSourceChanged(runtimeReport, previous)) {
      continue;
    }
    const prev = asFiniteNumber(projectMap?.[row.project]?.[row.metric]);
    if (prev == null) continue;
    const ratio = computeStepRatio(prev, row.value);
    if (ratio == null) continue;
    const maxMultiplier = asFiniteNumber(multipliers[row.metric]);
    if (maxMultiplier != null && ratio > maxMultiplier) {
      violations.push(
        `step_change_exceeds_limit:${row.project}:${row.metric}:x${ratio.toFixed(3)}>${maxMultiplier}`,
      );
    }
  }
  return violations;
}

function latestStateRows(rows) {
  const projects = {};
  for (const row of rows) {
    if (!projects[row.project]) projects[row.project] = {};
    projects[row.project][row.metric] = row.value;
  }
  return projects;
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Benchmark Sanity Gate (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Report: ${payload.report_path}`);
  lines.push(`Runtime Source Report: ${payload.runtime_source_report_path}`);
  lines.push(`Report Type: ${payload.report_type}`);
  lines.push(`Pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- required_rows: ${payload.summary.required_rows}`);
  lines.push(`- measured_rows: ${payload.summary.measured_rows}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- strict: ${payload.summary.strict}`);
  lines.push('');
  if (payload.violations.length > 0) {
    lines.push('## Violations');
    for (const violation of payload.violations) {
      lines.push(`- ${violation}`);
    }
    lines.push('');
  }
  lines.push('## Measured Rows');
  lines.push('| Project | Metric | Value |');
  lines.push('| --- | --- | ---: |');
  for (const row of payload.rows) {
    const value = row.value == null ? 'null' : String(row.value);
    lines.push(`| ${row.project} | ${row.metric} | ${value} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policy = readJson(args.policyPath);
  const reportPath = policy.report_path;
  const statePath = policy.state_path;

  if (!reportPath || !existsSync(resolve(reportPath))) {
    const out = {
      ok: false,
      type: 'benchmark_sanity_gate',
      generated_at: new Date().toISOString(),
      error: `benchmark_report_missing:${reportPath}`,
    };
    console.error(JSON.stringify(out, null, 2));
    process.exit(1);
  }

  const report = readJson(reportPath);
  const runtimeSourcePath = reportRuntimeSourcePath(report, reportPath, policy);
  const runtimeReport =
    runtimeSourcePath && existsSync(resolve(runtimeSourcePath)) ? readJson(runtimeSourcePath) : report;
  const projects = normalizeProjects(report);
  const rows = sanitizeMeasured(projects, policy);
  const previous = statePath && existsSync(resolve(statePath)) ? readJson(statePath) : null;
  const currentReportType = reportType(report);

  const violations = [
    ...checkRequiredRows(rows),
    ...checkBounds(rows, policy),
    ...checkRuntimeSource(runtimeReport, policy),
    ...checkStepChanges(rows, policy, previous, runtimeReport, reportPath, runtimeSourcePath, currentReportType),
  ];

  const payload = {
    ok: true,
    type: 'benchmark_sanity_gate',
    generated_at: new Date().toISOString(),
    policy_path: args.policyPath,
    report_path: reportPath,
    runtime_source_report_path: runtimeSourcePath,
    report_type: currentReportType,
    state_path: statePath,
    summary: {
      strict: args.strict,
      required_rows: rows.length,
      measured_rows: rows.filter((row) => row.value != null).length,
      violations: violations.length,
      pass: violations.length === 0,
    },
    rows,
    violations,
  };

  ensureParent(OUT_JSON);
  ensureParent(OUT_MD);
  writeFileSync(resolve(OUT_JSON), `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(resolve(OUT_MD), toMarkdown(payload));

  if (violations.length === 0 && statePath) {
    ensureParent(statePath);
    writeFileSync(
      resolve(statePath),
      `${JSON.stringify(
        {
          generated_at: payload.generated_at,
          source_report: reportPath,
          runtime_source_report: runtimeSourcePath,
          report_type: currentReportType,
          shared_throughput_source: throughputSource(runtimeReport),
          projects: latestStateRows(rows),
        },
        null,
        2,
      )}\n`,
    );
  }

  if (args.strict && violations.length > 0) {
    console.error(
      JSON.stringify(
        {
          ok: false,
          type: payload.type,
          out_json: OUT_JSON,
          summary: payload.summary,
          violations,
        },
        null,
        2,
      ),
    );
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: payload.type,
        out_json: OUT_JSON,
        out_markdown: OUT_MD,
        summary: payload.summary,
      },
      null,
      2,
    ),
  );
}

main();
