#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');

function parseArgs(argv = process.argv.slice(2)) {
  const out = {
    ts: 'coverage/ts/coverage-summary.json',
    rust: 'coverage/rust-summary.txt',
    outJson: 'coverage/combined-summary.json',
    outBadge: 'docs/client/badges/coverage.svg'
  };
  for (const token of argv) {
    if (token.startsWith('--ts=')) out.ts = token.slice('--ts='.length).trim() || out.ts;
    else if (token.startsWith('--rust=')) out.rust = token.slice('--rust='.length).trim() || out.rust;
    else if (token.startsWith('--out-json=')) {
      out.outJson = token.slice('--out-json='.length).trim() || out.outJson;
    } else if (token.startsWith('--out-badge=')) {
      out.outBadge = token.slice('--out-badge='.length).trim() || out.outBadge;
    }
  }
  return out;
}

function readJsonSafe(p) {
  try {
    return JSON.parse(fs.readFileSync(p, 'utf8'));
  } catch {
    return null;
  }
}

function readTextSafe(p) {
  try {
    return String(fs.readFileSync(p, 'utf8'));
  } catch {
    return '';
  }
}

function clampPercent(v) {
  if (!Number.isFinite(v)) return 0;
  return Math.max(0, Math.min(100, v));
}

function parseTsCoverage(tsPath) {
  const raw = readJsonSafe(tsPath) || {};
  const total = raw.total || {};
  const lines = total.lines || {};
  const statements = total.statements || {};
  const functions = total.functions || {};
  const branches = total.branches || {};
  const pct =
    Number.isFinite(Number(lines.pct)) ? Number(lines.pct)
      : Number.isFinite(Number(statements.pct)) ? Number(statements.pct)
      : 0;
  return {
    pct: clampPercent(pct),
    lines_total: Number(lines.total || 0),
    lines_covered: Number(lines.covered || 0),
    statements_pct: clampPercent(Number(statements.pct || 0)),
    functions_pct: clampPercent(Number(functions.pct || 0)),
    branches_pct: clampPercent(Number(branches.pct || 0))
  };
}

function parseRustCoverage(rustPath) {
  const text = readTextSafe(rustPath);
  const lines = text.split('\n');
  let pct = 0;
  for (const line of lines) {
    // cargo llvm-cov --summary-only has a TOTAL row with percent.
    // Example: TOTAL  1234  56  95.46%
    if (!/\bTOTAL\b/i.test(line)) continue;
    const m = line.match(/([0-9]+(?:\.[0-9]+)?)%/);
    if (m) {
      pct = Number(m[1]);
    }
  }
  if (!pct) {
    // fallback: first percent in text
    const m = text.match(/([0-9]+(?:\.[0-9]+)?)%/);
    if (m) pct = Number(m[1]);
  }
  return { pct: clampPercent(pct) };
}

function pickColor(pct) {
  if (pct >= 95) return '#22c55e';
  if (pct >= 90) return '#84cc16';
  if (pct >= 80) return '#f59e0b';
  return '#ef4444';
}

function buildBadge(label, value, color) {
  const left = 78;
  const right = 62;
  const width = left + right;
  const safeLabel = String(label).replace(/&/g, '&amp;').replace(/</g, '&lt;');
  const safeValue = String(value).replace(/&/g, '&amp;').replace(/</g, '&lt;');
  return [
    `<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"${width}\" height=\"20\" role=\"img\" aria-label=\"${safeLabel}: ${safeValue}\">`,
    '<title>coverage</title>',
    `<rect width=\"${left}\" height=\"20\" fill=\"#334155\"/>`,
    `<rect x=\"${left}\" width=\"${right}\" height=\"20\" fill=\"${color}\"/>`,
    `<text x=\"39\" y=\"14\" fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" font-size=\"11\">${safeLabel}</text>`,
    `<text x=\"${left + right / 2}\" y=\"14\" fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" font-size=\"11\">${safeValue}</text>`,
    '</svg>',
    ''
  ].join('\n');
}

function ensureDirFor(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function round2(v) {
  return Math.round(v * 100) / 100;
}

function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const ts = parseTsCoverage(args.ts);
  const rust = parseRustCoverage(args.rust);
  const combinedPct = round2((ts.pct + rust.pct) / 2);

  const payload = {
    ok: true,
    type: 'coverage_merge_summary',
    ts,
    rust,
    combined_pct: combinedPct,
    threshold_95_ok: combinedPct >= 95,
    ts_path: args.ts,
    rust_path: args.rust
  };

  ensureDirFor(args.outJson);
  fs.writeFileSync(args.outJson, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');

  const badge = buildBadge('coverage', `${combinedPct.toFixed(2)}%`, pickColor(combinedPct));
  ensureDirFor(args.outBadge);
  fs.writeFileSync(args.outBadge, badge, 'utf8');

  process.stdout.write(`${JSON.stringify(payload)}\n`);
  return 0;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  parseTsCoverage,
  parseRustCoverage,
  main
};

