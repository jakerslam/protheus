#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const SRS_PATH = 'docs/workspace/SRS.md';
const TODO_PATH = 'docs/workspace/TODO.md';
const OUT_JSON = 'artifacts/srs_full_regression_current.json';
const OUT_MD = 'docs/workspace/SRS_FULL_REGRESSION_CURRENT.md';

function shell(cmd) {
  return execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
}

function read(path) {
  return readFileSync(resolve(path), 'utf8');
}

function parseSrsRows(markdown) {
  const rows = [];
  const lines = markdown.split('\n');
  let section = 'Uncategorized';
  for (const line of lines) {
    const h = line.match(/^##\s+(.+)$/);
    if (h) {
      section = h[1].trim();
      continue;
    }
    if (!line.startsWith('|')) continue;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((c) => c.trim());
    if (cells.length < 5) continue;
    if (cells[0] === 'ID' || cells[0] === '---') continue;
    const id = cells[0];
    if (!/^V[0-9A-Z]+-/.test(id)) continue;
    rows.push({
      id,
      status: (cells[1] ?? '').toLowerCase(),
      upgrade: cells[2] ?? '',
      why: cells[3] ?? '',
      exitCriteria: cells[4] ?? '',
      impact: cells[5] ?? '',
      layerMap: cells[6] ?? '',
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

function quoteForSingleShell(str) {
  return `'${str.replace(/'/g, `'\"'\"'`)}'`;
}

function countIdHits(id, nonBacklog = false) {
  const q = quoteForSingleShell(id);
  if (nonBacklog) {
    const out = shell(
      `rg -F --no-messages -n ${q} core client apps adapters scripts tests .github docs ` +
        `-g '!docs/workspace/SRS.md' -g '!docs/workspace/TODO.md' -g '!docs/workspace/UPGRADE_BACKLOG.md' ` +
        `-g '!docs/workspace/SRS_*REGRESSION*.md' -g '!artifacts/srs_*regression*.json' | ` +
        "wc -l | awk '{print $1}'",
    );
    return Number(out.trim() || '0');
  }
  const out = shell(
    `rg -F --no-messages -n ${q} docs/workspace/SRS.md docs/workspace/TODO.md core client apps adapters scripts tests .github docs | ` +
      "wc -l | awk '{print $1}'",
  );
  return Number(out.trim() || '0');
}

function countCodeLikeHits(id) {
  const q = quoteForSingleShell(id);
  const out = shell(
    `rg -F --no-messages -n ${q} core client apps adapters scripts tests .github ` +
      `-g '!docs/workspace/SRS_*REGRESSION*.md' -g '!artifacts/srs_*regression*.json' ` +
      "wc -l | awk '{print $1}'",
  );
  return Number(out.trim() || '0');
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

function regressionSummary(item, cmdAudit, todoUnchecked) {
  const findings = [];
  if (!['queued', 'in_progress', 'blocked', 'done'].includes(item.status)) {
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
  if (item.status === 'in_progress' && item.nonBacklogEvidenceCount === 0 && item.evidenceCount === 0) {
    findings.push('in_progress_without_evidence');
  }
  if (item.status === 'done' && todoUnchecked) {
    findings.push('todo_conflicts_done_status');
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
    findings.includes('invalid_status_value')
  ) {
    severity = 'fail';
  }
  return { severity, findings };
}

function main() {
  const srs = read(SRS_PATH);
  const todo = read(TODO_PATH);
  const srsRows = parseSrsRows(srs);
  const todoUnchecked = parseTodoUnchecked(todo);
  const commandsById = parseTodoValidationCommands(todo);
  const packageScripts = loadPackageScripts();
  const cmdResolution = commandResolution(commandsById, packageScripts);

  const rows = srsRows.map((row, index) => {
    const evidenceCount = countIdHits(row.id, false);
    const nonBacklogEvidenceCount = countIdHits(row.id, true);
    const codeLikeEvidenceCount = countCodeLikeHits(row.id);
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
    doneWithoutNonBacklogEvidence: rows.filter(
      (r) => r.status === 'done' && r.nonBacklogEvidenceCount === 0,
    ).length,
    doneWithoutCodeEvidence: rows.filter(
      (r) => r.status === 'done' && r.codeLikeEvidenceCount === 0,
    ).length,
  };

  const payload = { summary, rows };
  mkdirSync(dirname(OUT_JSON), { recursive: true });
  writeFileSync(OUT_JSON, `${JSON.stringify(payload, null, 2)}\n`);

  const lines = [];
  lines.push('# SRS Full Regression Audit');
  lines.push('');
  lines.push(`- Source SRS items scanned: **${summary.totalSrsRows}**`);
  lines.push(
    `- Regression severities: **fail=${summary.regression.fail}**, **warn=${summary.regression.warn}**, **pass=${summary.regression.pass}**`,
  );
  lines.push(`- Done rows: **${summary.doneRows}**`);
  lines.push(
    `- Done rows without non-backlog evidence: **${summary.doneWithoutNonBacklogEvidence}**`,
  );
  lines.push(`- Done rows without code/test evidence: **${summary.doneWithoutCodeEvidence}**`);
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
  mkdirSync(dirname(OUT_MD), { recursive: true });
  writeFileSync(OUT_MD, `${lines.join('\n')}\n`);

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'srs_full_regression',
        out_json: OUT_JSON,
        out_markdown: OUT_MD,
        summary,
      },
      null,
      2,
    ),
  );
}

main();
