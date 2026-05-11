import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;
type Severity = 'pass' | 'white' | 'yellow' | 'red';

type Dimension = {
  name: string;
  metric_key: string;
  value: number;
  severity: Severity;
  details: Json;
  next_actions: string[];
};

const root = process.cwd();
const defaultPolicyPath = 'validation/scorecards/repo_entropy_scorecard_policy.json';

function readFlag(argv: string[], key: string): string | null {
  const prefix = `--${key}=`;
  const row = argv.find((arg) => arg.startsWith(prefix));
  if (row) return row.slice(prefix.length);
  const idx = argv.indexOf(`--${key}`);
  if (idx >= 0 && idx + 1 < argv.length) return argv[idx + 1];
  return null;
}

function parseBool(raw: string | null, fallback = false): boolean {
  if (raw == null || raw === '') return fallback;
  return ['1', 'true', 'yes', 'on'].includes(String(raw).toLowerCase());
}

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(root, filePath), 'utf8')) as Json;
  } catch {
    return null;
  }
}

function readJsonAbs(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as Json;
  } catch {
    return null;
  }
}

function listFiles(dirPath: string): string[] {
  const out: string[] = [];
  let entries: fs.Dirent[] = [];
  try {
    entries = fs.readdirSync(dirPath, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const child = path.join(dirPath, entry.name);
    if (entry.isDirectory()) out.push(...listFiles(child));
    else if (entry.isFile()) out.push(child);
  }
  return out;
}

function safeGit(args: string[]): string {
  try {
    return execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  } catch {
    return '';
  }
}

function currentRevision(): string {
  return safeGit(['rev-parse', 'HEAD']).trim() || 'unknown';
}

function severityFor(value: number, key: string, policy: Json): Severity {
  const red = Number(policy?.thresholds?.red?.[key] ?? Number.POSITIVE_INFINITY);
  const yellow = Number(policy?.thresholds?.yellow?.[key] ?? Number.POSITIVE_INFINITY);
  if (value >= red) return 'red';
  if (value >= yellow) return 'yellow';
  return value > 0 ? 'white' : 'pass';
}

function dimension(policy: Json, name: string, metricKey: string, value: number, details: Json, nextActions: string[]): Dimension {
  return {
    name,
    metric_key: metricKey,
    value,
    severity: severityFor(value, metricKey, policy),
    details,
    next_actions: nextActions,
  };
}

function countRequiredChecks(policy: Json): { required: number; source: string } {
  const artifacts = policy.source_artifacts || {};
  const reduction = readJson(String(artifacts.ci_required_gate_reduction_plan || ''));
  const direct = Number(reduction?.current_required_count ?? reduction?.required_count ?? 0);
  if (direct > 0) return { required: direct, source: String(artifacts.ci_required_gate_reduction_plan) };
  const manifest = readJson(String(artifacts.ci_workflow_tier_manifest || ''));
  const rows = Array.isArray(manifest?.workflows) ? manifest.workflows : [];
  const required = rows.filter((row: Json) => String(row.tier || row.required || '').includes('required')).length;
  return { required, source: String(artifacts.ci_workflow_tier_manifest || 'fallback') };
}

function commandMetrics(policy: Json) {
  const artifacts = policy.source_artifacts || {};
  const packageJson = readJson('package.json') as { scripts?: Record<string, string> } | null;
  const registry = readJson(String(artifacts.command_registry || 'tools/commands/command_registry.json'));
  const entries = Array.isArray(registry?.entries) ? registry.entries : Array.isArray(registry?.commands) ? registry.commands : [];
  const compat = entries.filter((row: Json) => String(row.lifecycle || row.status || '').includes('compat'));
  const operator = entries.filter((row: Json) => String(row.lifecycle || '').includes('operator_surface'));
  return {
    npm_scripts: Object.keys(packageJson?.scripts || {}).length,
    command_entries: entries.length,
    compat_command_entries: compat.length,
    operator_surface_entries: operator.length,
    metadata_curated_count: Number(registry?.metadata_curated_count || 0),
  };
}

function artifactMetrics(policy: Json) {
  const roots = Array.isArray(policy.artifact_roots) ? policy.artifact_roots : ['core/local/artifacts'];
  let count = 0;
  let bytes = 0;
  const byRoot: Json[] = [];
  for (const relRoot of roots) {
    const files = listFiles(path.join(root, relRoot));
    let rootBytes = 0;
    for (const file of files) {
      const stat = fs.statSync(file);
      rootBytes += stat.size;
    }
    count += files.length;
    bytes += rootBytes;
    byRoot.push({ root: relRoot, files: files.length, bytes: rootBytes });
  }
  return { core_local_artifacts: count, core_local_artifact_bytes: bytes, roots: byRoot };
}

function guardMetrics(policy: Json) {
  const roots = Array.isArray(policy.guard_roots) ? policy.guard_roots : ['tests/tooling/scripts/ci'];
  const guardFiles = roots.flatMap((relRoot: string) => listFiles(path.join(root, relRoot))).filter((file) => /(?:guard|audit|gate|scorecard)\.(?:ts|js)$/i.test(file));
  const guardRegistry = readJson(String(policy?.source_artifacts?.guard_registry_ownership || ''));
  const gateRegistry = readJson(String(policy?.source_artifacts?.tooling_gate_registry || 'tests/tooling/config/tooling_gate_registry.json'));
  const gateEntries = gateRegistry?.gates && typeof gateRegistry.gates === 'object' ? Object.keys(gateRegistry.gates).length : 0;
  return {
    guard_scripts: Number(guardRegistry?.guard_count || 0) || guardFiles.length,
    guard_files_detected: guardFiles.length,
    missing_ownership_count: Number(guardRegistry?.missing_ownership_count || 0),
    duplicate_guard_family_count: Number(guardRegistry?.duplicate_family_count || 0),
    gate_registry_entries: gateEntries,
  };
}

function ciMetrics(policy: Json) {
  const workflowRoot = path.join(root, String(policy.workflow_root || '.github/workflows'));
  const workflowFiles = listFiles(workflowRoot).filter((file) => /\.ya?ml$/i.test(file));
  const required = countRequiredChecks(policy);
  return {
    workflow_files: workflowFiles.length,
    required_ci_checks: required.required,
    required_ci_source: required.source,
  };
}

function duplicateSurfaceMetrics(policy: Json) {
  const pairs = Array.isArray(policy.duplicate_surface_pairs) ? policy.duplicate_surface_pairs : [];
  const active = pairs
    .map((row: Json) => {
      const roots = Array.isArray(row.roots) ? row.roots : [];
      const present = roots.filter((relRoot: string) => fs.existsSync(path.join(root, relRoot)));
      return {
        id: row.id,
        owner: row.owner || 'unknown',
        roots,
        present_roots: present,
        duplicate: present.length > 1,
        next_action: row.next_action || 'Declare canonical owner or retire duplicate surface.',
      };
    })
    .filter((row: Json) => row.duplicate);
  return {
    duplicate_surface_roots: active.length,
    active_duplicate_surfaces: active,
  };
}

function effectiveLocMetrics(policy: Json) {
  const locArtifact = readJson(String(policy?.source_artifacts?.effective_loc || 'core/local/artifacts/effective_loc_metric_current.json'));
  const current = Number(locArtifact?.counts?.nonblank_loc || locArtifact?.summary?.effective_loc || 0);
  const currentFiles = Number(locArtifact?.counts?.files || locArtifact?.summary?.effective_files || 0);
  const historyPath = path.resolve(root, String(policy.history_path || 'local/state/validation/repo_entropy_scorecard/history.jsonl'));
  let previous = 0;
  try {
    const rows = fs.readFileSync(historyPath, 'utf8').split(/\r?\n/).filter(Boolean);
    for (let idx = rows.length - 1; idx >= 0; idx -= 1) {
      const parsed = JSON.parse(rows[idx]);
      const candidate = Number(parsed?.summary?.effective_loc || parsed?.effective_loc || 0);
      if (candidate > 0) {
        previous = candidate;
        break;
      }
    }
  } catch {
    previous = 0;
  }
  const delta = previous > 0 ? current - previous : 0;
  const deltaPct = previous > 0 ? Number(((delta / previous) * 100).toFixed(3)) : 0;
  return {
    effective_loc: current,
    effective_files: currentFiles,
    effective_loc_previous: previous,
    effective_loc_delta: delta,
    effective_loc_delta_pct: deltaPct,
    source: locArtifact ? String(policy?.source_artifacts?.effective_loc) : 'missing_effective_loc_artifact',
  };
}

function markdownEscape(raw: unknown): string {
  return String(raw ?? '').replace(/\\/g, '\\\\').replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

function markdownFor(report: Json): string {
  const lines: string[] = [];
  lines.push('# Repo Entropy Scorecard');
  lines.push('');
  lines.push(`- generated_at: ${report.generated_at}`);
  lines.push(`- revision: ${report.revision}`);
  lines.push(`- severity: ${report.severity}`);
  lines.push(`- entropy_score: ${report.entropy_score}`);
  lines.push('');
  lines.push('## Dimensions');
  lines.push('');
  lines.push('| Dimension | Severity | Metric | Value | Next action |');
  lines.push('|---|---:|---|---:|---|');
  for (const row of report.dimensions || []) {
    lines.push(`| ${markdownEscape(row.name)} | ${markdownEscape(row.severity)} | ${markdownEscape(row.metric_key)} | ${markdownEscape(row.value)} | ${markdownEscape((row.next_actions || [])[0] || '')} |`);
  }
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  for (const [key, value] of Object.entries(report.summary || {})) {
    if (typeof value !== 'object') lines.push(`- ${key}: ${value}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeJson(filePath: string, payload: unknown) {
  const abs = path.resolve(root, filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function writeText(filePath: string, payload: string) {
  const abs = path.resolve(root, filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, payload);
}

function appendHistory(filePath: string, payload: unknown) {
  const abs = path.resolve(root, filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.appendFileSync(abs, `${JSON.stringify(payload)}\n`);
}

function run(argv: string[]): number {
  const policyRelPath = readFlag(argv, 'policy') || defaultPolicyPath;
  const policyPath = path.resolve(root, policyRelPath);
  const policy = readJsonAbs(policyPath) || {};
  const strict = argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false);
  const outJson = readFlag(argv, 'out-json') || String(policy.report_path || 'core/local/artifacts/repo_entropy_scorecard_current.json');
  const outMarkdown = readFlag(argv, 'out-markdown') || String(policy.markdown_report_path || 'local/workspace/reports/REPO_ENTROPY_SCORECARD_CURRENT.md');
  const historyPath = readFlag(argv, 'history') || String(policy.history_path || 'local/state/validation/repo_entropy_scorecard/history.jsonl');
  const dirtyRows = safeGit(['status', '--porcelain=v1']).split(/\r?\n/).filter(Boolean);
  const commands = commandMetrics(policy);
  const ci = ciMetrics(policy);
  const artifacts = artifactMetrics(policy);
  const guards = guardMetrics(policy);
  const duplicates = duplicateSurfaceMetrics(policy);
  const loc = effectiveLocMetrics(policy);

  const dimensions: Dimension[] = [
    dimension(policy, 'worktree_churn', 'dirty_paths', dirtyRows.length, { dirty_paths: dirtyRows.length }, [
      'Use safe commit workspace flow before risky operations.',
      'Split unrelated dirty state before release, history rewrite, or large refactors.',
    ]),
    dimension(policy, 'command_surface', 'command_entries', commands.command_entries, commands, [
      'Curate the operator command surface and hide compatibility aliases by default.',
      'Prefer tools/commands command runner over raw npm script discovery.',
    ]),
    dimension(policy, 'ci_surface', 'required_ci_checks', ci.required_ci_checks, ci, [
      'Keep branch protection focused on the small release-blocking CI tier.',
      'Demote advisory/nightly checks out of required status contexts.',
    ]),
    dimension(policy, 'artifact_pressure', 'core_local_artifacts', artifacts.core_local_artifacts, artifacts, [
      'Apply artifact retention and keep large/raw evidence behind compact latest refs.',
      'Move summaries into scorecards and raw streams into retention-managed evidence roots.',
    ]),
    dimension(policy, 'loc_growth', 'effective_loc_delta_pct', Math.max(0, loc.effective_loc_delta_pct), loc, [
      'Track useful contraction and prevent feature work from hiding representation growth.',
      'Use effective LOC deltas as a trend signal, not a standalone quality verdict.',
    ]),
    dimension(policy, 'guard_surface', 'guard_scripts', guards.guard_scripts, guards, [
      'Use guard registry ownership to retire duplicate, stale, or unowned guards.',
      'Group guards into validation domains instead of adding one-off scripts forever.',
    ]),
    dimension(policy, 'duplicate_surfaces', 'duplicate_surface_roots', duplicates.duplicate_surface_roots, duplicates, [
      'Give duplicate roots explicit compatibility status, owner, and retirement date.',
      'Do not allow old and new surfaces to both look canonical indefinitely.',
    ]),
  ];

  const severityWeight: Record<Severity, number> = { pass: 0, white: 1, yellow: 3, red: 8 };
  const rank: Record<Severity, number> = { pass: 0, white: 1, yellow: 2, red: 3 };
  const severity = dimensions.reduce<Severity>((acc, row) => (rank[row.severity] > rank[acc] ? row.severity : acc), 'pass');
  const entropyScore = dimensions.reduce((sum, row) => sum + severityWeight[row.severity], 0);
  const generatedAt = new Date().toISOString();
  const traceId = `validation:${generatedAt}:repo-entropy-scorecard`;
  const report = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: 'validation',
    type: 'repo_entropy_scorecard',
    generated_at: generatedAt,
    revision: currentRevision(),
    policy_path: path.relative(root, policyPath).replace(/\\/g, '/'),
    severity,
    entropy_score: entropyScore,
    red_dimensions: dimensions.filter((row) => row.severity === 'red').map((row) => row.name),
    yellow_dimensions: dimensions.filter((row) => row.severity === 'yellow').map((row) => row.name),
    dimensions,
    summary: {
      dirty_paths: dirtyRows.length,
      npm_scripts: commands.npm_scripts,
      command_entries: commands.command_entries,
      compat_command_entries: commands.compat_command_entries,
      operator_surface_entries: commands.operator_surface_entries,
      workflow_files: ci.workflow_files,
      required_ci_checks: ci.required_ci_checks,
      core_local_artifacts: artifacts.core_local_artifacts,
      core_local_artifact_bytes: artifacts.core_local_artifact_bytes,
      effective_loc: loc.effective_loc,
      effective_files: loc.effective_files,
      effective_loc_delta: loc.effective_loc_delta,
      effective_loc_delta_pct: loc.effective_loc_delta_pct,
      guard_scripts: guards.guard_scripts,
      gate_registry_entries: guards.gate_registry_entries,
      duplicate_surface_roots: duplicates.duplicate_surface_roots,
    },
    artifact_paths: [outJson, outMarkdown],
    strict_mode: strict,
    observational: policy?.policy?.scorecard_is_observational_by_default !== false,
  };

  writeJson(outJson, report);
  writeText(outMarkdown, markdownFor(report));
  appendHistory(historyPath, {
    generated_at: generatedAt,
    revision: report.revision,
    severity,
    entropy_score: entropyScore,
    summary: report.summary,
  });
  console.log(JSON.stringify(report, null, 2));
  return 0;
}

process.exit(run(process.argv.slice(2)));
