#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

type Json = Record<string, any>;
const root = process.cwd();
const policyRel = 'observability/sentinel/sentinel_command_entropy_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), 'utf8')) as Json;
const outRel = String(policy.output_path || 'core/local/artifacts/kernel_sentinel_command_entropy_current.json');
const historyRel = String(policy.history_path || 'local/state/observability/sentinel/command_entropy_history.jsonl');
const thresholds = policy.thresholds || {};
function readJson(rel: string): Json | null { try { return JSON.parse(fs.readFileSync(path.join(root, rel), 'utf8')) as Json; } catch { return null; } }
function readHistory(): Json[] {
  try { return fs.readFileSync(path.join(root, historyRel), 'utf8').split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line)); } catch { return []; }
}
const pkg = readJson('package.json') || {};
const scripts = Object.keys(pkg.scripts || {}).sort();
const registry = readJson('tools/commands/command_registry.json') || {};
const registryRows = Array.isArray(registry.entries)
  ? registry.entries
  : Array.isArray(registry.commands)
    ? registry.commands
    : [];
const defaultOperatorCommands = Number.isFinite(Number(registry.operator_surface_count))
  ? Number(registry.operator_surface_count)
  : registryRows.filter((row: Json) => row.operator_surface === true || row.default_operator_surface === true || row.default_visible === true || row.default === true).length;
const history = readHistory();
const previous = history.length ? history[history.length - 1] : null;
const previousScripts = new Set(Array.isArray(previous?.scripts) ? previous.scripts : []);
const newScripts = previous ? scripts.filter((name) => !previousScripts.has(name)) : [];
const removedScripts = previous ? Array.from(previousScripts).filter((name) => !scripts.includes(String(name))) : [];
const violations = [];
const warnings = [];
if (scripts.length > Number(thresholds.maximum_package_scripts || 1000)) violations.push({ kind: 'package_script_surface_above_threshold', actual: scripts.length, maximum: Number(thresholds.maximum_package_scripts || 1000), next_action: 'Demote compatibility aliases behind command runner or retire unused package scripts.' });
else if (scripts.length > Number(thresholds.package_script_warning_threshold || 1000)) warnings.push({ kind: 'package_script_surface_above_warning_threshold', actual: scripts.length, warning_threshold: Number(thresholds.package_script_warning_threshold || 1000), next_action: 'Continue retiring package-script aliases, but keep the curated command runner as the primary operator surface.' });
if (defaultOperatorCommands > Number(thresholds.maximum_default_operator_commands || 80)) violations.push({ kind: 'default_operator_command_surface_above_threshold', actual: defaultOperatorCommands, maximum: Number(thresholds.maximum_default_operator_commands || 80), next_action: 'Reduce default operator command surface in command registry.' });
if (newScripts.length > Number(thresholds.warn_on_new_scripts_since_last_run || 5)) violations.push({ kind: 'package_script_growth_since_last_run', new_count: newScripts.length, next_action: 'Review new package scripts and decide whether they belong behind curated command metadata.' });
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-command-entropy`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  type: 'kernel_sentinel_command_entropy_report',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyRel,
  package_script_count: scripts.length,
  default_operator_command_count: defaultOperatorCommands,
  new_script_count: newScripts.length,
  removed_script_count: removedScripts.length,
  new_scripts_since_last_run: newScripts.slice(0, 50),
  removed_scripts_since_last_run: removedScripts.slice(0, 50),
  warnings,
  violations,
  scripts,
};
const outPath = path.join(root, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);
const historyPath = path.join(root, historyRel);
fs.mkdirSync(path.dirname(historyPath), { recursive: true });
fs.appendFileSync(historyPath, `${JSON.stringify({ generated_at: report.generated_at, package_script_count: scripts.length, default_operator_command_count: defaultOperatorCommands, scripts })}\n`);
console.log(JSON.stringify({ ok: report.ok, type: report.type, package_script_count: scripts.length, default_operator_command_count: defaultOperatorCommands, new_script_count: newScripts.length, warning_count: warnings.length, violations }, null, 2));
