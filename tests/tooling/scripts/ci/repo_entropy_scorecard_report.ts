import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type Severity = "pass" | "white" | "yellow" | "red";

const root = process.cwd();
const policyPath = path.join(root, "validation/scorecards/repo_entropy_scorecard_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as Json;
  } catch {
    return null;
  }
}

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch {
    return "";
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
    if (entry.isFile()) out.push(child);
  }
  return out;
}

function git(args: string[]): string {
  try {
    return execFileSync("git", args, { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  } catch {
    return "";
  }
}

function severityFor(value: number, key: string): Severity {
  const thresholds = policy.thresholds as { red?: Record<string, number>; yellow?: Record<string, number> } | undefined;
  const red = thresholds?.red?.[key] ?? Number.POSITIVE_INFINITY;
  const yellow = thresholds?.yellow?.[key] ?? Number.POSITIVE_INFINITY;
  if (value >= red) return "red";
  if (value >= yellow) return "yellow";
  return value > 0 ? "white" : "pass";
}

function dimension(name: string, metricKey: string, value: number, nextActions: string[]) {
  return {
    name,
    metric_key: metricKey,
    value,
    severity: severityFor(value, metricKey),
    next_actions: nextActions,
  };
}

function countRequiredChecks(): number {
  const report = readJson(path.join(root, "validation/reports/ci_required_gate_reduction_plan_2026-05-10.json"));
  const direct = Number(report?.current_required_count || report?.required_count || 0);
  if (direct > 0) return direct;
  const manifest = readJson(path.join(root, "validation/conformance/contracts/ci_workflow_tier_manifest.json"));
  const rows = Array.isArray(manifest?.workflows) ? manifest?.workflows as Json[] : [];
  return rows.filter((row) => String(row.tier || row.required || "").includes("required")).length;
}

function commandSurface() {
  const packageJson = readJson(path.join(root, "package.json")) as { scripts?: Record<string, string> } | null;
  const registry = readJson(path.join(root, "tools/commands/command_registry.json")) as { commands?: Json[]; entries?: Json[]; metadata_curated_count?: number } | Json[] | null;
  const commands = Array.isArray(registry)
    ? registry
    : Array.isArray(registry?.commands)
      ? registry.commands
      : Array.isArray(registry?.entries)
        ? registry.entries
        : [];
  const compat = commands.filter((row) => Boolean(row.compatibility_alias || row.compat || row.legacy || String(row.status || "").includes("compat") || String(row.lifecycle || "").includes("compat")));
  const curated = commands.filter((row) => Boolean(row.curated || row.description || row.work_gate));
  return {
    npm_scripts: Object.keys(packageJson?.scripts || {}).length,
    command_entries: commands.length,
    compat_command_entries: compat.length,
    curated_command_entries: Number(registry && !Array.isArray(registry) ? registry.metadata_curated_count || 0 : 0) || curated.length,
  };
}

function trackedLoc() {
  const locPolicy = policy.tracked_loc as { extensions?: string[]; exclude_prefixes?: string[]; max_file_bytes?: number } | undefined;
  const extensions = new Set(locPolicy?.extensions || []);
  const excludes = locPolicy?.exclude_prefixes || [];
  const maxFileBytes = locPolicy?.max_file_bytes || 2_097_152;
  const files = git(["ls-files"]).split(/\r?\n/).filter(Boolean);
  let effectiveFiles = 0;
  let effectiveLoc = 0;
  for (const rel of files) {
    if (excludes.some((prefix) => rel.startsWith(prefix))) continue;
    if (!extensions.has(path.extname(rel))) continue;
    const full = path.join(root, rel);
    let stat: fs.Stats;
    try {
      stat = fs.statSync(full);
    } catch {
      continue;
    }
    if (!stat.isFile() || stat.size > maxFileBytes) continue;
    effectiveFiles += 1;
    effectiveLoc += readText(full).split(/\r?\n/).length;
  }
  return { effective_files: effectiveFiles, effective_loc: effectiveLoc };
}

const dirtyRows = git(["status", "--porcelain=v1"]).split(/\r?\n/).filter(Boolean);
const workflowFiles = listFiles(path.join(root, ".github/workflows")).filter((file) => /\.(ya?ml)$/i.test(file));
const artifactFiles = listFiles(path.join(root, "core/local/artifacts"));
const guardFiles = listFiles(path.join(root, "tests/tooling/scripts/ci")).filter((file) => /guard\.(ts|js)$/i.test(file));
const duplicateSurfaceRoots = [
  fs.existsSync(path.join(root, "orchestration")),
  fs.existsSync(path.join(root, "surface/orchestration")),
  fs.existsSync(path.join(root, "client")),
  fs.existsSync(path.join(root, "shell")),
].filter(Boolean).length >= 4 ? 2 : 0;
const commands = commandSurface();
const loc = trackedLoc();
const dimensions = [
  dimension("worktree_churn", "dirty_paths", dirtyRows.length, [
    "Use the safe commit workspace tool before commits.",
    "Split unrelated work into narrow temp-index commits or park it before risky operations.",
  ]),
  dimension("command_surface", "npm_scripts", commands.npm_scripts, [
    "Move operator usage to tools/commands/command_runner.ts.",
    "Keep compatibility aliases hidden unless explicitly requested.",
  ]),
  dimension("command_compat_surface", "compat_command_entries", commands.compat_command_entries, [
    "Curate high-value commands first and leave compatibility aliases behind command-runner opt-in.",
  ]),
  dimension("ci_surface", "required_ci_checks", countRequiredChecks(), [
    "Demote advisory/nightly checks out of branch protection.",
    "Keep required checks focused on release-blocking safety.",
  ]),
  dimension("artifact_pressure", "core_local_artifacts", artifactFiles.length, [
    "Retain compact latest refs and expire bulky local artifacts.",
    "Keep raw evidence in streams, not final scorecards.",
  ]),
  dimension("guard_surface", "guard_scripts", guardFiles.length, [
    "Register guard ownership and retire duplicate/stale guards.",
  ]),
  dimension("duplicate_surfaces", "duplicate_surface_roots", duplicateSurfaceRoots, [
    "Keep duplicate roots explicitly transitional with expiry or compatibility status.",
  ]),
  {
    name: "tracked_loc",
    metric_key: "effective_loc",
    value: loc.effective_loc,
    severity: "white" as Severity,
    next_actions: [
      "Track LOC deltas over time; optimize for useful contraction, not arbitrary shrinkage.",
    ],
  },
];
const severityRank: Record<Severity, number> = { pass: 0, white: 1, yellow: 2, red: 3 };
const worst = dimensions.reduce<Severity>((acc, row) => severityRank[row.severity as Severity] > severityRank[acc] ? row.severity as Severity : acc, "pass");
const redDimensions = dimensions.filter((row) => row.severity === "red").map((row) => row.name);
const traceId = `validation:${new Date().toISOString()}:repo-entropy-scorecard`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "validation",
  type: "repo_entropy_scorecard",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  severity: worst,
  red_dimensions: redDimensions,
  dimensions,
  summary: {
    dirty_paths: dirtyRows.length,
    workflow_files: workflowFiles.length,
    required_ci_checks: countRequiredChecks(),
    npm_scripts: commands.npm_scripts,
    command_entries: commands.command_entries,
    compat_command_entries: commands.compat_command_entries,
    curated_command_entries: commands.curated_command_entries,
    core_local_artifacts: artifactFiles.length,
    guard_scripts: guardFiles.length,
    effective_files: loc.effective_files,
    effective_loc: loc.effective_loc,
  },
};
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/repo_entropy_scorecard_current.json"));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
