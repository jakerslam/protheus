#!/usr/bin/env node
/* eslint-disable no-console */
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const SRS_PATH = 'docs/workspace/SRS.md';
const TODO_PATH = 'docs/workspace/TODO.md';
const OUT_JSON =
  readAliasedEnv('INFRING_SRS_FULL_REGRESSION_OUT_JSON', 'PROTHEUS_SRS_FULL_REGRESSION_OUT_JSON')
  || 'core/local/artifacts/srs_full_regression_current.json';
const OUT_MD =
  readAliasedEnv('INFRING_SRS_FULL_REGRESSION_OUT_MD', 'PROTHEUS_SRS_FULL_REGRESSION_OUT_MD')
  || 'local/workspace/reports/SRS_FULL_REGRESSION_CURRENT.md';

function readAliasedEnv(primary, legacy) {
  const preferred = String(process.env[primary] || '').trim();
  const legacyValue = String(process.env[legacy] || '').trim();
  if (!preferred && legacyValue) {
    process.env[primary] = legacyValue;
    return legacyValue;
  }
  if (preferred && !legacyValue) {
    process.env[legacy] = preferred;
  }
  return preferred;
}

function readAliasedBool(primary, legacy, fallback) {
  const value = readAliasedEnv(primary, legacy);
  if (!value) return fallback;
  return parseBool(value, fallback);
}

function parseCliFlags(argv = process.argv.slice(2)) {
  return {
    strict:
      hasFlag(argv, 'strict')
      || parseBool(
        readFlag(argv, 'strict'),
        readAliasedBool(
          'INFRING_SRS_FULL_REGRESSION_STRICT',
          'PROTHEUS_SRS_FULL_REGRESSION_STRICT',
          false
        )
      ),
    failOnWarn:
      hasFlag(argv, 'fail-on-warn')
      || parseBool(
        readFlag(argv, 'fail-on-warn'),
        readAliasedBool(
          'INFRING_SRS_FULL_REGRESSION_FAIL_ON_WARN',
          'PROTHEUS_SRS_FULL_REGRESSION_FAIL_ON_WARN',
          false
        )
      ),
    outJson: String(readFlag(argv, 'out-json') || OUT_JSON),
    outMarkdown: String(readFlag(argv, 'out-markdown') || OUT_MD),
  };
}

function read(path) {
  return readFileSync(resolve(path), 'utf8');
}

