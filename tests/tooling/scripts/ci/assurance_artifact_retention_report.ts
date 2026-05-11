import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type FileRow = {
  rel: string;
  bytes: number;
  mtime_ms: number;
  prefix: string;
  raw_marker_hits: string[];
};

const root = process.cwd();
const policyRel = "validation/conformance/contracts/assurance_artifact_retention_policy.json";
const policyPath = path.join(root, policyRel);
const outRel = `validation/reports/assurance_artifact_retention_report_${new Date().toISOString().slice(0, 10)}.json`;
const outPath = path.join(root, outRel);

function readJson(rel: string): Json {
  return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
}

function walk(dir: string): string[] {
  const abs = path.join(root, dir);
  if (!fs.existsSync(abs)) return [];
  const out: string[] = [];
  for (const entry of fs.readdirSync(abs, { withFileTypes: true })) {
    const rel = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(rel));
    else out.push(rel);
  }
  return out;
}

function prefixFor(rel: string): string {
  const base = path.basename(rel).replace(/\.(json|jsonl|md|txt)$/i, "");
  return base
    .replace(/_\d{4}-\d{2}-\d{2}.*$/, "")
    .replace(/_\d{8}T\d{6}Z?.*$/, "")
    .replace(/_current$/, "");
}

function markerHits(rel: string, markers: string[]): string[] {
  const abs = path.join(root, rel);
  let text = "";
  try {
    text = fs.readFileSync(abs, "utf8").slice(0, 2_000_000);
  } catch {
    return [];
  }
  return markers.filter((marker) => text.includes(marker));
}

const policy = readJson(policyRel);
const markers = Array.isArray(policy.raw_evidence_markers) ? policy.raw_evidence_markers.map(String) : [];
const roots = Array.isArray(policy.roots) ? policy.roots : [];
const rootReports = roots.map((entry) => {
  const cfg = entry as Json;
  const rootPath = String(cfg.path || "");
  const files: FileRow[] = walk(rootPath)
    .filter((rel) => /\.(json|jsonl|md|txt)$/i.test(rel))
    .map((rel) => {
      const stat = fs.statSync(path.join(root, rel));
      return {
        rel,
        bytes: stat.size,
        mtime_ms: stat.mtimeMs,
        prefix: prefixFor(rel),
        raw_marker_hits: markerHits(rel, markers),
      };
    });
  const totalBytes = files.reduce((sum, row) => sum + row.bytes, 0);
  const maxFileBytes = Number(cfg.max_file_bytes || 0);
  const maxTotalBytes = Number(cfg.max_total_bytes || 0);
  const retainLatest = Number(cfg.retain_latest_per_prefix || 0);
  const byPrefix = new Map<string, FileRow[]>();
  for (const file of files) {
    const rows = byPrefix.get(file.prefix) || [];
    rows.push(file);
    byPrefix.set(file.prefix, rows);
  }
  const cleanupCandidates: FileRow[] = [];
  for (const rows of byPrefix.values()) {
    rows.sort((a, b) => b.mtime_ms - a.mtime_ms);
    cleanupCandidates.push(...rows.slice(retainLatest));
  }
  return {
    path: rootPath,
    file_count: files.length,
    total_bytes: totalBytes,
    max_total_bytes: maxTotalBytes,
    over_total_budget: maxTotalBytes > 0 && totalBytes > maxTotalBytes,
    oversize_files: files.filter((row) => maxFileBytes > 0 && row.bytes > maxFileBytes).map((row) => ({
      rel: row.rel,
      bytes: row.bytes,
    })),
    raw_marker_files: files.filter((row) => row.raw_marker_hits.length > 0).map((row) => ({
      rel: row.rel,
      hits: row.raw_marker_hits,
    })),
    cleanup_candidates: cleanupCandidates.map((row) => ({
      rel: row.rel,
      bytes: row.bytes,
      prefix: row.prefix,
    })),
  };
});

const payload = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention`,
  source_domain: "validation",
  ok: true,
  type: "assurance_artifact_retention_report",
  generated_at: new Date().toISOString(),
  policy_path: policyRel,
  roots: rootReports,
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: payload.type, report_path: outRel, roots: rootReports.length }, null, 2));
