#!/usr/bin/env node
'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function findRepoRoot(startDir) {
  let dir = path.resolve(startDir || process.cwd());
  while (true) {
    const cargo = path.join(dir, 'Cargo.toml');
    const layer0 = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    const legacy = path.join(dir, 'crates', 'ops', 'Cargo.toml');
    if (fs.existsSync(cargo) && (fs.existsSync(layer0) || fs.existsSync(legacy))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return path.resolve(startDir || process.cwd());
    }
    dir = parent;
  }
}

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const ROOT = findRepoRoot(CLIENT_ROOT);

const DEFAULT_BASELINE_STATE_PATH = path.join(CLIENT_ROOT, 'state', 'ops', 'f100_enterprise_baseline_gate', 'latest.json');
const DEFAULT_COVERAGE_SUMMARY_PATH = path.join(ROOT, 'coverage', 'combined-summary.json');
const DEFAULT_COVERAGE_FALLBACK_PATH = path.join(CLIENT_ROOT, 'docs', 'reports', 'coverage_baseline_2026-03-06.json');
const DEFAULT_STATUS_DOC_PATH = path.join(CLIENT_ROOT, 'docs', 'ops', 'F100_A_PLUS_READINESS_STATUS.md');
const DEFAULT_STATE_PATH = path.join(CLIENT_ROOT, 'state', 'ops', 'f100_a_plus_readiness_gate', 'latest.json');

