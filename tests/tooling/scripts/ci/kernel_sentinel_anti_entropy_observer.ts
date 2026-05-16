#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

const root = process.cwd();
const strict = process.argv.includes('--strict=1');
const artifactDir = path.join(root, 'core/local/artifacts');
const reportDir = path.join(root, 'local/workspace/reports');
const policyPath = path.join(root, 'observability/sentinel/sentinel_anti_entropy_observer_policy.json');
const outPath = path.join(artifactDir, 'kernel_sentinel_anti_entropy_observer_current.json');
const mdPath = path.join(reportDir, 'KERNEL_SENTINEL_ANTI_ENTROPY_OBSERVER_CURRENT.md');

type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';
type Finding = {
  id: string;
  severity: Severity;
  domain: string;
  summary: string;
  root_cause_hypothesis: string;
  next_action: string;
  owner_guess: string;
  evidence_refs: string[];
};

type Json = Record<string, any>;

function rel(p: string): string {
  return path.relative(root, p).split(path.sep).join('/');
}

function readJson(file: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8')) as Json;
  } catch {
    return null;
  }
}

function readText(file: string): string | null {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return null;
  }
}

function artifactAgeHours(file: string): number | null {
  try {
    const stat = fs.statSync(file);
    return (Date.now() - stat.mtimeMs) / 3_600_000;
  } catch {
    return null;
  }
}

function addFinding(findings: Finding[], finding: Finding) {
  if (!finding.evidence_refs.length) {
    finding.evidence_refs.push('missing_evidence_ref');
  }
  findings.push(finding);
}

function countWorkflowFiles(): number {
  const workflowDir = path.join(root, '.github/workflows');
  try {
    return fs.readdirSync(workflowDir).filter((name) => /\.ya?ml$/i.test(name)).length;
  } catch {
    return 0;
  }
}

function countRequiredWorkflows(tierManifest: Json | null): number {
  if (!tierManifest) return 0;
  const rows = Array.isArray(tierManifest.workflows) ? tierManifest.workflows : [];
  return rows.filter((row: Json) => row.required === true || row.branch_protection_required === true || row.tier === 'required').length;
}

function writeMarkdown(report: Json, findings: Finding[]) {
  const lines = [
    '# Kernel Sentinel Anti-Entropy Observer',
    '',
    `Generated: ${report.generated_at}`,
    '',
    `Status: ${report.ok ? 'ok' : 'needs_attention'}`,
    `Anti-entropy score: ${report.anti_entropy_score}`,
    `Findings: ${findings.length}`,
    '',
    '## Top Findings',
    '',
  ];
  if (!findings.length) {
    lines.push('- No anti-entropy findings crossed the configured thresholds.');
  } else {
    for (const finding of findings.slice(0, 10)) {
      lines.push(`- ${finding.severity.toUpperCase()} ${finding.id}: ${finding.summary}`);
      lines.push(`  Next: ${finding.next_action}`);
      lines.push(`  Evidence: ${finding.evidence_refs.join(', ')}`);
    }
  }
  lines.push('', '## Coverage', '', '```json', JSON.stringify(report.coverage, null, 2), '```', '');
  fs.mkdirSync(path.dirname(mdPath), { recursive: true });
  fs.writeFileSync(mdPath, `${lines.join('\n')}\n`);
}

const policy = readJson(policyPath) ?? {};
const thresholds = policy.thresholds ?? {};
const weights = policy.severity_weights ?? { critical: 35, high: 20, medium: 10, low: 5, info: 0 };
const findings: Finding[] = [];

const artifactFiles = Object.keys(policy.freshness_budgets_hours ?? {});
const artifactHealth = artifactFiles.map((name) => {
  const file = path.join(artifactDir, name);
  const ageHours = artifactAgeHours(file);
  const budget = Number(policy.freshness_budgets_hours?.[name] ?? 168);
  const json = readJson(file);
  const exists = ageHours !== null;
  const fresh = exists && ageHours <= budget;
  if (!exists || !fresh) {
    addFinding(findings, {
      id: exists ? `stale_artifact_${name.replace(/[^a-zA-Z0-9]+/g, '_')}` : `missing_artifact_${name.replace(/[^a-zA-Z0-9]+/g, '_')}`,
      severity: name.includes('worktree') || name.includes('feedback') ? 'high' : 'medium',
      domain: 'stale_artifacts',
      summary: exists
        ? `${name} is older than its ${budget}h freshness budget.`
        : `${name} is missing, leaving Sentinel with incomplete anti-entropy context.`,
      root_cause_hypothesis: 'Automatic dream/heartbeat maintenance is not refreshing all Sentinel source artifacts consistently.',
      next_action: `Refresh or wire the producer for ${name}, then rerun ops:ksent:anti-entropy:observer.`,
      owner_guess: 'observability/sentinel',
      evidence_refs: [rel(file)]
    });
  }
  return { name, exists, age_hours: ageHours, freshness_budget_hours: budget, fresh, ok: json?.ok };
});

