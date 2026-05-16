import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type StatusRow = { status: string; path: string };

const root = process.cwd();
const outRel = flag("out-json", "core/local/artifacts/kernel_sentinel_worktree_danger_current.json");
const strict = process.argv.includes("--strict=1");
const trackedChurnWarningThreshold = Number(flag("tracked-churn-warning-threshold", "20"));
const untrackedChurnWarningThreshold = Number(flag("untracked-churn-warning-threshold", "200"));
const canonicalDocs = ["README.md", "ARCHITECTURE.md"];

function flag(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] || fallback : fallback;
}

function run(command: string): string {
  try {
    return execSync(command, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  } catch {
    return "";
  }
}

function writeJson(rel: string, payload: unknown): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function statusRows(includeUntracked: boolean): StatusRow[] {
  const flag = includeUntracked ? "-uall" : "-uno";
  const raw = run(`git status --porcelain=v1 ${flag}`);
  if (!raw) return [];
  return raw
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => ({ status: line.slice(0, 2), path: line.slice(3).trim() }));
}

function readChurnArtifact(): Json | null {
  const rel = "core/local/artifacts/churn_guard_current.json";
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function revCount(): { behind: number; ahead: number } {
  const raw = run("git rev-list --left-right --count origin/main...HEAD");
  const [behindRaw, aheadRaw] = raw.split(/\s+/);
  return { behind: Number(behindRaw || 0), ahead: Number(aheadRaw || 0) };
}

function rowDeleted(row: StatusRow): boolean {
  return row.status.includes("D");
}

function finding(id: string, severity: string, summary: string, evidenceRefs: string[], nextAction: string): Json {
  return {
    id,
    severity,
    actionable: true,
    owner_guess: "observability/sentinel",
    root_cause_cluster_key: "repo_worktree_integrity",
    root_cause_hypothesis: summary,
    evidence_refs: evidenceRefs,
    next_action: nextAction,
  };
}

const trackedRows = statusRows(false);
const allRows = statusRows(true);
const untrackedRows = allRows.filter((row) => row.status === "??");
const deletedCanonicalDocs = trackedRows
  .filter(rowDeleted)
  .map((row) => row.path)
  .filter((rowPath) => canonicalDocs.includes(rowPath));
const churnArtifact = readChurnArtifact();
const churnSummary = (churnArtifact?.summary || {}) as Json;
const churnFailures = Array.isArray(churnArtifact?.failures) ? (churnArtifact.failures as Json[]) : [];
const revisions = revCount();
const findings: Json[] = [];

if (deletedCanonicalDocs.length > 0) {
  findings.push(
    finding(
      "canonical_docs_deleted_in_worktree",
      "critical",
      "Canonical repo docs are deleted in the working tree, which makes architecture and operator guidance locally unsafe.",
      deletedCanonicalDocs,
      "Restore README.md and ARCHITECTURE.md from HEAD or intentionally replace them in a scoped docs commit.",
    ),
  );
}

if (trackedRows.length >= trackedChurnWarningThreshold) {
  findings.push(
    finding(
      "tracked_worktree_churn_above_sentinel_threshold",
      "high",
      `Tracked worktree churn is ${trackedRows.length}, above the Sentinel threshold ${trackedChurnWarningThreshold}.`,
      ["git status --porcelain=v1 -uno"],
      "Split the worktree into scoped commits or isolate unrelated work in a separate worktree before continuing.",
    ),
  );
}

if (untrackedRows.length >= untrackedChurnWarningThreshold) {
  findings.push(
    finding(
      "untracked_worktree_churn_above_sentinel_threshold",
      "medium",
      `Untracked worktree churn is ${untrackedRows.length}, above the Sentinel threshold ${untrackedChurnWarningThreshold}.`,
      ["git status --porcelain=v1 -uall"],
      "Archive, ignore, or relocate generated work products so active repo health is readable.",
    ),
  );
}

if (Number(churnSummary.blocking_likely_unstaged_moves || 0) > 0) {
  findings.push(
    finding(
      "push_blocking_likely_unstaged_moves",
      "high",
      "The churn guard detected likely unstaged moves that can block safe pushes or cause accidental deletions.",
      ["core/local/artifacts/churn_guard_current.json"],
      "Resolve or intentionally stage the move pairs before pushing.",
    ),
  );
}

if (churnArtifact && churnArtifact.ok === false) {
  findings.push(
    finding(
      "churn_guard_currently_failing",
      "high",
      "The current churn guard artifact reports failure, so Sentinel should not call the workspace healthy.",
      ["core/local/artifacts/churn_guard_current.json"],
      "Use the churn report to reduce dirty state, then rerun the guard before promotion or release.",
    ),
  );
}

if (revisions.behind > 0 || revisions.ahead > 0) {
  findings.push(
    finding(
      "branch_divergence_present",
      revisions.behind > 0 ? "high" : "medium",
      `Branch divergence detected: ahead=${revisions.ahead}, behind=${revisions.behind}.`,
      ["git rev-list --left-right --count origin/main...HEAD"],
      revisions.behind > 0
        ? "Rebase/merge remote main in a clean worktree before pushing more work."
        : "Push or document local commits so Sentinel's view matches remote repo state.",
    ),
  );
}

const result = {
  trace_id: `observability:${new Date().toISOString()}:kernel-sentinel-worktree-danger`,
  source_domain: "observability",
  type: "kernel_sentinel_worktree_danger_report",
  generated_at: new Date().toISOString(),
  ok: findings.length === 0,
  thresholds: {
    tracked_churn_warning: trackedChurnWarningThreshold,
    untracked_churn_warning: untrackedChurnWarningThreshold,
  },
  summary: {
    tracked_churn: trackedRows.length,
    untracked_churn: untrackedRows.length,
    deleted_canonical_docs: deletedCanonicalDocs,
    ahead: revisions.ahead,
    behind: revisions.behind,
    churn_guard_ok: churnArtifact?.ok ?? null,
    churn_guard_failures: churnFailures.map((row) => row.id || row.kind || "unknown").slice(0, 10),
  },
  finding_count: findings.length,
  findings: findings.slice(0, 10),
};

writeJson(outRel, result);
console.log(JSON.stringify(result, null, 2));
if (strict && !result.ok) process.exitCode = 1;
