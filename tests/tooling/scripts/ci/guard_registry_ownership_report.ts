import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;

const root = process.cwd();
const policyRelPath = 'validation/conformance/contracts/guard_registry_ownership_policy.json';
const policyPath = path.join(root, policyRelPath);
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Json;

function rel(filePath: string): string {
  return path.relative(root, filePath).replace(/\\/g, '/');
}

function readJson(relativePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8')) as Json;
  } catch {
    return null;
  }
}

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
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

function normalizedFamily(filePath: string): string {
  const suffixes = (policy.duplicate_stem_suffixes as string[]) || [];
  let out = path.basename(filePath).replace(/\.(ts|js)$/i, '').replace(/[-_]/g, '_');
  let changed = true;
  while (changed) {
    changed = false;
    for (const suffix of suffixes) {
      const normalizedSuffix = suffix.replace(/[-_]/g, '_');
      if (out.endsWith(normalizedSuffix)) {
        out = out.slice(0, -normalizedSuffix.length);
        changed = true;
      }
    }
  }
  return out || path.basename(filePath);
}

function packageScriptReferences(): string[] {
  const packageJson = readJson(String(policy.registry_inputs?.package_json || 'package.json')) as { scripts?: Record<string, string> } | null;
  const scripts = packageJson?.scripts || {};
  return Object.entries(scripts).flatMap(([name, command]) => [name, command]);
}

function gateRegistryReferences() {
  const registry = readJson(String(policy.registry_inputs?.tooling_gate_registry || 'tests/tooling/config/tooling_gate_registry.json'));
  const gates = registry?.gates && typeof registry.gates === 'object' ? registry.gates : {};
  const references: string[] = [];
  const artifactPaths = new Set<string>();
  for (const [id, gate] of Object.entries(gates)) {
    references.push(id, JSON.stringify(gate));
    for (const artifact of (gate as Json).artifact_paths || []) artifactPaths.add(String(artifact).replace(/\\/g, '/'));
  }
  return { references, artifactPaths, gateCount: Object.keys(gates).length };
}

function commandRegistryReferences(): string[] {
  const registry = readJson(String(policy.registry_inputs?.command_registry || 'tools/commands/command_registry.json'));
  const entries = Array.isArray(registry?.entries) ? registry.entries : Array.isArray(registry?.commands) ? registry.commands : [];
  return entries.flatMap((entry: Json) => [entry.id, entry.name, entry.command, JSON.stringify(entry)].filter(Boolean).map(String));
}

function releaseManifestArtifactPaths(): Set<string> {
  const manifest = readJson(String(policy.registry_inputs?.release_proof_pack_manifest || 'validation/release_gates/contracts/release_proof_pack_manifest.json'));
  const out = new Set<string>();
  for (const artifact of manifest?.required_artifacts || []) out.add(String(artifact).replace(/\\/g, '/'));
  for (const rows of Object.values(manifest?.artifact_groups || {})) {
    if (Array.isArray(rows)) for (const artifact of rows) out.add(String(artifact).replace(/\\/g, '/'));
  }
  return out;
}

function hasReference(references: string[], needle: string): boolean {
  const normalized = needle.replace(/\\/g, '/');
  const base = path.basename(normalized);
  return references.some((row) => String(row || '').replace(/\\/g, '/').includes(normalized) || String(row || '').includes(base));
}

function isGuardLikeScript(filePath: string): boolean {
  if (!/\.(ts|js)$/i.test(filePath)) return false;
  const base = path.basename(filePath).toLowerCase();
  const patterns = (policy.script_name_patterns as string[]) || ['guard'];
  return patterns.some((pattern) => base.includes(pattern.toLowerCase()));
}

function isGuardLikeArtifact(filePath: string): boolean {
  if (!filePath.endsWith('.json')) return false;
  const relative = rel(filePath);
  const patterns = (policy.artifact_name_patterns as string[]) || ['_guard_current.json'];
  return patterns.some((pattern) => relative.endsWith(pattern));
}

