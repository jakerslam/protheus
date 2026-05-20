#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

type Json = Record<string, any>;
const root = process.cwd();
const policyRel = 'observability/sentinel/sentinel_daily_state_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), 'utf8')) as Json;
function readJson(rel: string): Json | null { try { return JSON.parse(fs.readFileSync(path.join(root, rel), 'utf8')) as Json; } catch { return null; } }
function statIso(rel: string): string | null { try { return fs.statSync(path.join(root, rel)).mtime.toISOString(); } catch { return null; } }
const inputs = policy.inputs || {};
const loaded: Record<string, Json | null> = {};
for (const [key, rel] of Object.entries(inputs)) loaded[key] = readJson(String(rel));
const anti = loaded.anti_entropy || {};
const feedback = loaded.feedback_summary || {};
const worktree = loaded.worktree_danger || {};
const worktreeScope = loaded.worktree_scope || {};
const freshness = loaded.artifact_freshness || {};
const command = loaded.command_entropy || {};
const ciSurface = loaded.ci_surface_sprawl || {};
const timing = loaded.timing_trend || {};
const risks = [
  ...(Array.isArray(anti.top_findings) ? anti.top_findings : []),
  ...(Array.isArray(feedback.top_worktree_danger_findings) ? feedback.top_worktree_danger_findings : []),
  ...(Array.isArray(freshness.violations) ? freshness.violations.map((v: Json) => ({ id: v.kind, severity: 'medium', summary: `${v.kind}: ${v.path || ''}`, next_action: v.next_action || 'Refresh Sentinel source artifact.', evidence_refs: [v.path].filter(Boolean) })) : []),
  ...(Array.isArray(command.violations) ? command.violations.map((v: Json) => ({ id: v.kind, severity: 'medium', summary: `${v.kind}: ${v.actual ?? v.new_count ?? ''}`, next_action: v.next_action || 'Reduce command entropy.', evidence_refs: ['package.json', 'tools/commands/command_registry.json'] })) : []),
  ...(Array.isArray(ciSurface.violations) ? ciSurface.violations.map((v: Json) => ({ id: v.kind, severity: 'medium', summary: `${v.kind}: ${v.actual ?? ''}`, next_action: v.next_action || 'Reduce or classify CI workflow surface.', evidence_refs: ['.github/workflows', 'validation/conformance/contracts/ci_workflow_tier_manifest.json'] })) : [])
];
const seen = new Set<string>();
const topRisks = risks.filter((risk: Json) => {
  const key = String(risk.id || risk.kind || risk.summary || JSON.stringify(risk));
  if (seen.has(key)) return false;
  seen.add(key);
  return true;
}).slice(0, Number(policy.budgets?.max_top_risks || 5));
const nextAction = topRisks[0]?.next_action || 'No immediate Sentinel action required.';
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-daily-state`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: String(anti.trace_id || feedback.trace_id || worktree.trace_id || ''),
  source_domain: 'observability',
  type: 'kernel_sentinel_daily_state_report',
  generated_at: new Date().toISOString(),
  ok: Boolean(anti.ok === true && feedback.ok !== false && freshness.ok !== false),
  status: anti.status || feedback.status || 'unknown',
  anti_entropy_score: anti.anti_entropy_score ?? null,
  worktree: {
    ok: worktree.ok ?? null,
    tracked_churn: worktree.summary?.tracked_churn ?? null,
    untracked_churn: worktree.summary?.untracked_churn ?? null,
    finding_count: worktree.finding_count ?? null,
    scoped_tracked: worktreeScope.tracked ?? null,
    scoped_restricted_tracked: worktreeScope.restricted_tracked ?? null,
    recommended_commit_order: worktreeScope.recommended_commit_order ?? [],
  },
  freshness: {
    ok: freshness.ok ?? null,
    missing_count: freshness.missing_count ?? null,
    stale_count: freshness.stale_count ?? null,
    fresh_count: freshness.fresh_count ?? null,
  },
  command_entropy: {
    ok: command.ok ?? null,
    package_script_count: command.package_script_count ?? null,
    default_operator_command_count: command.default_operator_command_count ?? null,
    new_script_count: command.new_script_count ?? null,
    warning_count: Array.isArray(command.warnings) ? command.warnings.length : 0,
  },
  ci_surface: {
    ok: ciSurface.ok ?? null,
    total: ciSurface.summary?.total ?? null,
    required: ciSurface.summary?.required ?? null,
    advisory: ciSurface.summary?.advisory ?? null,
    unknown: ciSurface.summary?.unknown ?? null,
  },
  timing_trend: {
    status: timing.status || null,
    sample_count: timing.sample_count ?? null,
    full_sample_count: timing.full_sample_count ?? null,
    generated_at: timing.generated_at || null,
  },
  top_risks: topRisks,
  recommended_next_action: nextAction,
  input_mtimes: Object.fromEntries(Object.entries(inputs).map(([key, rel]) => [key, statIso(String(rel))])),
  source_refs: inputs,
};
const outJson = path.join(root, String(policy.output_json || 'core/local/artifacts/kernel_sentinel_daily_state_current.json'));
fs.mkdirSync(path.dirname(outJson), { recursive: true });
fs.writeFileSync(outJson, `${JSON.stringify(report, null, 2)}\n`);
const lines = ['# Kernel Sentinel Daily State', '', `Generated: ${report.generated_at}`, '', `Status: ${report.status}`, `OK: ${report.ok}`, `Anti-entropy score: ${report.anti_entropy_score}`, '', '## Top Risks', ''];
for (const risk of topRisks) {
  lines.push(`- ${String(risk.id || risk.kind || 'risk')}: ${String(risk.summary || risk.root_cause_hypothesis || '')}`);
  lines.push(`  Next: ${String(risk.next_action || 'Inspect source refs.')}`);
}
lines.push('', '## Recommended Next Action', '', String(nextAction), '');
const outMd = path.join(root, String(policy.output_markdown || 'local/workspace/reports/KERNEL_SENTINEL_DAILY_STATE_CURRENT.md'));
fs.mkdirSync(path.dirname(outMd), { recursive: true });
fs.writeFileSync(outMd, `${lines.join('\n')}\n`);
console.log(JSON.stringify({ ok: report.ok, status: report.status, anti_entropy_score: report.anti_entropy_score, top_risk_count: topRisks.length, report: path.relative(root, outJson), markdown: path.relative(root, outMd) }, null, 2));