const worktreeDangerPath = path.join(artifactDir, 'kernel_sentinel_worktree_danger_current.json');
const worktreeDanger = readJson(worktreeDangerPath);
if (worktreeDanger && worktreeDanger.ok === false) {
  addFinding(findings, {
    id: 'worktree_danger_active',
    severity: 'high',
    domain: 'worktree_risk',
    summary: `Worktree danger report is active with ${worktreeDanger.finding_count ?? 'unknown'} findings.`,
    root_cause_hypothesis: 'Local repo churn or branch divergence is high enough to make accidental bad commits more likely.',
    next_action: 'Inspect the worktree danger report, split unrelated changes, and stabilize the branch before broad edits.',
    owner_guess: 'repo_hygiene',
    evidence_refs: [rel(worktreeDangerPath)]
  });
}

const anchors = Array.isArray(policy.canonical_architecture_anchors) ? policy.canonical_architecture_anchors : [];
const anchorHealth = anchors.map((anchor: string) => ({ path: anchor, exists: fs.existsSync(path.join(root, anchor)) }));
const missingAnchors = anchorHealth.filter((row: Json) => !row.exists);
if (missingAnchors.length) {
  addFinding(findings, {
    id: 'canonical_architecture_anchors_missing',
    severity: 'critical',
    domain: 'architecture_anchor_health',
    summary: `${missingAnchors.length} canonical architecture anchors are missing.`,
    root_cause_hypothesis: 'Repo cleanup or branch churn removed documents that act as high-level architectural source-of-truth.',
    next_action: `Restore or intentionally replace: ${missingAnchors.map((row: Json) => row.path).join(', ')}.`,
    owner_guess: 'governance/docs',
    evidence_refs: missingAnchors.map((row: Json) => row.path)
  });
}

const packageJson = readJson(path.join(root, 'package.json'));
const scriptCount = packageJson?.scripts ? Object.keys(packageJson.scripts).length : 0;
if (scriptCount > Number(thresholds.maximum_package_scripts_before_entropy_finding ?? 1000)) {
  addFinding(findings, {
    id: 'package_script_surface_above_entropy_threshold',
    severity: 'medium',
    domain: 'command_surface_entropy',
    summary: `package.json exposes ${scriptCount} scripts, above the anti-entropy threshold.`,
    root_cause_hypothesis: 'Compatibility aliases and operational commands are accumulating faster than the curated command surface can absorb them.',
    next_action: 'Continue moving human/agent entrypoints behind the curated command runner and demote aliases from the default surface.',
    owner_guess: 'tools/commands',
    evidence_refs: ['package.json', 'tools/commands/command_registry.json']
  });
}

const commandGuardPath = path.join(artifactDir, 'command_operator_surface_guard_current.json');
const commandGuard = readJson(commandGuardPath);
const defaultCommandCount = Number(commandGuard?.summary?.default_operator_command_count ?? commandGuard?.default_operator_command_count ?? 0);
if (defaultCommandCount > Number(thresholds.maximum_default_operator_commands ?? 80)) {
  addFinding(findings, {
    id: 'default_operator_command_surface_too_large',
    severity: 'high',
    domain: 'command_surface_entropy',
    summary: `Default operator command surface has ${defaultCommandCount} commands.`,
    root_cause_hypothesis: 'The command runner is exposing too many commands by default, making agent/operator choice noisier.',
    next_action: 'Compress the default operator command surface back under policy limit.',
    owner_guess: 'tools/commands',
    evidence_refs: [rel(commandGuardPath)]
  });
}

const tierManifestPath = path.join(root, 'validation/conformance/contracts/ci_workflow_tier_manifest.json');
const tierManifest = readJson(tierManifestPath);
const workflowCount = countWorkflowFiles();
const requiredWorkflowCount = countRequiredWorkflows(tierManifest);
if (workflowCount > Number(thresholds.maximum_total_ci_workflows ?? 45)) {
  addFinding(findings, {
    id: 'ci_workflow_count_above_entropy_threshold',
    severity: 'medium',
    domain: 'ci_surface_sprawl',
    summary: `${workflowCount} GitHub workflow files are present, above the configured anti-entropy threshold.`,
    root_cause_hypothesis: 'CI grew as local guard coverage grew, but workflow count has not been compressed into tiered reusable lanes.',
    next_action: 'Consolidate low-value or duplicate workflows into tiered CI runners and keep branch protection focused on required gates.',
    owner_guess: 'validation/conformance',
    evidence_refs: ['.github/workflows', rel(tierManifestPath)]
  });
}
if (requiredWorkflowCount > Number(thresholds.maximum_required_ci_workflows ?? 30)) {
  addFinding(findings, {
    id: 'required_ci_workflow_count_above_policy',
    severity: 'high',
    domain: 'ci_surface_sprawl',
    summary: `${requiredWorkflowCount} workflows appear required by the tier manifest.`,
    root_cause_hypothesis: 'Branch-protection pressure may be too broad, creating red-noise instead of release confidence.',
    next_action: 'Reduce required checks to the minimum release-critical set and leave the rest advisory or scheduled.',
    owner_guess: 'validation/conformance',
    evidence_refs: [rel(tierManifestPath)]
  });
}

