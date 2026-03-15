#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { execSync } from 'node:child_process';

const OUT_JSON = 'core/local/artifacts/mcu_proof_preflight_current.json';
const OUT_MD = 'local/workspace/reports/MCU_PROOF_PREFLIGHT.md';

function run(cmd) {
  try {
    return execSync(cmd, { stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' }).trim();
  } catch {
    return '';
  }
}

function which(cmd) {
  const found = run(`command -v ${cmd}`);
  return found.length > 0 ? found : null;
}

function detectSerialPorts() {
  const patterns = ['/dev/tty.usb*', '/dev/cu.usb*', '/dev/ttyACM*', '/dev/ttyUSB*'];
  const ports = [];
  for (const pattern of patterns) {
    const out = run(`ls ${pattern} 2>/dev/null`);
    if (!out) continue;
    for (const line of out.split('\n')) {
      const trimmed = line.trim();
      if (trimmed.length > 0) ports.push(trimmed);
    }
  }
  return [...new Set(ports)];
}

function hasPythonEsptool() {
  const out = run('python3 -m esptool version 2>/dev/null | head -n 1');
  return out.toLowerCase().includes('esptool v');
}

function fileSizeBytes(path) {
  const out = run(`stat -f%z ${path}`);
  if (!out) return null;
  const n = Number(out);
  return Number.isFinite(n) ? n : null;
}

function main() {
  const tools = {
    openocd: which('openocd'),
    picotool: which('picotool'),
    esptool:
      which('esptool') ??
      which('esptool.py') ??
      (hasPythonEsptool() ? 'python3 -m esptool' : null),
    screen: which('screen'),
  };

  const tinyDaemonPath = resolve(
    'target/x86_64-unknown-linux-musl/release/protheusd_tiny_max',
  );
  const tinyDaemonBytes = existsSync(tinyDaemonPath) ? fileSizeBytes(tinyDaemonPath) : null;
  const tinyDaemonMb = tinyDaemonBytes == null ? null : Number((tinyDaemonBytes / 1_000_000).toFixed(3));
  const serialPorts = detectSerialPorts();

  const blockers = [];
  if (!tools.openocd) blockers.push('missing_tool_openocd');
  if (!tools.picotool) blockers.push('missing_tool_picotool');
  if (!tools.esptool) blockers.push('missing_tool_esptool');
  if (serialPorts.length === 0) blockers.push('no_usb_serial_device_detected');
  if (!existsSync(tinyDaemonPath)) blockers.push('missing_tiny_max_daemon_artifact');

  const payload = {
    ok: blockers.length === 0,
    type: 'mcu_proof_preflight',
    generatedAt: new Date().toISOString(),
    tinyMax: {
      daemonPath: tinyDaemonPath,
      daemonBytes: tinyDaemonBytes,
      daemonMb: tinyDaemonMb,
      sub300kb: tinyDaemonBytes != null ? tinyDaemonBytes <= 300_000 : false,
    },
    tools,
    serialPorts,
    blockers,
    unblockRef: 'HMAN-092',
    runbook: 'docs/ops/RUNBOOK-005-mcu-proof-sprint.md',
  };

  mkdirSync(dirname(resolve(OUT_JSON)), { recursive: true });
  mkdirSync(dirname(resolve(OUT_MD)), { recursive: true });
  writeFileSync(resolve(OUT_JSON), `${JSON.stringify(payload, null, 2)}\n`);

  const md = [];
  md.push('# MCU Proof Preflight');
  md.push('');
  md.push(`Generated: ${payload.generatedAt}`);
  md.push('');
  md.push(`- status: **${payload.ok ? 'ok' : 'blocked_external'}**`);
  md.push(`- unblock_ref: **${payload.unblockRef}**`);
  md.push(`- runbook: \`${payload.runbook}\``);
  md.push('');
  md.push('## Tiny-Max Artifact');
  md.push('');
  md.push(`- daemon_path: \`${payload.tinyMax.daemonPath}\``);
  md.push(`- daemon_bytes: \`${payload.tinyMax.daemonBytes ?? 'missing'}\``);
  md.push(`- daemon_mb: \`${payload.tinyMax.daemonMb ?? 'missing'}\``);
  md.push(`- sub_300kb: \`${payload.tinyMax.sub300kb}\``);
  md.push('');
  md.push('## Tooling');
  md.push('');
  md.push(`- openocd: \`${payload.tools.openocd ?? 'missing'}\``);
  md.push(`- picotool: \`${payload.tools.picotool ?? 'missing'}\``);
  md.push(`- esptool: \`${payload.tools.esptool ?? 'missing'}\``);
  md.push(`- screen: \`${payload.tools.screen ?? 'missing'}\``);
  md.push('');
  md.push('## Serial Ports');
  md.push('');
  if (payload.serialPorts.length === 0) {
    md.push('- none detected');
  } else {
    for (const port of payload.serialPorts) md.push(`- \`${port}\``);
  }
  md.push('');
  md.push('## Blockers');
  md.push('');
  if (payload.blockers.length === 0) {
    md.push('- none');
  } else {
    for (const blocker of payload.blockers) md.push(`- \`${blocker}\``);
  }
  md.push('');
  writeFileSync(resolve(OUT_MD), `${md.join('\n')}\n`);

  console.log(JSON.stringify(payload, null, 2));
}

main();
