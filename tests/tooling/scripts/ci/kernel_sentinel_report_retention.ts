import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type Target = { dir?: string; prefix?: string; suffix?: string; retain_latest?: number; max_age_ms?: number };

const root = process.cwd();
const policyRel = "observability/sentinel/sentinel_report_retention_policy.json";
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), "utf8")) as Json;
const outputRel = String(policy.output_path || "core/local/artifacts/kernel_sentinel_report_retention_current.json");
const archiveDirRel = String(policy.archive_dir || "observability/reports/archive/sentinel");
const apply = process.argv.includes("--apply=1");
const strict = process.argv.includes("--strict=1");
const targets = Array.isArray(policy.targets) ? (policy.targets as Target[]) : [];

function writeJson(rel: string, payload: unknown): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function fileRows(target: Target) {
  const dirRel = String(target.dir || "");
  const dir = path.join(root, dirRel);
  const prefix = String(target.prefix || "");
  const suffix = String(target.suffix || "");
  try {
    return fs
      .readdirSync(dir)
      .filter((name) => name.startsWith(prefix) && name.endsWith(suffix))
      .map((name) => {
        const rel = path.join(dirRel, name);
        const stat = fs.statSync(path.join(root, rel));
        return { rel, mtimeMs: stat.mtimeMs, ageMs: Math.max(0, Date.now() - stat.mtimeMs), size_bytes: stat.size };
      })
      .sort((a, b) => b.mtimeMs - a.mtimeMs);
  } catch {
    return [];
  }
}

const archiveCandidates: Array<{ rel: string; archive_rel: string; age_ms: number; size_bytes: number; reason: string }> = [];
for (const target of targets) {
  const retainLatest = Math.max(1, Number(target.retain_latest || 7));
  const maxAgeMs = Math.max(1, Number(target.max_age_ms || 2_592_000_000));
  const rows = fileRows(target);
  rows.forEach((row, index) => {
    if (index < retainLatest && row.ageMs <= maxAgeMs) return;
    const archiveRel = path.join(archiveDirRel, path.basename(row.rel));
    archiveCandidates.push({
      rel: row.rel,
      archive_rel: archiveRel,
      age_ms: Math.round(row.ageMs),
      size_bytes: row.size_bytes,
      reason: index >= retainLatest ? "beyond_retain_latest" : "older_than_max_age",
    });
  });
}

const moved: string[] = [];
if (apply) {
  for (const row of archiveCandidates) {
    const src = path.join(root, row.rel);
    const dest = path.join(root, row.archive_rel);
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    if (fs.existsSync(src)) {
      fs.renameSync(src, dest);
      moved.push(row.archive_rel);
    }
  }
}

const result = {
  trace_id: `observability:${new Date().toISOString()}:kernel-sentinel-report-retention`,
  source_domain: "observability",
  type: "kernel_sentinel_report_retention",
  generated_at: new Date().toISOString(),
  ok: true,
  policy_path: policyRel,
  mode: apply ? "apply" : "dry_run",
  archive_candidate_count: archiveCandidates.length,
  moved_count: moved.length,
  archive_candidates: archiveCandidates.slice(0, 25),
  moved,
};

writeJson(outputRel, result);
console.log(JSON.stringify(result, null, 2));
if (strict && result.ok !== true) process.exitCode = 1;
