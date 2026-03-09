#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/memory_runtime (authoritative intent)
// Compatibility JS implementation retained for deterministic test parity.

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function nowIso() {
  return new Date().toISOString();
}

function ensureDir(absDir) {
  fs.mkdirSync(absDir, { recursive: true });
}

function readJson(filePath, fallback = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, payload) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath, payload) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function readJsonl(filePath) {
  try {
    if (!fs.existsSync(filePath)) return [];
    return fs.readFileSync(filePath, 'utf8')
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter(Boolean);
  } catch {
    return [];
  }
}

function parseArgs(argv = []) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx > 2) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = String(argv[i + 1] || '');
    if (next && !next.startsWith('--')) {
      out[key] = next;
      i += 1;
      continue;
    }
    out[key] = '1';
  }
  return out;
}

function parseDate(value) {
  const raw = cleanText(value, 20);
  return /^\d{4}-\d{2}-\d{2}$/.test(raw) ? raw : nowIso().slice(0, 10);
}

function addDays(dateStr, deltaDays) {
  const d = new Date(`${dateStr}T00:00:00.000Z`);
  d.setUTCDate(d.getUTCDate() + Number(deltaDays || 0));
  return d.toISOString().slice(0, 10);
}

function parseMemoryIndexRows(memoryIndexPath) {
  try {
    const text = fs.readFileSync(memoryIndexPath, 'utf8');
    const rows = [];
    let headers = null;
    for (const rawLine of text.split(/\r?\n/)) {
      const line = String(rawLine || '').trim();
      if (!line.startsWith('|')) continue;
      const cells = line.split('|').slice(1, -1).map((cell) => cleanText(cell, 400));
      if (!cells.length) continue;
      if (cells.every((cell) => /^[-: ]+$/.test(cell))) continue;
      const normalized = cells.map((cell) => {
        const s = cell.toLowerCase().replace(/[^a-z0-9_]+/g, '_');
        if (s.includes('node_id')) return 'node_id';
        if (s.startsWith('tags')) return 'tags';
        if (s.startsWith('file')) return 'file';
        if (s.startsWith('summary') || s.startsWith('title')) return 'summary';
        return s;
      });
      if (normalized.includes('node_id') && normalized.includes('file')) {
        headers = normalized;
        continue;
      }
      if (!headers) continue;
      const row = {};
      for (let i = 0; i < headers.length; i += 1) row[headers[i]] = cells[i] || '';
      const nodeId = cleanText(row.node_id || '', 120).replace(/`/g, '');
      if (!nodeId) continue;
      const file = cleanText(row.file || '', 220);
      const summary = cleanText(row.summary || '', 220);
      const dateMatch = String(file).match(/(\d{4}-\d{2}-\d{2})/);
      rows.push({
        node_id: nodeId,
        file,
        summary,
        date: dateMatch ? dateMatch[1] : null
      });
    }
    return rows;
  } catch {
    return [];
  }
}

function buildThemeRows(pointerRows, top = 5) {
  const counts = new Map();
  for (const row of pointerRows) {
    const topics = Array.isArray(row && row.topics) ? row.topics : [];
    for (const topic of topics) {
      const token = cleanText(topic, 80).toLowerCase();
      if (!token) continue;
      counts.set(token, (counts.get(token) || 0) + 1);
    }
  }
  return Array.from(counts.entries())
    .sort((a, b) => Number(b[1]) - Number(a[1]) || String(a[0]).localeCompare(String(b[0])))
    .slice(0, Math.max(1, Number(top || 5)))
    .map(([token, score]) => ({ token, score }));
}

function defaultPaths() {
  const pointersDir = process.env.MEMORY_DREAM_POINTERS_DIR
    ? path.resolve(String(process.env.MEMORY_DREAM_POINTERS_DIR))
    : path.join(ROOT, 'client', 'runtime', 'local', 'state', 'memory', 'eyes_pointers');
  const outputDir = process.env.MEMORY_DREAM_OUTPUT_DIR
    ? path.resolve(String(process.env.MEMORY_DREAM_OUTPUT_DIR))
    : path.join(ROOT, 'client', 'runtime', 'local', 'state', 'memory', 'dreams');
  const ledgerPath = process.env.MEMORY_DREAM_LEDGER_PATH
    ? path.resolve(String(process.env.MEMORY_DREAM_LEDGER_PATH))
    : path.join(outputDir, 'dream_runs.jsonl');
  const memoryIndexPath = process.env.MEMORY_DREAM_MEMORY_INDEX_PATH
    ? path.resolve(String(process.env.MEMORY_DREAM_MEMORY_INDEX_PATH))
    : path.join(ROOT, 'client', 'memory', 'MEMORY_INDEX.md');
  const adaptivePointersPath = process.env.MEMORY_DREAM_ADAPTIVE_POINTERS_PATH
    ? path.resolve(String(process.env.MEMORY_DREAM_ADAPTIVE_POINTERS_PATH))
    : path.join(ROOT, 'client', 'runtime', 'local', 'state', 'memory', 'adaptive_pointers.jsonl');
  const failurePointersDir = process.env.MEMORY_DREAM_FAILURE_POINTERS_DIR
    ? path.resolve(String(process.env.MEMORY_DREAM_FAILURE_POINTERS_DIR))
    : path.join(ROOT, 'client', 'runtime', 'local', 'state', 'memory', 'failure_pointers');
  return { pointersDir, outputDir, ledgerPath, memoryIndexPath, adaptivePointersPath, failurePointersDir };
}

function runDream(dateStr, opts = {}) {
  const { pointersDir, outputDir, ledgerPath, memoryIndexPath, adaptivePointersPath, failurePointersDir } = defaultPaths();
  const days = Math.max(1, Number(opts.days || 2));
  const top = Math.max(1, Number(opts.top || 5));

  const pointerRows = [];
  for (let i = 0; i < days; i += 1) {
    const day = addDays(dateStr, -i);
    const fp = path.join(pointersDir, `${day}.jsonl`);
    pointerRows.push(...readJsonl(fp));
  }
  pointerRows.push(...readJsonl(adaptivePointersPath));

  const failureRows = readJsonl(path.join(failurePointersDir, `${dateStr}.jsonl`));

  const indexRows = parseMemoryIndexRows(memoryIndexPath)
    .filter((row) => row && row.date && String(row.date) < String(dateStr))
    .slice(0, 20);

  const themes = buildThemeRows(pointerRows, top);
  const olderLinks = indexRows.slice(0, top).map((row) => ({
    node_id: row.node_id,
    ref: `${row.file}#${row.node_id}`,
    summary: row.summary
  }));

  const payload = {
    ok: true,
    type: 'memory_dream',
    ts: nowIso(),
    date: dateStr,
    pointer_rows: pointerRows.length,
    failure_pointer_rows: failureRows.length,
    themes: themes.length,
    older_links_total: olderLinks.length,
    dream_file_json: path.join(outputDir, `${dateStr}.json`),
    dream_file_md: path.join(outputDir, `${dateStr}.md`)
  };

  const jsonDoc = {
    ...payload,
    themes,
    source_refs: pointerRows
      .map((row) => `${cleanText(row && row.memory_file || '', 220)}#${cleanText(row && row.node_id || '', 120)}`)
      .filter((ref) => ref !== '#')
      .slice(0, 200),
    older_links: olderLinks
  };

  const mdLines = [
    `# Memory Dream Sheet: ${dateStr}`,
    '',
    `Pointer rows: ${pointerRows.length}`,
    `Failure pointers: ${failureRows.length}`,
    `Themes: ${themes.length}`,
    `Older links: ${olderLinks.length}`,
    '',
    '## Pointer References',
    ...jsonDoc.source_refs.slice(0, 40).map((ref) => `- ${ref}`),
    '',
    '## Older Memory Echoes',
    ...olderLinks.slice(0, 20).map((row) => `- ${row.ref}`),
    ''
  ];

  writeJson(path.join(outputDir, `${dateStr}.json`), jsonDoc);
  ensureDir(outputDir);
  fs.writeFileSync(path.join(outputDir, `${dateStr}.md`), `${mdLines.join('\n')}\n`, 'utf8');
  appendJsonl(ledgerPath, payload);

  return payload;
}

