import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

type Json = Record<string, unknown>;

type Candidate = {
  name: string;
  path: string;
  exists: boolean;
  size_bytes: number | null;
  mtime_ms: number | null;
  sha256: string | null;
  stale_vs_source: boolean | null;
};

const root = process.cwd();
const policyPath = path.join(root, "validation/regression/fixtures/gateway_idempotence/gateway_status_launcher_drift_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch {
    return "";
  }
}

function fileStat(filePath: string): fs.Stats | null {
  try {
    return fs.statSync(filePath);
  } catch {
    return null;
  }
}

function sha256(filePath: string): string | null {
  try {
    return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
  } catch {
    return null;
  }
}

function walkNewestMtime(dirPath: string): number {
  let newest = 0;
  let entries: fs.Dirent[] = [];
  try {
    entries = fs.readdirSync(dirPath, { withFileTypes: true });
  } catch {
    return newest;
  }
  for (const entry of entries) {
    const child = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      newest = Math.max(newest, walkNewestMtime(child));
    } else if (entry.isFile() && (entry.name.endsWith(".rs") || entry.name === "Cargo.toml")) {
      const stat = fileStat(child);
      newest = Math.max(newest, stat?.mtimeMs || 0);
    }
  }
  return newest;
}

function newestSourceMtime(): number {
  const roots = Array.isArray(policy.ops_source_roots) ? policy.ops_source_roots : [];
  let newest = 0;
  for (const sourceRoot of roots) {
    newest = Math.max(newest, walkNewestMtime(path.join(root, String(sourceRoot))));
  }
  const manifest = fileStat(path.join(root, String(policy.ops_manifest_path || "")));
  return Math.max(newest, manifest?.mtimeMs || 0);
}

function candidate(name: string, relOrAbs: string, sourceNewestMtimeMs: number): Candidate {
  const candidatePath = path.isAbsolute(relOrAbs) ? relOrAbs : path.join(root, relOrAbs);
  const stat = fileStat(candidatePath);
  const exists = Boolean(stat?.isFile());
  const mtimeMs = exists ? stat?.mtimeMs || null : null;
  return {
    name,
    path: candidatePath,
    exists,
    size_bytes: exists ? stat?.size || 0 : null,
    mtime_ms: mtimeMs,
    sha256: exists ? sha256(candidatePath) : null,
    stale_vs_source: exists && mtimeMs !== null ? mtimeMs + 1000 < sourceNewestMtimeMs : null,
  };
}

const sourceNewestMtimeMs = newestSourceMtime();
const envCandidate = process.env.INFRING_NPM_BINARY
  ? candidate("INFRING_NPM_BINARY", process.env.INFRING_NPM_BINARY, sourceNewestMtimeMs)
  : null;
const candidates = [
  ...(envCandidate ? [envCandidate] : []),
  candidate("vendor", "client/cli/npm/vendor/infring-ops", sourceNewestMtimeMs),
  candidate("debug", "target/debug/infring-ops", sourceNewestMtimeMs),
  candidate("release", "target/release/infring-ops", sourceNewestMtimeMs),
];
const selectedCandidate = candidates.find((row) => row.exists) || null;
const launcherPath = path.join(root, String(policy.launcher_path || ""));
const launcherSource = readText(launcherPath);
const packageJson = JSON.parse(readText(path.join(root, "package.json")) || "{}") as { scripts?: Record<string, string> };
const expectedScripts = policy.package_scripts as Record<string, string> | undefined;
const scriptMismatches = Object.entries(expectedScripts || {}).filter(([key, value]) => {
  const actualKey = key === "gateway_status" ? "gateway:status" : key;
  return packageJson.scripts?.[actualKey] !== value;
});
const homeInstalled = [
  candidate("home_local", path.join(os.homedir(), ".local/bin/infring-ops"), sourceNewestMtimeMs),
  candidate("home_infring", path.join(os.homedir(), ".infring/bin/infring-ops"), sourceNewestMtimeMs),
];
const launcherMissing = !launcherSource;
const launcherUsesHomeBinary = launcherSource.includes("$HOME/.local") || launcherSource.includes(".infring/bin/infring-ops");
const launcherHasCargoFallback = launcherSource.includes("cargo run --quiet --manifest-path");
const selectedStale = Boolean(selectedCandidate?.stale_vs_source);
const diagnostic = launcherMissing || scriptMismatches.length > 0 || launcherUsesHomeBinary || !launcherHasCargoFallback
  ? "gateway_status_launcher_policy_mismatch"
  : selectedStale
    ? "gateway_status_launcher_selected_binary_stale"
    : selectedCandidate
      ? "gateway_status_launcher_current"
      : "gateway_status_launcher_uses_cargo_fallback";
const nextActions = diagnostic === "gateway_status_launcher_selected_binary_stale"
  ? [
      "Rebuild the source-authoritative ops binary: cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops",
      "Rerun gateway status through npm so the launcher resolves the fresh target/debug/infring-ops binary.",
    ]
  : diagnostic === "gateway_status_launcher_policy_mismatch"
    ? [
        "Restore client/cli/bin/infring to source-authoritative resolution: vendor, target/debug, target/release, then cargo fallback.",
        "Keep npm gateway status read-only and routed through the repo launcher instead of a home-installed binary.",
      ]
    : diagnostic === "gateway_status_launcher_uses_cargo_fallback"
      ? [
          "No repo binary is present; gateway status will use cargo fallback if Cargo is installed.",
          "For faster repeated status checks, build target/debug/infring-ops from source.",
        ]
      : [];
const traceId = `validation:${new Date().toISOString()}:gateway-status-launcher-drift`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "validation",
  type: "gateway_status_launcher_drift_report",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  launcher_path: String(policy.launcher_path || ""),
  source_newest_mtime_ms: sourceNewestMtimeMs,
  selected_candidate: selectedCandidate ? selectedCandidate.name : "cargo_fallback",
  diagnostic,
  severity: diagnostic === "gateway_status_launcher_current" ? "pass" : "yellow",
  root_cause_hypothesis: selectedStale
    ? "The repo launcher is resolving a binary older than the current ops source tree, so status can reflect stale dispatch/security behavior even when source-level cargo run is fixed."
    : launcherMissing || scriptMismatches.length > 0 || launcherUsesHomeBinary || !launcherHasCargoFallback
      ? "Gateway status launcher policy does not match the source-authoritative contract."
      : selectedCandidate
        ? "Gateway status resolves a source-local binary that is current relative to the ops source tree."
        : "No source-local binary exists, so status depends on cargo fallback at runtime.",
  next_actions: nextActions,
  candidates,
  home_installed_candidates: homeInstalled,
  script_mismatches: scriptMismatches.map(([key, expected]) => ({
    key,
    expected,
    actual: packageJson.scripts?.[key === "gateway_status" ? "gateway:status" : key] || null,
  })),
  launcher_has_cargo_fallback: launcherHasCargoFallback,
  launcher_uses_home_binary: launcherUsesHomeBinary,
};

const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/gateway_status_launcher_drift_current.json"));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
