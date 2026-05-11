#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
const TODO = 'docs/workspace/todo/todo_registry.json';
const DOCTRINE = 'docs/workspace/REAL_WORK_FIRST.md';
const README = 'docs/workspace/todo/README.md';
const ALLOWED = new Set(['real_work', 'reliability', 'simplification']);
function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}
function boolFlag(name: string, fallback = false): boolean {
  const raw = flag(name, fallback ? '1' : '0');
  return raw === '1' || raw === 'true';
}
function abs(rel: string): string { return path.join(ROOT, rel); }
function read(rel: string): string { return fs.readFileSync(abs(rel), 'utf8'); }
function json(rel: string): any { return JSON.parse(read(rel)); }
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function main(): void {
  const strict = boolFlag('strict', true);
  const minSoonScore = Number(flag('min-soon-score', '3'));
  const outJson = flag('out-json', 'core/local/artifacts/todo_real_work_gate_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/TODO_REAL_WORK_GATE_GUARD_CURRENT.md');
  const registry = json(TODO);
  const doctrine = read(DOCTRINE);
  const readme = read(README);
  const violations: Violation[] = [];
  for (const token of ['The Three Operating Laws', 'Sacred Workflow', 'Admission Gates', 'real_work', 'reliability', 'simplification']) {
    if (!doctrine.includes(token)) violations.push({ kind: 'real_work_doctrine_token_missing', path: DOCTRINE, detail: token });
  }
  for (const token of ['work_gate', 'real_work_score', 'Scripted Workflow']) {
    if (!readme.includes(token)) violations.push({ kind: 'todo_readme_token_missing', path: README, detail: token });
  }
  const items = Array.isArray(registry.items) ? registry.items : [];
  for (const item of items) {
    const id = String(item.id || '<missing>');
    const gate = String(item.work_gate || '');
    const score = Number(item.real_work_score);
    if (!ALLOWED.has(gate)) violations.push({ kind: 'todo_work_gate_missing', path: TODO, detail: `${id} needs work_gate real_work|reliability|simplification` });
    if (!Number.isInteger(score) || score < 1 || score > 5) violations.push({ kind: 'todo_real_work_score_invalid', path: TODO, detail: `${id} needs real_work_score 1..5` });
    if ((item.section === 'red' || item.section === 'yellow') && Number.isInteger(score) && score < minSoonScore) {
      violations.push({ kind: 'todo_soon_score_too_low', path: TODO, detail: `${id} is ${item.section} but score=${score}; move to white or raise score with evidence` });
    }
    if (item.section === 'red' && gate !== 'reliability' && score < 5) {
      violations.push({ kind: 'todo_red_not_reliability_or_critical', path: TODO, detail: `${id} red items should be reliability or score 5 real-work emergencies` });
    }
  }
  const payload = {
    ok: violations.length === 0,
    type: 'todo_real_work_gate_guard',
    generated_at: new Date().toISOString(),
    strict,
    active_items: items.length,
    min_soon_score: minSoonScore,
    gate_counts: items.reduce((acc: Record<string, number>, item: any) => { const key = String(item.work_gate || 'missing'); acc[key] = (acc[key] || 0) + 1; return acc; }, {}),
    violations,
  };
  ensureDir(outJson); fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd); fs.writeFileSync(abs(outMd), `# TODO Real Work Gate Guard\n\n- ok: ${payload.ok}\n- active_items: ${items.length}\n- violations: ${violations.length}\n\n${violations.map((v) => `- ${v.kind}: ${v.detail}`).join('\n') || '- none'}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