function markdownEscape(raw: unknown): string {
  return String(raw ?? '').replace(/\\/g, '\\\\').replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

function markdownFor(report: Json): string {
  const lines = [
    '# Guard Registry Ownership Report',
    '',
    `- generated_at: ${report.generated_at}`,
    `- severity: ${report.severity}`,
    `- guard_count: ${report.guard_count}`,
    `- registered_guard_count: ${report.registered_guard_count}`,
    `- unregistered_guard_count: ${report.unregistered_guard_count}`,
    `- missing_ownership_count: ${report.missing_ownership_count}`,
    `- duplicate_family_count: ${report.duplicate_family_count}`,
    `- orphan_artifact_count: ${report.orphan_artifact_count}`,
    '',
    '## Top Findings',
    '',
    '| Kind | Path / Family | Detail | Next action |',
    '|---|---|---|---|',
  ];
  for (const finding of report.findings || []) {
    lines.push(`| ${markdownEscape(finding.kind)} | ${markdownEscape(finding.path || finding.family || finding.artifact)} | ${markdownEscape(finding.detail || finding.count || '')} | ${markdownEscape(finding.next_action)} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

const markers = (policy.required_source_markers_any as string[]) || [];
const guardRoots = (policy.guard_roots as string[]) || [];
const artifactRoots = (policy.artifact_roots as string[]) || [];
const packageRefs = packageScriptReferences();
const gateRefs = gateRegistryReferences();
const commandRefs = commandRegistryReferences();
const releaseArtifactPaths = releaseManifestArtifactPaths();
const allRefs = [...packageRefs, ...gateRefs.references, ...commandRefs];
const declaredArtifacts = new Set([...gateRefs.artifactPaths, ...releaseArtifactPaths]);

const guardFiles = guardRoots
  .flatMap((relativeRoot) => listFiles(path.join(root, relativeRoot)))
  .filter(isGuardLikeScript)
  .sort();

const rows = guardFiles.map((file) => {
  const relative = rel(file);
  const source = readText(file);
  const matchedMarkers = markers.filter((marker) => source.includes(marker));
  const registered_by_package = hasReference(packageRefs, relative);
  const registered_by_gate_registry = hasReference(gateRefs.references, relative);
  const registered_by_command_registry = hasReference(commandRefs, relative);
  const registered = registered_by_package || registered_by_gate_registry || registered_by_command_registry;
  return {
    path: relative,
    family: normalizedFamily(file),
    has_ownership_marker: matchedMarkers.length > 0,
    matched_markers: matchedMarkers,
    registered,
    registered_by_package,
    registered_by_gate_registry,
    registered_by_command_registry,
  };
});

const byFamily = new Map<string, Json[]>();
for (const row of rows) byFamily.set(row.family, [...(byFamily.get(row.family) || []), row]);
const duplicateFamilies = [...byFamily.entries()]
  .filter(([, familyRows]) => familyRows.length > 1)
  .map(([family, familyRows]) => ({ family, count: familyRows.length, paths: familyRows.map((row) => row.path) }));
const missingOwnership = rows.filter((row) => !row.has_ownership_marker);
const unregistered = rows.filter((row) => !row.registered);
const stale = rows.filter((row) => !row.registered && !row.has_ownership_marker);

const artifactRows = artifactRoots
  .flatMap((relativeRoot) => listFiles(path.join(root, relativeRoot)))
  .filter(isGuardLikeArtifact)
  .map((file) => {
    const relative = rel(file);
    return {
      path: relative,
      declared: declaredArtifacts.has(relative) || hasReference(allRefs, relative),
      bytes: fs.statSync(file).size,
    };
  })
  .sort((a, b) => a.path.localeCompare(b.path));
const orphanArtifacts = artifactRows.filter((row) => !row.declared);

const findingLimit = Number(policy.finding_limits?.max_findings_in_report || 250);
const findings = [
  ...missingOwnership.map((row) => ({
    kind: 'guard_missing_ownership_marker',
    path: row.path,
    detail: `family=${row.family}`,
    owner_guess: 'validation',
    next_action: 'Add a source_domain, policy path, owner_domain, or Layer ownership marker.',
  })),
  ...unregistered.map((row) => ({
    kind: 'unregistered_guard_script',
    path: row.path,
    detail: `family=${row.family}`,
    owner_guess: 'validation',
    next_action: 'Register this guard in package scripts and tooling_gate_registry, or retire it if it is stale.',
  })),
  ...stale.map((row) => ({
    kind: 'stale_guard_candidate',
    path: row.path,
    detail: `family=${row.family}`,
    owner_guess: 'validation',
    next_action: 'Either wire this guard into the operator/gate surface or delete/archive it as stale.',
  })),
  ...duplicateFamilies.map((row) => ({
    kind: 'duplicate_guard_family',
    family: row.family,
    count: row.count,
    paths: row.paths,
    next_action: 'Decide whether these are distinct tiers or merge/retire duplicate guard surfaces.',
  })),
  ...orphanArtifacts.map((row) => ({
    kind: 'orphan_guard_artifact',
    artifact: row.path,
    detail: `bytes=${row.bytes}`,
    next_action: 'Add this artifact to a registered gate or release manifest, or clean it with artifact retention.',
  })),
].slice(0, findingLimit);

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:guard-registry-ownership`;
const severity = stale.length > 0 || orphanArtifacts.length > 0 ? 'yellow' : missingOwnership.length > 0 || duplicateFamilies.length > 0 || unregistered.length > 0 ? 'white' : 'pass';
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'guard_registry_ownership_report',
  generated_at: generatedAt,
  policy_path: policyRelPath,
  severity,
  gate_registry_count: gateRefs.gateCount,
  guard_count: rows.length,
  registered_guard_count: rows.filter((row) => row.registered).length,
  unregistered_guard_count: unregistered.length,
  stale_guard_candidate_count: stale.length,
  owned_guard_count: rows.length - missingOwnership.length,
  missing_ownership_count: missingOwnership.length,
  duplicate_family_count: duplicateFamilies.length,
  guard_artifact_count: artifactRows.length,
  orphan_artifact_count: orphanArtifacts.length,
  findings,
  duplicate_families: duplicateFamilies,
  orphan_artifacts: orphanArtifacts.slice(0, Number(policy.finding_limits?.max_rows_in_report || 500)),
  rows: rows.slice(0, Number(policy.finding_limits?.max_rows_in_report || 500)),
};

const reportPath = path.join(root, String(policy.report_path || 'core/local/artifacts/guard_registry_ownership_current.json'));
const markdownPath = path.join(root, String(policy.markdown_report_path || 'local/workspace/reports/GUARD_REGISTRY_OWNERSHIP_CURRENT.md'));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.mkdirSync(path.dirname(markdownPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
fs.writeFileSync(markdownPath, markdownFor(report));
console.log(JSON.stringify(report, null, 2));
