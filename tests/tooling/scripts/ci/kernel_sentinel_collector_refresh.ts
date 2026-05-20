#!/usr/bin/env node
/* eslint-disable no-console */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

function flag(name: string, fallback = ""): string {
  const exact = `--${name}`;
  const prefix = `${exact}=`;
  for (let idx = 2; idx < process.argv.length; idx += 1) {
    const arg = process.argv[idx] || "";
    if (arg === exact) return process.argv[idx + 1] || "";
    if (arg.startsWith(prefix)) return arg.slice(prefix.length);
  }
  return fallback;
}

function writeJson(rel: string, payload: unknown): void {
  const abs = path.join(root, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

const timeoutMs = Math.max(1_000, Number(flag("timeout-ms", "120000")));
const outJson = flag("out-json", "core/local/artifacts/kernel_sentinel_collector_refresh_current.json");
const collectorArtifact = flag("collector-artifact", "core/local/artifacts/kernel_sentinel_collector_current.json");
const generatedAt = new Date().toISOString();
const traceId = `observability:${generatedAt}:kernel-sentinel-collector-refresh`;
function resolveOpsBinary(): string | null {
  const candidates = [
    process.env.INFRING_SENTINEL_COLLECTOR_OPS_BINARY || "",
    "target/debug/infring-ops",
    "target/release/infring-ops",
  ].filter(Boolean);
  for (const candidate of candidates) {
    const abs = path.isAbsolute(candidate) ? candidate : path.join(root, candidate);
    try {
      if (fs.statSync(abs).isFile()) return abs;
    } catch {
      // try next candidate
    }
  }
  return null;
}

const opsBinary = resolveOpsBinary();
const command = opsBinary || "cargo";
const args = opsBinary ? [
  "kernel-sentinel",
  "collect",
  `--collector-artifact=${collectorArtifact}`,
]
  : [
      "run",
      "--quiet",
      "--manifest-path",
      "core/layer0/ops/Cargo.toml",
      "--bin",
      "infring-ops",
      "--",
      "kernel-sentinel",
      "collect",
      `--collector-artifact=${collectorArtifact}`,
    ];

const child = spawn(command, args, { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
let stdout = "";
let stderr = "";
let timedOut = false;
const timer = setTimeout(() => {
  timedOut = true;
  try {
    child.kill("SIGTERM");
  } catch {
    // best effort
  }
}, timeoutMs);

child.stdout.on("data", (chunk) => {
  stdout += String(chunk);
  if (stdout.length > 12_000) stdout = stdout.slice(-12_000);
});
child.stderr.on("data", (chunk) => {
  stderr += String(chunk);
  if (stderr.length > 12_000) stderr = stderr.slice(-12_000);
});

child.on("close", (code, signal) => {
  clearTimeout(timer);
  const ok = !timedOut && code === 0;
  const payload = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: "observability",
    type: "kernel_sentinel_collector_refresh",
    generated_at: generatedAt,
    ok,
    timeout_ms: timeoutMs,
    timed_out: timedOut,
    exit_code: code,
    signal,
    command,
    used_prebuilt_ops_binary: Boolean(opsBinary),
    collector_artifact: collectorArtifact,
    stdout_tail: stdout.trim().slice(-4000),
    stderr_tail: stderr.trim().slice(-4000),
    next_action: ok
      ? "Use refreshed collector artifact for Sentinel evidence freshness decisions."
      : "Treat previous collector warnings as stale until this bounded refresh succeeds.",
  };
  writeJson(outJson, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (!ok) process.exitCode = 1;
});
