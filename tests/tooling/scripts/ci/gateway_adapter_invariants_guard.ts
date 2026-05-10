#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/gateway_adapter_invariants_policy.json';
const files = ['adapters/runtime/infring_dashboard.ts', 'adapters/runtime/agent_ws_bridge.ts'];
const violations: any[] = [];
for (const rel of files) {
  const full = path.join(ROOT, rel);
  if (!fs.existsSync(full)) { violations.push({ kind: 'gateway_file_missing', path: rel }); continue; }
  const text = fs.readFileSync(full, 'utf8');
  if (/setTimeout\([^,]+,\s*(Number\(|parseInt\(|flags\.|req\.|process\.env\.)/.test(text)) violations.push({ kind: 'possibly_unbounded_user_timer', path: rel });
  if (/raw_runtime_state|all_state|mirror_state|full_state/.test(text)) violations.push({ kind: 'forbidden_gateway_payload_shape_token', path: rel });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'gateway_adapter_invariants_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/gateway_adapter_invariants_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
