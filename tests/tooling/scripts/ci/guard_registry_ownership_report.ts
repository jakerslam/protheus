import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/conformance/contracts/guard_registry_ownership_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;

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

function stem(filePath: string): string {
  return path.basename(filePath).replace(/\.(ts|js)$/i, "").replace(/[_-]guard$/i, "");
}

const markers = (policy.required_source_markers_any as string[]) || [];
const roots = (policy.guard_roots as string[]) || [];
const guardFiles = roots.flatMap((rel) => listFiles(path.join(root, rel))).filter((file) => /guard\.(ts|js)$/i.test(file));
const rows = guardFiles.map((file) => {
  const rel = path.relative(root, file);
  const source = fs.readFileSync(file, "utf8");
  const matchedMarkers = markers.filter((marker) => source.includes(marker));
  return {
    path: rel,
    family: stem(file),
    has_ownership_marker: matchedMarkers.length > 0,
    matched_markers: matchedMarkers,
  };
});
const byFamily = new Map<string, Json[]>();
for (const row of rows) {
  const family = String(row.family || "");
  byFamily.set(family, [...(byFamily.get(family) || []), row]);
}
const duplicateFamilies = [...byFamily.entries()]
  .filter(([, familyRows]) => familyRows.length > 1)
  .map(([family, familyRows]) => ({ family, count: familyRows.length, paths: familyRows.map((row) => row.path) }));
const missingOwnership = rows.filter((row) => !row.has_ownership_marker);
const traceId = `validation:${new Date().toISOString()}:guard-registry-ownership`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "validation",
  type: "guard_registry_ownership_report",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  guard_count: rows.length,
  owned_guard_count: rows.length - missingOwnership.length,
  missing_ownership_count: missingOwnership.length,
  duplicate_family_count: duplicateFamilies.length,
  severity: missingOwnership.length > 0 ? "yellow" : duplicateFamilies.length > 0 ? "white" : "pass",
  findings: [
    ...missingOwnership.slice(0, 50).map((row) => ({
      kind: "guard_missing_ownership_marker",
      path: row.path,
      owner_guess: "validation",
      next_action: "Add a source_domain, policy path, owner_domain, or Layer ownership marker.",
    })),
    ...duplicateFamilies.slice(0, 25).map((row) => ({
      kind: "duplicate_guard_family",
      family: row.family,
      count: row.count,
      paths: row.paths,
      next_action: "Decide whether these guards are distinct tiers or stale duplicates.",
    })),
  ],
  rows,
};
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/guard_registry_ownership_current.json"));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
