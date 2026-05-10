#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'observability/traces/non_shell_trace_completeness_policy.json';
const artifactsRoot = path.join(ROOT, 'core/local/artifacts');
const rows: any[] = [];
if (fs.existsSync(artifactsRoot)) {
  for (const name of fs.readdirSync(artifactsRoot).filter((n) => n.endsWith('.json')).sort()) {
    const file = path.join(artifactsRoot, name);
    try {
      const p = JSON.parse(fs.readFileSync(file, 'utf8'));
      const body = JSON.stringify(p);
      rows.push({ path: `core/local/artifacts/${name}`, has_trace_id: body.includes('trace_id'), type: p.type || p.schema_id || null, ok: p.ok ?? null });
    } catch {}
  }
}
const missing = rows.filter((r) => !r.has_trace_id).slice(0, 100);
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: true, type: 'non_shell_trace_completeness_guard', generated_at: new Date().toISOString(), policy_path: policyPath, mode: 'warning_until_emitters_are_wired', scanned: rows.length, missing_trace_id_sample: missing };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/non_shell_trace_completeness_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