function parseArgs(argv) {
  const args = {
    command: 'run',
    strict: false,
    write: true
  };
  const parts = argv.slice(2);
  if (parts.length > 0 && !parts[0].startsWith('--')) {
    args.command = parts[0];
  }
  for (const raw of parts) {
    if (!raw.startsWith('--')) {
      continue;
    }
    const [key, value = '1'] = raw.slice(2).split('=');
    if (key === 'strict') {
      args.strict = value === '1' || value === 'true';
    } else if (key === 'write') {
      args.write = value === '1' || value === 'true';
    }
  }
  return args;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function hash(payload) {
  return crypto.createHash('sha256').update(payload).digest('hex');
}

function fileExists(relativePath) {
  return fs.existsSync(path.join(ROOT, relativePath));
}

function fileContains(relativePath, pattern) {
  const absolute = path.join(ROOT, relativePath);
  if (!fs.existsSync(absolute)) {
    return false;
  }
  const body = fs.readFileSync(absolute, 'utf8');
  return body.includes(pattern);
}

function parseCoveragePct() {
  const primary = process.env.F100_A_PLUS_COVERAGE_PATH || DEFAULT_COVERAGE_SUMMARY_PATH;
  const fallback = process.env.F100_A_PLUS_COVERAGE_FALLBACK_PATH || DEFAULT_COVERAGE_FALLBACK_PATH;

  const sources = [primary, fallback];
  for (const source of sources) {
    if (!fs.existsSync(source)) {
      continue;
    }
    const payload = readJson(source);
    if (typeof payload.combined_lines_pct === 'number') {
      return { value: payload.combined_lines_pct, source: rel(source) };
    }
    if (typeof payload.lines?.pct === 'number') {
      return { value: payload.lines.pct, source: rel(source) };
    }
  }
  return { value: null, source: null };
}

function countSemanticTags() {
  const run = spawnSync('git', ['tag', '-l', 'v*'], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  if (run.status !== 0) {
    return { count: 0, source: 'git_tag_scan_failed' };
  }
  const tags = String(run.stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  return { count: tags.length, source: 'git tag -l v*' };
}

function evaluateChecks() {
  const minCoverage = Number(process.env.F100_A_PLUS_MIN_COVERAGE || 90);
  const minTags = Number(process.env.F100_A_PLUS_MIN_TAGS || 9);
  const baselineStatePath = process.env.F100_A_PLUS_BASELINE_STATE_PATH || DEFAULT_BASELINE_STATE_PATH;

  const checks = [];

  let baselineOk = false;
  if (fs.existsSync(baselineStatePath)) {
    try {
      baselineOk = readJson(baselineStatePath)?.ok === true;
    } catch {
      baselineOk = false;
    }
  }
  checks.push({
    id: 'enterprise_baseline_contract_pass',
    ok: baselineOk,
    expected: 'baseline.ok == true',
    actual: baselineOk ? 'true' : 'false',
    source: rel(baselineStatePath)
  });

  const coverage = parseCoveragePct();
  checks.push({
    id: 'combined_coverage_threshold',
    ok: typeof coverage.value === 'number' && coverage.value >= minCoverage,
    expected: `combined_lines_pct >= ${minCoverage}`,
    actual: coverage.value == null ? 'missing' : String(coverage.value),
    source: coverage.source || 'coverage summary missing'
  });

  const tags = countSemanticTags();
  checks.push({
    id: 'semantic_release_cadence',
    ok: tags.count >= minTags,
    expected: `v* tags >= ${minTags}`,
    actual: String(tags.count),
    source: tags.source
  });

  checks.push({
    id: 'release_slsa_attestation_enabled',
    ok: fileContains('.github/workflows/release-security-artifacts.yml', 'actions/attest-build-provenance@v2'),
    expected: 'release workflow contains actions/attest-build-provenance@v2',
    actual: fileContains('.github/workflows/release-security-artifacts.yml', 'actions/attest-build-provenance@v2') ? 'present' : 'missing',
    source: '.github/workflows/release-security-artifacts.yml'
  });

  checks.push({
    id: 'support_envelope_template_present',
    ok: fileExists('docs/client/ENTERPRISE_SUPPORT_ENVELOPE_TEMPLATE.md'),
    expected: 'support template exists',
    actual: fileExists('docs/client/ENTERPRISE_SUPPORT_ENVELOPE_TEMPLATE.md') ? 'present' : 'missing',
    source: 'docs/client/ENTERPRISE_SUPPORT_ENVELOPE_TEMPLATE.md'
  });

  checks.push({
    id: 'case_study_template_present',
    ok: fileExists('docs/client/REFERENCE_CUSTOMER_CASE_STUDY_TEMPLATE.md'),
    expected: 'case study template exists',
    actual: fileExists('docs/client/REFERENCE_CUSTOMER_CASE_STUDY_TEMPLATE.md') ? 'present' : 'missing',
    source: 'docs/client/REFERENCE_CUSTOMER_CASE_STUDY_TEMPLATE.md'
  });

  checks.push({
    id: 'legal_packet_checklist_present',
    ok: fileExists('docs/client/LEGAL_ENTERPRISE_PACKET_CHECKLIST.md'),
    expected: 'legal packet checklist exists',
    actual: fileExists('docs/client/LEGAL_ENTERPRISE_PACKET_CHECKLIST.md') ? 'present' : 'missing',
    source: 'docs/client/LEGAL_ENTERPRISE_PACKET_CHECKLIST.md'
  });

  const humanActionsPath = 'docs/client/HUMAN_ONLY_ACTIONS.md';
  const hmanTrackers = ['HMAN-026', 'HMAN-027', 'HMAN-028', 'HMAN-029', 'HMAN-030', 'HMAN-031', 'HMAN-032', 'HMAN-033', 'HMAN-034', 'HMAN-035'];
  const hmanMissing = hmanTrackers.filter((token) => !fileContains(humanActionsPath, token));
  checks.push({
    id: 'human_owner_blockers_registered',
    ok: hmanMissing.length === 0,
    expected: `all markers present: ${hmanTrackers.join(', ')}`,
    actual: hmanMissing.length === 0 ? 'all_markers_present' : `missing:${hmanMissing.join(',')}`,
    source: humanActionsPath
  });

  return checks;
}

function renderMarkdown(report) {
  const lines = [];
  lines.push('# F100 A+ Readiness Status');
  lines.push('');
  lines.push(`Generated: ${report.generated_at}`);
  lines.push('');
  lines.push('| Check | Status | Expected | Actual | Source |');
  lines.push('|---|---|---|---|---|');
  for (const check of report.checks) {
    lines.push(
      `| \`${check.id}\` | ${check.ok ? 'PASS' : 'FAIL'} | \`${check.expected}\` | \`${check.actual}\` | \`${check.source}\` |`
    );
  }
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Total checks: ${report.summary.total_checks}`);
  lines.push(`- Passed checks: ${report.summary.passed_checks}`);
  lines.push(`- Failed checks: ${report.summary.failed_checks}`);
  lines.push(`- Overall status: ${report.ok ? 'PASS' : 'FAIL'}`);
  lines.push(`- Receipt hash: \`${report.receipt_hash}\``);
  lines.push('');
  lines.push('## Note');
  lines.push('');
  lines.push('A FAIL here does not imply runtime insecurity; it means Fortune-100 A+ procurement proof requirements are still incomplete.');
  lines.push('');
  return lines.join('\n');
}

function run(args) {
  const checks = evaluateChecks();
  const passed = checks.filter((row) => row.ok).length;
  const failed = checks.length - passed;

  const report = {
    schema_id: 'f100_a_plus_readiness_gate_result',
    schema_version: '1.0.0',
    generated_at: new Date().toISOString(),
    ok: failed === 0,
    summary: {
      total_checks: checks.length,
      passed_checks: passed,
      failed_checks: failed
    },
    checks
  };

  report.receipt_hash = hash(
    JSON.stringify({
      schema_id: report.schema_id,
      schema_version: report.schema_version,
      generated_at: report.generated_at,
      summary: report.summary,
      checks: report.checks.map((check) => ({
        id: check.id,
        ok: check.ok,
        expected: check.expected,
        actual: check.actual
      }))
    })
  );

  const docPath = process.env.F100_A_PLUS_DOC_PATH || DEFAULT_STATUS_DOC_PATH;
  const statePath = process.env.F100_A_PLUS_STATE_PATH || DEFAULT_STATE_PATH;

  if (args.write) {
    ensureDir(docPath);
    fs.writeFileSync(docPath, renderMarkdown(report));
  }

  ensureDir(statePath);
  fs.writeFileSync(statePath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);

  if (args.strict && !report.ok) {
    process.exit(1);
  }
}

function status() {
  const statePath = process.env.F100_A_PLUS_STATE_PATH || DEFAULT_STATE_PATH;
  if (!fs.existsSync(statePath)) {
    process.stdout.write(
      `${JSON.stringify(
        {
          schema_id: 'f100_a_plus_readiness_gate_result',
          schema_version: '1.0.0',
          ok: false,
          reason: 'state_missing',
          state_path: rel(statePath)
        },
        null,
        2
      )}\n`
    );
    process.exit(1);
  }
  process.stdout.write(fs.readFileSync(statePath, 'utf8'));
}

function main() {
  const args = parseArgs(process.argv);
  if (args.command === 'run') {
    run(args);
    return;
  }
  if (args.command === 'status') {
    status();
    return;
  }
  process.stderr.write(
    `unknown_command:${args.command}\nusage: node client/runtime/systems/ops/f100_a_plus_readiness_gate.js [run|status] [--strict=1] [--write=1]\n`
  );
  process.exit(1);
}

main();
