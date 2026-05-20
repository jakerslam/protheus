#!/usr/bin/env node
import child_process from 'child_process';
import fs from 'fs';
import path from 'path';

type Json = Record<string, any>;
const root = process.cwd();
const policyRel = 'observability/sentinel/sentinel_worktree_scope_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), 'utf8')) as Json;
function run(cmd: string): string { return child_process.execSync(cmd, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }); }
function classify(file: string): string {
  const scopes = policy.scope_prefixes || {};
  for (const [scope, prefixes] of Object.entries(scopes)) {
    if ((prefixes as string[]).some((prefix) => file === prefix || file.startsWith(prefix))) return scope;
  }
  return 'other';
}
function restricted(file: string): boolean {
  return (policy.restricted_lanes || []).some((prefix: string) => file.startsWith(prefix));
}
const lines = run('git status --porcelain=v1 -uall').split(/\r?\n/).filter(Boolean);
const rows = lines.map((line) => {
  const status = line.slice(0, 2);
  const raw = line.slice(3);
  const file = raw.includes(' -> ') ? raw.split(' -> ').pop() || raw : raw;
  return { status, path: file, scope: classify(file), restricted: restricted(file), tracked: status !== '??' };
});
const byScope: Record<string, Json> = {};
for (const row of rows) {
  const bucket = byScope[row.scope] || { scope: row.scope, total: 0, tracked: 0, untracked: 0, restricted: 0, examples: [] };
  bucket.total += 1;
  if (row.tracked) bucket.tracked += 1; else bucket.untracked += 1;
  if (row.restricted) bucket.restricted += 1;
  if (bucket.examples.length < 12) bucket.examples.push({ status: row.status, path: row.path, restricted: row.restricted });
  byScope[row.scope] = bucket;
}
const scopes = Object.values(byScope).sort((a: Json, b: Json) => b.total - a.total);
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-worktree-scope`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  type: 'kernel_sentinel_worktree_scope_report',
  generated_at: new Date().toISOString(),
  ok: rows.filter((row) => row.tracked && !row.restricted).length <= 20,
  policy_path: policyRel,
  total: rows.length,
  tracked: rows.filter((row) => row.tracked).length,
  untracked: rows.filter((row) => !row.tracked).length,
  restricted_tracked: rows.filter((row) => row.tracked && row.restricted).length,
  unrestricted_tracked: rows.filter((row) => row.tracked && !row.restricted).length,
  scopes,
  recommended_commit_order: scopes
    .filter((scope: Json) => scope.tracked > 0 && scope.restricted === 0)
    .map((scope: Json) => scope.scope),
  restricted_scope_warning: scopes.filter((scope: Json) => scope.restricted > 0).map((scope: Json) => ({ scope: scope.scope, restricted: scope.restricted })),
  next_action: 'Commit unrestricted scopes separately; do not touch restricted Shell/Orchestration scopes without explicit permission.'
};
const outJson = path.join(root, String(policy.output_json));
fs.mkdirSync(path.dirname(outJson), { recursive: true });
fs.writeFileSync(outJson, `${JSON.stringify(report, null, 2)}\n`);
const md = ['# Kernel Sentinel Worktree Scope', '', `Generated: ${report.generated_at}`, '', `Tracked: ${report.tracked}`, `Untracked: ${report.untracked}`, `Restricted tracked: ${report.restricted_tracked}`, '', '## Scopes', ''];
for (const scope of scopes) md.push(`- ${scope.scope}: total=${scope.total}, tracked=${scope.tracked}, untracked=${scope.untracked}, restricted=${scope.restricted}`);
md.push('', '## Next action', '', report.next_action, '');
const outMd = path.join(root, String(policy.output_markdown));
fs.mkdirSync(path.dirname(outMd), { recursive: true });
fs.writeFileSync(outMd, `${md.join('\n')}\n`);
console.log(JSON.stringify({ ok: report.ok, tracked: report.tracked, untracked: report.untracked, restricted_tracked: report.restricted_tracked, report: path.relative(root, outJson), markdown: path.relative(root, outMd) }, null, 2));