function statusDream(dateStr) {
  const { outputDir } = defaultPaths();
  const jsonPath = path.join(outputDir, `${dateStr}.json`);
  const payload = readJson(jsonPath, null);
  if (!payload) {
    return {
      ok: true,
      type: 'memory_dream_status',
      date: dateStr,
      exists: false,
      themes: 0,
      failure_pointer_rows: 0,
      older_links_total: 0
    };
  }
  return {
    ok: true,
    type: 'memory_dream_status',
    date: dateStr,
    exists: true,
    themes: Array.isArray(payload.themes) ? payload.themes.length : Number(payload.themes || 0),
    failure_pointer_rows: Number(payload.failure_pointer_rows || 0),
    older_links_total: Number(payload.older_links_total || 0),
    dream_file_json: jsonPath,
    dream_file_md: path.join(outputDir, `${dateStr}.md`)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'run', 24).toLowerCase();
  const dateStr = parseDate(args._[1] || nowIso().slice(0, 10));

  if (cmd === 'status') {
    const out = statusDream(dateStr);
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(0);
  }

  if (cmd === 'run') {
    const out = runDream(dateStr, {
      days: Number(args.days || 2),
      top: Number(args.top || 5)
    });
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(out && out.ok === true ? 0 : 1);
  }

  process.stdout.write(`${JSON.stringify({ ok: false, reason: `unknown_command:${cmd}` }, null, 2)}\n`);
  process.exit(1);
}

if (require.main === module) {
  main();
}

module.exports = {
  runDream,
  statusDream
};