const installerGuardPath = path.join(artifactDir, 'windows_installer_contract_guard_current.json');
const installerGuard = readJson(installerGuardPath);
if (installerGuard && installerGuard.ok === false) {
  addFinding(findings, {
    id: 'windows_installer_contract_guard_failing',
    severity: 'high',
    domain: 'install_health',
    summary: 'Windows installer contract guard is failing.',
    root_cause_hypothesis: 'Installer behavior drifted from the Windows recovery/bootstrap contract.',
    next_action: 'Inspect the installer guard violations and patch install.ps1 before release.',
    owner_guess: 'install',
    evidence_refs: [rel(installerGuardPath)]
  });
}

const proofGuardPath = path.join(artifactDir, 'real_work_workflow_proof_guard_current.json');
const proofGuard = readJson(proofGuardPath);
if (proofGuard && proofGuard.ok === false) {
  addFinding(findings, {
    id: 'real_work_workflow_proof_failing',
    severity: 'high',
    domain: 'real_work_proof_health',
    summary: 'Real-work workflow proof guard is failing.',
    root_cause_hypothesis: 'The repo may be proving governance but not enough practical operator workflows.',
    next_action: 'Restore failing proof lanes or mark obsolete lanes with explicit replacement proof.',
    owner_guess: 'validation/proof_packs',
    evidence_refs: [rel(proofGuardPath)]
  });
}
const readyJourneys = Number(proofGuard?.summary?.ready_operator_journey_count ?? proofGuard?.ready_operator_journey_count ?? 0);
if (proofGuard && readyJourneys < Number(thresholds.minimum_real_work_ready_operator_journeys ?? 2)) {
  addFinding(findings, {
    id: 'real_work_operator_journey_coverage_low',
    severity: 'medium',
    domain: 'real_work_proof_health',
    summary: `Only ${readyJourneys} ready operator journeys are proven.`,
    root_cause_hypothesis: 'The proof pack may still overrepresent internal guards instead of practical work loops.',
    next_action: 'Add or refresh practical operator journeys for install recovery, gateway operation, and repo-safe edits.',
    owner_guess: 'validation/proof_packs',
    evidence_refs: [rel(proofGuardPath)]
  });
}

findings.sort((a, b) => {
  const aw = Number(weights[a.severity] ?? 0);
  const bw = Number(weights[b.severity] ?? 0);
  return bw - aw || a.id.localeCompare(b.id);
});
const penalty = findings.reduce((sum, finding) => sum + Number(weights[finding.severity] ?? 0), 0);
const score = Math.max(0, 100 - penalty);
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-anti-entropy-observer`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: String(worktreeDanger?.trace_id || commandGuard?.trace_id || installerGuard?.trace_id || proofGuard?.trace_id || ''),
  source_domain: 'observability',
  schema_version: 1,
  generated_at: new Date().toISOString(),
  type: 'kernel_sentinel_anti_entropy_observer',
  ok: findings.length === 0,
  status: findings.length === 0 ? 'healthy' : 'anti_entropy_needs_attention',
  anti_entropy_score: score,
  finding_count: findings.length,
  domains_with_findings: Array.from(new Set(findings.map((finding) => finding.domain))).sort(),
  coverage: {
    artifact_health: artifactHealth,
    architecture_anchors: anchorHealth,
    package_script_count: scriptCount,
    default_operator_command_count: defaultCommandCount,
    workflow_count: workflowCount,
    required_workflow_count: requiredWorkflowCount,
    real_work_ready_operator_journeys: readyJourneys
  },
  top_findings: findings.slice(0, 10),
  findings,
  source_refs: [
    rel(policyPath),
    rel(worktreeDangerPath),
    rel(commandGuardPath),
    rel(installerGuardPath),
    rel(proofGuardPath),
    rel(tierManifestPath),
    'package.json'
  ]
};

fs.mkdirSync(artifactDir, { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);
writeMarkdown(report, findings);
console.log(JSON.stringify({ ok: report.ok, status: report.status, anti_entropy_score: report.anti_entropy_score, finding_count: report.finding_count, report: rel(outPath), markdown: rel(mdPath) }, null, 2));
if (strict && !report.ok) process.exit(1);