function parseSrsRows(markdown) {
  const rows = [];
  const lines = markdown.split('\n');
  let section = 'Uncategorized';

  function splitTableCells(line) {
    const cells = [];
    let current = '';
    let inBackticks = false;
    for (let i = 0; i < line.length; i += 1) {
      const ch = line[i];
      const escaped = i > 0 && line[i - 1] === '\\';
      if (ch === '`' && !escaped) {
        inBackticks = !inBackticks;
      }
      if (ch === '|' && !escaped && !inBackticks) {
        cells.push(current.trim());
        current = '';
        continue;
      }
      current += ch;
    }
    cells.push(current.trim());
    return cells;
  }

  for (const line of lines) {
    const h = line.match(/^##\s+(.+)$/);
    if (h) {
      section = h[1].trim();
      continue;
    }
    if (!line.startsWith('|')) continue;
    const rawCells = splitTableCells(line).slice(1, -1).map((c) => c.trim());
    const cells = [...rawCells];
    if (cells.length < 5) continue;
    if (cells[0] === 'ID' || cells[0] === '---') continue;
    const id = cells[0];
    if (!/^V[0-9A-Z]+-/.test(id)) continue;
    let impact = cells[5] ?? '';
    let layerMap = cells[6] ?? '';
    if ((!impact || !layerMap) && cells.length === 6) {
      const compact = String(cells[5] ?? '').trim();
      const compactMatch = compact.match(/^(\d+)\s*\/\s*(.+)$/);
      if (compactMatch) {
        impact = compactMatch[1];
        layerMap = compactMatch[2];
      }
    }
    rows.push({
      id,
      status: (cells[1] ?? '').toLowerCase(),
      upgrade: cells[2] ?? '',
      why: cells[3] ?? '',
      exitCriteria: cells[4] ?? '',
      impact,
      layerMap,
      section,
    });
  }
  return rows;
}

function parseTodoUnchecked(todo) {
  const out = new Set();
  for (const m of todo.matchAll(/^- \[ \]\s+`([^`]+)`/gm)) {
    out.add(m[1]);
  }
  return out;
}

function parseTodoValidationCommands(todo) {
  const lines = todo.split('\n');
  const map = new Map();
  let currentId = null;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const idMatch = line.match(/^- \[[ x]\]\s+`([^`]+)`/);
    if (idMatch) {
      currentId = idMatch[1];
      if (!map.has(currentId)) map.set(currentId, []);
      continue;
    }
    if (!currentId) continue;
    if (/^- \[[ x]\]\s+`/.test(line)) continue;
    const commands = [...line.matchAll(/`([^`]+)`/g)].map((m) => m[1]);
    for (const c of commands) {
      if (
        c.startsWith('npm run') ||
        c.startsWith('node ') ||
        c.startsWith('cargo ') ||
        c.startsWith('bash ') ||
        c.startsWith('./')
      ) {
        map.get(currentId).push(c);
      }
    }
  }
  return map;
}

function parseRipgrepJsonMatches(raw, counts) {
  const text = String(raw ?? '');
  if (!text.trim()) return;
  const lines = text.split('\n');
  for (const line of lines) {
    if (!line.trim()) continue;
    let event;
    try {
      event = JSON.parse(line);
    } catch {
      continue;
    }
    if (event?.type !== 'match') continue;
    const submatches = event?.data?.submatches ?? [];
    for (const sub of submatches) {
      const matched = sub?.match?.text;
      if (!matched || !counts.has(matched)) continue;
      counts.set(matched, (counts.get(matched) ?? 0) + 1);
    }
  }
}

function countHitsById(ids, paths, globs = []) {
  const counts = new Map(ids.map((id) => [id, 0]));
  if (ids.length === 0) return counts;

  const tmp = mkdtempSync(join(tmpdir(), 'srs-full-regression-'));
  const patternFile = join(tmp, 'id_patterns.txt');
  // Longest-first prevents prefix collisions (e.g. `V6-X-1` matching inside `V6-X-10`).
  const ordered = [...ids].sort((a, b) => b.length - a.length || a.localeCompare(b));
  writeFileSync(patternFile, `${ordered.join('\n')}\n`, 'utf8');

  const existingPaths = paths.filter((candidate) => existsSync(resolve(candidate)));
  if (existingPaths.length === 0) {
    rmSync(tmp, { recursive: true, force: true });
    return counts;
  }

  const args = ['-F', '--no-messages', '-n', '--json', '-f', patternFile, ...existingPaths];
  for (const g of globs) args.push('-g', g);

  try {
    const out = execFileSync('rg', args, { encoding: 'utf8', maxBuffer: 1024 * 1024 * 512 });
    parseRipgrepJsonMatches(out, counts);
  } catch (err) {
    // rg exits 1 when no matches are found; still parse any stdout if present.
    if (err && typeof err === 'object') {
      parseRipgrepJsonMatches(err.stdout, counts);
      if (err.status !== 1) throw err;
    } else {
      throw err;
    }
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }

  return counts;
}

function loadPackageScripts() {
  const pkg = JSON.parse(read('package.json'));
  return new Set(Object.keys(pkg.scripts ?? {}));
}

function extractNpmScriptName(cmd) {
  const parts = cmd.split(/\s+/);
  if (parts[0] !== 'npm' || parts[1] !== 'run') return null;
  if (parts[2] === '-s') return parts[3] ?? null;
  return parts[2] ?? null;
}

function commandResolution(commandsById, packageScripts) {
  const out = new Map();
  for (const [id, cmds] of commandsById.entries()) {
    const resolved = [];
    const unresolved = [];
    for (const cmd of cmds) {
      if (cmd.startsWith('npm run')) {
        const name = extractNpmScriptName(cmd);
        if (name && packageScripts.has(name)) resolved.push(cmd);
        else unresolved.push(cmd);
        continue;
      }
      if (cmd.startsWith('node ') || cmd.startsWith('bash ')) {
        const file = cmd.split(/\s+/)[1];
        if (file && existsSync(file)) resolved.push(cmd);
        else unresolved.push(cmd);
        continue;
      }
      if (cmd.startsWith('./')) {
        const file = cmd.split(/\s+/)[0];
        if (existsSync(file)) resolved.push(cmd);
        else unresolved.push(cmd);
        continue;
      }
      if (cmd.startsWith('cargo ')) {
        resolved.push(cmd);
      }
    }
    out.set(id, { resolved, unresolved });
  }
  return out;
}

function hasExternalPreparedPacket(id) {
  const dir = resolve('docs/external/evidence', id);
  if (!existsSync(dir)) return false;
  const files = readdirSync(dir);
  const hasReadme = files.includes('README.md');
  const hasManifest = files.includes('packet_manifest.json');
  const hasPacket = files.some((name) => /^external_execution_packet_.*\.md$/i.test(name));
  return hasReadme && hasManifest && hasPacket;
}

function regressionSummary(item, cmdAudit, todoUnchecked) {
  const findings = [];
  if (
    !['queued', 'in_progress', 'blocked', 'blocked_external_prepared', 'done', 'existing-coverage-validated'].includes(
      item.status,
    )
  ) {
    findings.push('invalid_status_value');
  }
  if (!/^\d+$/.test(item.impact || '')) {
    findings.push('missing_or_invalid_impact');
  }
  if (!item.layerMap) {
    findings.push('missing_layer_map');
  }
  if (item.status === 'done' && item.nonBacklogEvidenceCount === 0) {
    findings.push('done_without_non_backlog_evidence');
  }
  if (item.status === 'done' && item.codeLikeEvidenceCount === 0) {
    findings.push('done_without_code_or_test_evidence');
  }
  if (item.status === 'existing-coverage-validated' && item.nonBacklogEvidenceCount === 0) {
    findings.push('coverage_without_non_backlog_evidence');
  }
  if (item.status === 'existing-coverage-validated' && item.codeLikeEvidenceCount === 0) {
    findings.push('coverage_without_code_or_test_evidence');
  }
  if (item.status === 'in_progress' && item.nonBacklogEvidenceCount === 0 && item.evidenceCount === 0) {
    findings.push('in_progress_without_evidence');
  }
  if (item.status === 'blocked_external_prepared' && !item.externalPreparedPacket) {
    findings.push('blocked_external_prepared_without_packet');
  }
  if (item.status === 'done' && todoUnchecked) {
    findings.push('todo_conflicts_done_status');
  }
  if (item.status === 'done' && item.id.startsWith('V8-') && !item.v8RuntimeProofCovered) {
    findings.push('v8_done_missing_runtime_proof_coverage');
  }
  if (item.status === 'existing-coverage-validated' && todoUnchecked) {
    findings.push('todo_conflicts_coverage_status');
  }
  if (cmdAudit && cmdAudit.unresolved.length > 0) {
    findings.push('unresolved_validation_commands');
  }
  if (item.id.includes('..')) {
    findings.push('aggregate_id_range_requires_split_execution');
  }

  let severity = 'pass';
  if (findings.length > 0) severity = 'warn';
  if (
    findings.includes('unresolved_validation_commands') ||
    findings.includes('invalid_status_value') ||
    findings.includes('done_without_non_backlog_evidence') ||
    findings.includes('done_without_code_or_test_evidence') ||
    findings.includes('v8_done_missing_runtime_proof_coverage') ||
    findings.includes('coverage_without_non_backlog_evidence') ||
    findings.includes('coverage_without_code_or_test_evidence') ||
    findings.includes('blocked_external_prepared_without_packet')
  ) {
    severity = 'fail';
  }
  return { severity, findings };
}

function shouldRetryForEvidenceCollapse(payload) {
  const summary = payload?.summary ?? {};
  if ((summary?.regression?.fail ?? 0) === 0) return false;
  if ((summary.doneWithoutNonBacklogEvidence ?? 0) > 0) return true;
  if ((summary.doneWithoutCodeEvidence ?? 0) > 0) return true;
  return payload.rows.some((row) =>
    row.regression.findings.some((finding) =>
      [
        'done_without_non_backlog_evidence',
        'done_without_code_or_test_evidence',
        'coverage_without_non_backlog_evidence',
        'coverage_without_code_or_test_evidence',
      ].includes(finding),
    ),
  );
}

function buildRegressionPayload() {
  const srs = read(SRS_PATH);
  const todo = read(TODO_PATH);
  const srsRows = parseSrsRows(srs);
  const uniqueIds = [...new Set(srsRows.map((row) => row.id))];
  const todoUnchecked = parseTodoUnchecked(todo);
  const commandsById = parseTodoValidationCommands(todo);
  const packageScripts = loadPackageScripts();
  const cmdResolution = commandResolution(commandsById, packageScripts);
  const v8RuntimeProofPath = resolve('core/layer0/ops/tests/v8_runtime_proof.rs');
  const v8RuntimeProofSource = existsSync(v8RuntimeProofPath) ? readFileSync(v8RuntimeProofPath, 'utf8') : '';

  const evidencePaths = [
    'docs/workspace/SRS.md',
    'docs/workspace/TODO.md',
    'core',
    'client',
    'apps',
    'adapters',
    'scripts',
    'tests',
    '.github',
    'docs',
  ];

  const evidenceCounts = countHitsById(uniqueIds, evidencePaths);

  const nonBacklogEvidenceCounts = countHitsById(
    uniqueIds,
    ['core', 'client', 'apps', 'adapters', 'scripts', 'tests', '.github', 'docs'],
    [
      '!docs/workspace/SRS.md',
      '!docs/workspace/TODO.md',
      '!docs/workspace/UPGRADE_BACKLOG.md',
      '!docs/workspace/SRS_*REGRESSION*.md',
      '!core/local/artifacts/srs_*regression*.json',
    ],
  );

  const codeLikeEvidenceCounts = countHitsById(
    uniqueIds,
    ['core', 'client', 'apps', 'adapters', 'scripts', 'tests', '.github'],
    ['!docs/workspace/SRS_*REGRESSION*.md', '!core/local/artifacts/srs_*regression*.json'],
  );

  const rows = srsRows.map((row, index) => {
    const evidenceCount = evidenceCounts.get(row.id) ?? 0;
    const nonBacklogEvidenceCount = nonBacklogEvidenceCounts.get(row.id) ?? 0;
    const codeLikeEvidenceCount = codeLikeEvidenceCounts.get(row.id) ?? 0;
    const cmdAudit = cmdResolution.get(row.id) ?? { resolved: [], unresolved: [] };
    const item = {
      rank: index + 1,
      ...row,
      evidenceCount,
      nonBacklogEvidenceCount,
      codeLikeEvidenceCount,
      todoUnchecked: todoUnchecked.has(row.id),
      validationCommandsResolved: cmdAudit.resolved.length,
      validationCommandsUnresolved: cmdAudit.unresolved,
      externalPreparedPacket: hasExternalPreparedPacket(row.id),
      v8RuntimeProofCovered: !row.id.startsWith('V8-') || v8RuntimeProofSource.includes(`"${row.id}"`),
    };
    item.regression = regressionSummary(item, cmdAudit, todoUnchecked.has(row.id));
    return item;
  });

  const summary = {
    generatedAt: new Date().toISOString(),
    source: { srs: SRS_PATH, todo: TODO_PATH },
    totalSrsRows: rows.length,
    regression: {
      fail: rows.filter((r) => r.regression.severity === 'fail').length,
      warn: rows.filter((r) => r.regression.severity === 'warn').length,
      pass: rows.filter((r) => r.regression.severity === 'pass').length,
    },
    doneRows: rows.filter((r) => r.status === 'done').length,
    existingCoverageRows: rows.filter((r) => r.status === 'existing-coverage-validated').length,
    doneWithoutNonBacklogEvidence: rows.filter(
      (r) => r.status === 'done' && r.nonBacklogEvidenceCount === 0,
    ).length,
    doneWithoutCodeEvidence: rows.filter(
      (r) => r.status === 'done' && r.codeLikeEvidenceCount === 0,
    ).length,
  };

  return { summary, rows };
}

function writeArtifacts(payload, outJson = OUT_JSON, outMarkdown = OUT_MD) {
  const { summary, rows } = payload;
  writeJsonArtifact(resolve(outJson), payload);

  const lines = [];
  lines.push('# SRS Full Regression Audit');
  lines.push('');
  lines.push(`- Source SRS items scanned: **${summary.totalSrsRows}**`);
  lines.push(
    `- Regression severities: **fail=${summary.regression.fail}**, **warn=${summary.regression.warn}**, **pass=${summary.regression.pass}**`,
  );
  lines.push(`- Done rows: **${summary.doneRows}**`);
  lines.push(`- Existing coverage validated rows: **${summary.existingCoverageRows}**`);
  lines.push(
    `- Done rows without non-backlog evidence: **${summary.doneWithoutNonBacklogEvidence}**`,
  );
  lines.push(`- Done rows without code/test evidence: **${summary.doneWithoutCodeEvidence}**`);
  if (summary.retry?.attempted) {
    lines.push(
      `- Retry evidence-collapse check: **attempted**, used_second=**${summary.retry.used_second ? 'true' : 'false'}**`,
    );
  }
  lines.push(`- Machine report: \`${OUT_JSON}\``);
  lines.push('');
  lines.push('| # | ID | Status | Evidence | Non-Backlog | Code/Test | Regression |');
  lines.push('|---:|---|---|---:|---:|---:|---|');
  for (const item of rows) {
    lines.push(
      `| ${item.rank} | ${item.id} | ${item.status} | ${item.evidenceCount} | ${item.nonBacklogEvidenceCount} | ${item.codeLikeEvidenceCount} | ${item.regression.severity} |`,
    );
  }
  lines.push('');
  lines.push('## Fail Findings');
  lines.push('');
  for (const item of rows.filter((r) => r.regression.severity === 'fail')) {
    lines.push(`- \`${item.id}\` (${item.status}): ${item.regression.findings.join(', ')}`);
  }
  lines.push('');
  lines.push('## Warn Findings');
  lines.push('');
  for (const item of rows.filter((r) => r.regression.severity === 'warn')) {
    lines.push(`- \`${item.id}\` (${item.status}): ${item.regression.findings.join(', ')}`);
  }
  writeTextArtifact(resolve(outMarkdown), `${lines.join('\n')}\n`);
}

function main() {
  const started = Date.now();
  const flags = parseCliFlags();
  const first = buildRegressionPayload();
  let payload = first;
  let retry = null;
  if (shouldRetryForEvidenceCollapse(first)) {
    const second = buildRegressionPayload();
    retry = {
      attempted: true,
      first_fail: first.summary.regression.fail,
      second_fail: second.summary.regression.fail,
      first_done_without_non_backlog_evidence: first.summary.doneWithoutNonBacklogEvidence,
      second_done_without_non_backlog_evidence: second.summary.doneWithoutNonBacklogEvidence,
      first_done_without_code_evidence: first.summary.doneWithoutCodeEvidence,
      second_done_without_code_evidence: second.summary.doneWithoutCodeEvidence,
      used_second: false,
    };
    const firstSeverityTuple = [
      first.summary.regression.fail,
      first.summary.doneWithoutNonBacklogEvidence,
      first.summary.doneWithoutCodeEvidence,
    ];
    const secondSeverityTuple = [
      second.summary.regression.fail,
      second.summary.doneWithoutNonBacklogEvidence,
      second.summary.doneWithoutCodeEvidence,
    ];
    if (
      JSON.stringify(secondSeverityTuple) !== JSON.stringify(firstSeverityTuple) &&
      (
        second.summary.regression.fail < first.summary.regression.fail ||
        second.summary.doneWithoutNonBacklogEvidence < first.summary.doneWithoutNonBacklogEvidence ||
        second.summary.doneWithoutCodeEvidence < first.summary.doneWithoutCodeEvidence
      )
    ) {
      payload = second;
      retry.used_second = true;
    }
  }
  payload.summary.retry = retry;
  writeArtifacts(payload, flags.outJson, flags.outMarkdown);
  const summary = payload.summary;
  const shouldFail =
    flags.strict && (summary.regression.fail > 0 || (flags.failOnWarn && summary.regression.warn > 0));
  const result = {
    ok: !shouldFail,
    type: 'srs_full_regression',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: 'ops',
    revision: currentRevision(process.cwd()),
    inputs: {
      strict: flags.strict,
      fail_on_warn: flags.failOnWarn,
      out_json: flags.outJson,
      out_markdown: flags.outMarkdown,
    },
    summary,
    failures: rowsToFailures(payload.rows),
    artifact_paths: [flags.outJson, flags.outMarkdown],
    report: payload,
  };
  return emitStructuredResult(result, {
    outPath: '',
    strict: flags.strict || flags.failOnWarn,
    ok: result.ok,
    history: true,
  });
}

function rowsToFailures(rows) {
  return rows
    .filter((item) => item.regression.severity !== 'pass')
    .map((item) => ({
      id: item.id,
      detail: item.regression.findings.join(', '),
    }));
}

process.exit(main());
