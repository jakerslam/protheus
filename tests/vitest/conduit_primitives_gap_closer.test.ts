import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { describe, expect, test } from 'vitest';

const ROOT = process.cwd();

const wrapperFiles = [
  'client/runtime/systems/autonomy/self_improvement_cadence_orchestrator.ts',
  'client/runtime/systems/memory/causal_temporal_graph.ts',
  'client/runtime/systems/execution/task_decomposition_primitive.ts',
  'client/runtime/systems/workflow/universal_outreach_primitive.ts',
] as const;

function collectFilesUnder(relativeRoot: string, suffix: string): string[] {
  const out: string[] = [];
  const root = path.join(ROOT, relativeRoot);
  const walk = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const abs = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(abs);
        continue;
      }
      if (!entry.isFile()) continue;
      if (!abs.endsWith(suffix)) continue;
      out.push(path.relative(ROOT, abs).replace(/\\/g, '/'));
    }
  };
  if (fs.existsSync(root)) {
    walk(root);
  }
  return out.sort();
}

describe('conduit primitive wrapper contract', () => {
  test.each(wrapperFiles)('wrapper contract enforced for %s', async (relativePath) => {
    const full = path.join(ROOT, relativePath);
    const source = fs.readFileSync(full, 'utf8');
    // Wrapper contract allows either ts_bootstrap entrypoints or direct Rust lane bridges.
    const hasBootstrapEntrypoint =
      source.includes('ts_bootstrap.ts') && source.includes('bootstrap(__filename, module)');
    const hasRustLaneBridge = source.includes('createOpsLaneBridge');
    expect(hasBootstrapEntrypoint || hasRustLaneBridge).toBe(true);
    expect(source.includes('legacy_retired_lane_bridge')).toBe(false);
  });

  test('install.sh exists and references hosted installer endpoint', () => {
    const source = fs.readFileSync(path.join(ROOT, 'install.sh'), 'utf8');
    expect(source.includes('api.github.com/repos')).toBe(true);
    expect(source.includes('protheus-ops')).toBe(true);
    expect(source.includes('infringd')).toBe(true);
    expect(source.includes('--repair')).toBe(true);
    expect(source.includes("'protheusd' is deprecated")).toBe(true);
    expect(source.includes('persist_path_for_shell')).toBe(true);
    expect(source.includes('PATH persisted in')).toBe(true);
    expect(source.includes('activate now: .')).toBe(true);
  });

  test('install.sh gateway fallback is Rust-first (Node optional legacy only)', () => {
    const source = fs.readFileSync(path.join(ROOT, 'install.sh'), 'utf8');
    expect(source).toMatch(
      /launch_cmd="cd \$root && exec \$dashboard_bin gateway start --dashboard-host=\$host --dashboard-port=\$port --dashboard-open=0"/,
    );
    expect(
      source.includes(
        'infring_gateway_spawn_detached_logged /tmp/infring-dashboard-serve.log "$dashboard_bin"',
      ),
    ).toBe(true);
    expect(
      source.includes(
        'gateway start "--dashboard-host=${host}" "--dashboard-port=${port}" "--dashboard-open=0"',
      ),
    ).toBe(true);
    expect(source).not.toMatch(
      /infring_gateway_spawn_detached_logged \/tmp\/infring-dashboard-serve\.log node/,
    );
  });

  test('install.sh enforces runtime entrypoint integrity contract', () => {
    const source = fs.readFileSync(path.join(ROOT, 'install.sh'), 'utf8');
    expect(source.includes('verify_workspace_runtime_contract')).toBe(true);
    expect(source.includes('RUNTIME_MANIFEST_REL')).toBe(true);
    expect(source.includes('run_post_install_smoke_tests')).toBe(true);
    expect(source.includes('dashboard_route_check')).toBe(true);
  });

  test('install.ps1 exists and provisions Windows wrappers', () => {
    const source = fs.readFileSync(path.join(ROOT, 'install.ps1'), 'utf8');
    expect(source.includes('protheus-ops.exe')).toBe(true);
    expect(source.includes('infringd.cmd')).toBe(true);
    expect(source.includes('protheusd.cmd')).toBe(true);
    expect(source.includes('$Repair')).toBe(true);
    expect(source.includes('conduit_daemon')).toBe(true);
  });

  test('architecture doc includes conduit mermaid map', () => {
    const source = fs.readFileSync(path.join(ROOT, 'ARCHITECTURE.md'), 'utf8');
    expect(source.includes('```mermaid')).toBe(true);
    expect(source.includes('Conduit')).toBe(true);
    expect(source.includes('Core')).toBe(true);
  });

  test('getting started doc includes curl and powershell install paths', () => {
    const source = fs.readFileSync(path.join(ROOT, 'docs/client/GETTING_STARTED.md'), 'utf8');
    expect(source.includes('curl -fsSL https://get.protheus.ai/install | sh')).toBe(true);
    expect(source.includes('install.ps1')).toBe(true);
    expect(source.includes('infring --help')).toBe(true);
  });

  test('unknown guard json mode emits a single machine-readable payload', () => {
    const entrypoint = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
    const guard = path.join(ROOT, 'client/runtime/systems/ops/protheus_unknown_guard.ts');
    const proc = spawnSync(process.execPath, [entrypoint, guard, '--json', 'bogus-cmd'], {
      cwd: ROOT,
      encoding: 'utf8',
    });

    expect(proc.status).toBe(2);
    const lines = String(proc.stdout || '')
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
    expect(lines.length).toBe(1);
    const payload = JSON.parse(lines[0]);
    expect(payload.ok).toBe(false);
    expect(payload.type).toBe('protheus_unknown_guard');
    expect(payload.command).toBe('bogus-cmd');
    expect(String(proc.stderr || '').trim()).toBe('');
  });

  test('runtime manifest lists resolvable runtime entrypoints', () => {
    const rel = 'client/runtime/config/install_runtime_manifest_v1.txt';
    const manifestPath = path.join(ROOT, rel);
    expect(fs.existsSync(manifestPath)).toBe(true);
    const rows = fs
      .readFileSync(manifestPath, 'utf8')
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith('#'));
    expect(rows.length).toBeGreaterThan(0);
    for (const entry of rows) {
      const abs = path.join(ROOT, entry);
      const alt =
        entry.endsWith('.js')
          ? path.join(ROOT, entry.slice(0, -3) + '.ts')
          : entry.endsWith('.ts')
            ? path.join(ROOT, entry.slice(0, -3) + '.js')
            : '';
      expect(fs.existsSync(abs) || (!!alt && fs.existsSync(alt))).toBe(true);
    }
  });

  test('runtime bootstrap dependencies are shipped as runtime deps', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
    const deps = (pkg && pkg.dependencies) || {};
    const devDeps = (pkg && pkg.devDependencies) || {};
    expect(typeof deps.typescript).toBe('string');
    expect(typeof deps.ws).toBe('string');
    expect(devDeps.typescript).toBeUndefined();
    expect(devDeps.ws).toBeUndefined();
  });

  test('installer smoke checks canonical dashboard route', () => {
    const source = fs.readFileSync(path.join(ROOT, 'install.sh'), 'utf8');
    expect(source.includes('dashboard status --json')).toBe(true);
  });

  test('protheusctl route map targets are resolvable or explicitly optional-not-shipped', () => {
    const routeSources = [
      'core/layer0/ops/src/protheusctl.rs',
      'core/layer0/ops/src/protheusctl_routes.rs',
      'core/layer0/ops/src/protheusctl_plane_shortcuts.rs',
      ...collectFilesUnder('core/layer0/ops/src/protheusctl_parts', '.rs'),
      ...collectFilesUnder('core/layer0/ops/src/protheusctl_routes_parts', '.rs'),
    ];
    const optionalNotShipped = new Set<string>([
      'client/cognition/adaptive/rsi/rsi_bootstrap.js',
      'client/runtime/systems/economy/donor_mining_dashboard.js',
      'client/runtime/systems/edge/mobile_lifecycle_resilience.ts',
      'client/runtime/systems/edge/mobile_ops_top.ts',
      'client/runtime/systems/edge/protheus_edge_runtime.ts',
      'client/runtime/systems/migration/core_migration_bridge.js',
      'client/runtime/systems/migration/universal_importers.js',
      'client/runtime/systems/ops/fluxlattice_program.js',
      'client/runtime/systems/ops/host_adaptation_operator_surface.js',
      'client/runtime/systems/ops/mobile_wrapper_distribution_pack.js',
      'client/runtime/systems/ops/perception_polish_program.js',
      'client/runtime/systems/ops/platform_socket_runtime.ts',
      'client/runtime/systems/ops/productized_suite_program.js',
      'client/runtime/systems/ops/protheus_demo.js',
      'client/runtime/systems/ops/protheus_diagram.js',
      'client/runtime/systems/ops/protheus_examples.js',
      'client/runtime/systems/ops/protheus_version_cli.js',
      'client/runtime/systems/ops/protheusctl_skills_discover.js',
      'client/runtime/systems/ops/rust_hybrid_migration_program.js',
      'client/runtime/systems/ops/scale_readiness_program.js',
      'client/runtime/systems/ops/settlement_program.js',
      'client/runtime/systems/ops/wasi2_execution_completeness_gate.js',
      'client/runtime/systems/personas/ambient_stance.js',
      'client/runtime/systems/personas/cli.js',
      'client/runtime/systems/spawn/mobile_edge_swarm_bridge.ts',
      'client/runtime/systems/spine/spine_safe_launcher.js',
      'client/runtime/systems/ops/rust_authoritative_microkernel_acceleration.js',
    ]);
    const routeTargets = new Set<string>();
    for (const rel of routeSources) {
      const source = fs.readFileSync(path.join(ROOT, rel), 'utf8');
      const pattern = /script_rel:\s*"([^"]+)"/g;
      let match: RegExpExecArray | null = null;
      while ((match = pattern.exec(source))) {
        routeTargets.add(match[1]);
      }
    }

    const unresolved: string[] = [];
    for (const target of routeTargets) {
      if (target.startsWith('core://')) continue;
      const abs = path.join(ROOT, target);
      const alt =
        target.endsWith('.js')
          ? path.join(ROOT, target.slice(0, -3) + '.ts')
          : target.endsWith('.ts')
            ? path.join(ROOT, target.slice(0, -3) + '.js')
            : '';
      const exists = fs.existsSync(abs) || (!!alt && fs.existsSync(alt));
      if (exists) continue;
      if (optionalNotShipped.has(target)) continue;
      unresolved.push(target);
    }

    const staleOptional = [...optionalNotShipped].filter((target) => {
      const abs = path.join(ROOT, target);
      const alt =
        target.endsWith('.js')
          ? path.join(ROOT, target.slice(0, -3) + '.ts')
          : target.endsWith('.ts')
            ? path.join(ROOT, target.slice(0, -3) + '.js')
            : '';
      return fs.existsSync(abs) || (!!alt && fs.existsSync(alt));
    });

    expect(unresolved.sort()).toEqual([]);
    expect(staleOptional.sort()).toEqual([]);
  });

  test('runtime manifest entrypoints are wired from CLI route surfaces', () => {
    const routeSources = [
      'core/layer0/ops/src/protheusctl.rs',
      'core/layer0/ops/src/protheusctl_routes.rs',
      'core/layer0/ops/src/protheusctl_plane_shortcuts.rs',
      ...collectFilesUnder('core/layer0/ops/src/protheusctl_parts', '.rs'),
      ...collectFilesUnder('core/layer0/ops/src/protheusctl_routes_parts', '.rs'),
    ];
    const routeTargets = new Set<string>();
    for (const rel of routeSources) {
      const source = fs.readFileSync(path.join(ROOT, rel), 'utf8');
      const pattern = /script_rel:\s*"([^"]+)"/g;
      let match: RegExpExecArray | null = null;
      while ((match = pattern.exec(source))) {
        routeTargets.add(match[1]);
      }
    }

    const manifestPath = path.join(ROOT, 'client/runtime/config/install_runtime_manifest_v1.txt');
    const manifestEntries = fs
      .readFileSync(manifestPath, 'utf8')
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith('#'));
    expect(manifestEntries.length).toBeGreaterThan(0);
    const bootstrapOnlyEntrypoints = new Set<string>([
      'client/runtime/systems/ops/protheusd.ts',
      'client/runtime/systems/ops/protheus_unknown_guard.ts',
    ]);

    for (const entry of manifestEntries) {
      if (bootstrapOnlyEntrypoints.has(entry)) {
        continue;
      }
      const counterpart = entry.endsWith('.js')
        ? entry.slice(0, -3) + '.ts'
        : entry.endsWith('.ts')
          ? entry.slice(0, -3) + '.js'
          : '';
      expect(routeTargets.has(entry) || (!!counterpart && routeTargets.has(counterpart))).toBe(true);
    }
  });

  test('unknown command fallback is core-native and not JS-asset dependent', () => {
    const source = fs.readFileSync(
      path.join(ROOT, 'core/layer0/ops/src/protheusctl_parts/030-usage.rs'),
      'utf8',
    );
    expect(source.includes('"core://unknown-command"')).toBe(true);
  });
});

describe('conduit client coverage paths', () => {
  test('message budget constants match expected contract count', async () => {
    const conduit = await import(pathToFileURL(path.join(ROOT, 'client/runtime/systems/conduit/conduit-client.ts')).href);
    expect(conduit.MAX_CONDUIT_MESSAGE_TYPES).toBe(10);
    expect(conduit.TS_COMMAND_TYPES.length + conduit.RUST_EVENT_TYPES.length).toBe(10);
  });

  test('overStdio sends signed envelope and parses response', async () => {
    const conduit = await import(pathToFileURL(path.join(ROOT, 'client/runtime/systems/conduit/conduit-client.ts')).href);
    const script = `
process.stdin.setEncoding('utf8');
let buffer = '';
process.stdin.on('data', (chunk) => {
  buffer += chunk;
  if (!buffer.includes('\\n')) return;
  const line = buffer.split('\\n')[0];
  const req = JSON.parse(line);
  const response = {
    schema_id: req.schema_id,
    schema_version: req.schema_version,
    request_id: req.request_id,
    ts_ms: req.ts_ms,
    event: {
      type: 'system_feedback',
      status: 'ok',
      detail: {
        command_type: req.command.type,
        signature_len: String(req.security.signature || '').length,
        token_len: String(req.security.capability_token.signature || '').length
      },
      violation_reason: null
    },
    validation: {
      ok: true,
      fail_closed: false,
      reason: 'validated',
      policy_receipt_hash: 'p',
      security_receipt_hash: 's',
      receipt_hash: 'v'
    },
    crossing: {
      crossing_id: req.request_id,
      direction: 'TsToRust',
      command_type: req.command.type,
      deterministic_hash: 'd',
      ts_ms: req.ts_ms
    },
    receipt_hash: 'r'
  };
  process.stdout.write(JSON.stringify(response) + '\\n');
});
`;
    const client = conduit.ConduitClient.overStdio(
      process.execPath,
      ['-e', script],
      ROOT,
      { token_ttl_ms: 60_000 },
    );

    const response = await client.send({ type: 'get_system_status' }, 'req-stdio-1');
    await client.close();

    expect(response.request_id).toBe('req-stdio-1');
    expect((response.event as any).status).toBe('ok');
    expect((response.event as any).detail.command_type).toBe('get_system_status');
    expect((response.event as any).detail.signature_len).toBeGreaterThan(16);
    expect((response.event as any).detail.token_len).toBeGreaterThan(16);
  }, 60_000);

  test('overStdio surfaces stderr as conduit error', async () => {
    const conduit = await import(pathToFileURL(path.join(ROOT, 'client/runtime/systems/conduit/conduit-client.ts')).href);
    const client = conduit.ConduitClient.overStdio(
      process.execPath,
      ['-e', 'process.stderr.write(\"boom\\n\"); setTimeout(() => process.exit(1), 10);'],
      ROOT,
    );

    await expect(client.send({ type: 'list_active_agents' }, 'req-stdio-err')).rejects.toThrow(
      /conduit_stdio_error|conduit_stdio_exit/,
    );
    await client.close();
  });

  test('overUnixSocket path works for single roundtrip', async () => {
    if (process.platform === 'win32') return;
    const previousFallback = process.env.PROTHEUS_CONDUIT_TS_FALLBACK;
    process.env.PROTHEUS_CONDUIT_TS_FALLBACK = '1';
    const sockets = new Set<net.Socket>();
    let socketPath = '';
    let server: net.Server | null = null;
    let client: { send: (...args: any[]) => Promise<any>; close: () => Promise<void> } | null = null;
    try {
      const conduit = await import(pathToFileURL(path.join(ROOT, 'client/runtime/systems/conduit/conduit-client.ts')).href);
      socketPath = path.join('/tmp', `pc-${process.pid}-${randomUUID()}.sock`);
      if (fs.existsSync(socketPath)) {
        fs.unlinkSync(socketPath);
      }

      server = net.createServer((socket) => {
        sockets.add(socket);
        socket.once('close', () => sockets.delete(socket));
        let buffer = '';
        socket.setEncoding('utf8');
        socket.on('data', (chunk) => {
          buffer += chunk;
          if (!buffer.includes('\n')) return;
          const line = buffer.split('\n')[0];
          let req: any = null;
          try {
            req = JSON.parse(line);
          } catch (error) {
            socket.end();
            return;
          }
          const response = {
            schema_id: req.schema_id,
            schema_version: req.schema_version,
            request_id: req.request_id,
            ts_ms: req.ts_ms,
            event: {
              type: 'system_feedback',
              status: 'ok',
              detail: { command_type: req.command.type },
              violation_reason: null
            },
            validation: {
              ok: true,
              fail_closed: false,
              reason: 'validated',
              policy_receipt_hash: 'p',
              security_receipt_hash: 's',
              receipt_hash: 'v'
            },
            crossing: {
              crossing_id: req.request_id,
              direction: 'TsToRust',
              command_type: req.command.type,
              deterministic_hash: 'd',
              ts_ms: req.ts_ms
            },
            receipt_hash: 'r'
          };
          socket.end(JSON.stringify(response) + '\n');
        });
      });

      await new Promise<void>((resolve, reject) => {
        server!.listen(socketPath, () => resolve());
        server!.once('error', reject);
      });

      client = conduit.ConduitClient.overUnixSocket(socketPath, {
        client_id: 'vitest-unix-socket',
        signing_key_id: 'vitest-signing-key',
        signing_secret: 'vitest-signing-secret',
        token_key_id: 'vitest-token-key',
        token_secret: 'vitest-token-secret',
        token_ttl_ms: 60_000,
      });
      const response = await client.send({ type: 'get_system_status' }, 'req-socket-1');

      expect(response.request_id).toBe('req-socket-1');
      expect((response.event as any).detail.command_type).toBe('get_system_status');
    } finally {
      if (client) {
        await client.close();
      }
      for (const socket of sockets) {
        socket.destroy();
      }
      if (server) {
        await Promise.race([
          new Promise<void>((resolve) => server!.close(() => resolve())),
          new Promise<void>((resolve) => setTimeout(resolve, 2_000)),
        ]);
      }
      if (socketPath && fs.existsSync(socketPath)) {
        fs.unlinkSync(socketPath);
      }
      if (typeof previousFallback === 'string') {
        process.env.PROTHEUS_CONDUIT_TS_FALLBACK = previousFallback;
      } else {
        delete process.env.PROTHEUS_CONDUIT_TS_FALLBACK;
      }
    }
  }, 90_000);
});

describe('direct conduit lane bridge coverage paths', () => {
  test('findRepoRoot resolves workspace root from nested directory', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const found = bridge.findRepoRoot(path.join(ROOT, 'client', 'runtime', 'systems', 'ops'));
    expect(found).toBe(ROOT);
  }, 90_000);

  test('createConduitLaneModule normalizes lane id and exposes async builders', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const lane = bridge.createConduitLaneModule('systems-primitives-policy-vm', ROOT);
    expect(lane.LANE_ID).toBe('SYSTEMS-PRIMITIVES-POLICY-VM');
    expect(typeof lane.buildLaneReceipt).toBe('function');
    expect(typeof lane.verifyLaneReceipt).toBe('function');
  });

  test('runLaneViaConduit fails closed when daemon exits before responding', async () => {
    const previousCommand = process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND;
    const previousArgs = process.env.PROTHEUS_CONDUIT_DAEMON_ARGS;
    process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND = process.execPath;
    process.env.PROTHEUS_CONDUIT_DAEMON_ARGS = '-e process.exit(0)';
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const receipt = await bridge.runLaneViaConduit('SYSTEMS-PRIMITIVES-POLICY-VM', ROOT);
    if (previousCommand == null) {
      delete process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND;
    } else {
      process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND = previousCommand;
    }
    if (previousArgs == null) {
      delete process.env.PROTHEUS_CONDUIT_DAEMON_ARGS;
    } else {
      process.env.PROTHEUS_CONDUIT_DAEMON_ARGS = previousArgs;
    }
    expect(receipt.ok).toBe(false);
    expect(String(receipt.error || '')).not.toHaveLength(0);
    expect(receipt.type).toBe('conduit_lane_bridge_error');
  }, 10_000);

  test('findRepoRoot falls back to process cwd when no markers exist', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-no-root-'));
    const nested = path.join(temp, 'a', 'b', 'c');
    fs.mkdirSync(nested, { recursive: true });
    expect(bridge.findRepoRoot(nested)).toBe(process.cwd());
  });

  test('runLaneViaConduit returns lane receipt when conduit provides one', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-bridge-ok-'));
    fs.mkdirSync(path.join(temp, 'core', 'layer0', 'ops'), { recursive: true });
    fs.mkdirSync(path.join(temp, 'client', 'runtime', 'systems', 'conduit'), { recursive: true });
    fs.writeFileSync(path.join(temp, 'Cargo.toml'), '[package]\nname="tmp"\nversion="0.0.0"\n');
    fs.writeFileSync(path.join(temp, 'core', 'layer0', 'ops', 'Cargo.toml'), '[package]\nname="ops"\nversion="0.0.0"\n');
    fs.writeFileSync(
      path.join(temp, 'client', 'runtime', 'systems', 'conduit', 'conduit-client.js'),
      `module.exports = {
  ConduitClient: {
    overStdio() {
      return {
        send: async () => ({
          event: {
            type: 'system_feedback',
            detail: { lane_receipt: { ok: true, lane_id: 'SYSTEM-LANE', receipt_hash: 'r' } }
          }
        }),
        close: async () => {}
      };
    }
  }
};\n`,
      'utf8',
    );

    const receipt = await bridge.runLaneViaConduit('system-lane', temp);
    expect(receipt.ok).toBe(true);
    expect(receipt.lane_id).toBe('SYSTEM-LANE');
  });

  test('runLaneViaConduit emits lane_receipt_missing when event detail lacks receipt', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-bridge-missing-'));
    fs.mkdirSync(path.join(temp, 'core', 'layer0', 'ops'), { recursive: true });
    fs.mkdirSync(path.join(temp, 'client', 'runtime', 'systems', 'conduit'), { recursive: true });
    fs.writeFileSync(path.join(temp, 'Cargo.toml'), '[package]\nname="tmp"\nversion="0.0.0"\n');
    fs.writeFileSync(path.join(temp, 'core', 'layer0', 'ops', 'Cargo.toml'), '[package]\nname="ops"\nversion="0.0.0"\n');
    fs.writeFileSync(
      path.join(temp, 'client', 'runtime', 'systems', 'conduit', 'conduit-client.js'),
      `module.exports = {
  ConduitClient: {
    overStdio() {
      return {
        send: async () => ({ event: { type: 'system_feedback', detail: { } } }),
        close: async () => {}
      };
    }
  }
};\n`,
      'utf8',
    );

    const receipt = await bridge.runLaneViaConduit('system-lane', temp);
    expect(receipt.ok).toBe(false);
    expect(receipt.error).toBe('lane_receipt_missing');
    expect(receipt.type).toBe('conduit_lane_bridge_error');
  });

  test('runLaneViaConduit catches client send errors', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-bridge-throw-'));
    fs.mkdirSync(path.join(temp, 'core', 'layer0', 'ops'), { recursive: true });
    fs.mkdirSync(path.join(temp, 'client', 'runtime', 'systems', 'conduit'), { recursive: true });
    fs.writeFileSync(path.join(temp, 'Cargo.toml'), '[package]\nname="tmp"\nversion="0.0.0"\n');
    fs.writeFileSync(path.join(temp, 'core', 'layer0', 'ops', 'Cargo.toml'), '[package]\nname="ops"\nversion="0.0.0"\n');
    fs.writeFileSync(
      path.join(temp, 'client', 'runtime', 'systems', 'conduit', 'conduit-client.js'),
      `module.exports = {
  ConduitClient: {
    overStdio() {
      return {
        send: async () => { throw new Error('boom-send'); },
        close: async () => {}
      };
    }
  }
};\n`,
      'utf8',
    );

    const receipt = await bridge.runLaneViaConduit('system-lane', temp);
    expect(receipt.ok).toBe(false);
    expect(String(receipt.error || '')).toContain('boom-send');
  });

  test('createConduitLaneModule verify reflects normalized lane id and daemon args env', async () => {
    const prevCommand = process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND;
    const prevArgs = process.env.PROTHEUS_CONDUIT_DAEMON_ARGS;
    process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND = '/tmp/mock-daemon';
    process.env.PROTHEUS_CONDUIT_DAEMON_ARGS = '--a 1 --b 2';

    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-bridge-verify-'));
    fs.mkdirSync(path.join(temp, 'core', 'layer0', 'ops'), { recursive: true });
    fs.mkdirSync(path.join(temp, 'client', 'runtime', 'systems', 'conduit'), { recursive: true });
    fs.writeFileSync(path.join(temp, 'Cargo.toml'), '[package]\nname="tmp"\nversion="0.0.0"\n');
    fs.writeFileSync(path.join(temp, 'core', 'layer0', 'ops', 'Cargo.toml'), '[package]\nname="ops"\nversion="0.0.0"\n');
    const clientModulePath = path.join(temp, 'client', 'runtime', 'systems', 'conduit', 'conduit-client.js');
    fs.writeFileSync(
      clientModulePath,
      `globalThis.__infringBridgeLast = null;
module.exports = {
  ConduitClient: {
    overStdio(command, args, root) {
      globalThis.__infringBridgeLast = { command, args, root };
      return {
        send: async () => ({
          event: {
            type: 'system_feedback',
            detail: { lane_receipt: { ok: true, lane_id: 'SYSTEM-LANE', receipt_hash: 'r' } }
          }
        }),
        close: async () => {}
      };
    }
  }
};\n`,
      'utf8',
    );

    const lane = bridge.createConduitLaneModule('system-lane', temp);
    expect(await lane.verifyLaneReceipt()).toBe(true);

    const seen = (globalThis as any).__infringBridgeLast;
    expect(seen.command).toBe('/tmp/mock-daemon');
    expect(seen.args).toEqual(['--a', '1', '--b', '2']);

    if (prevCommand == null) delete process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND;
    else process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND = prevCommand;
    if (prevArgs == null) delete process.env.PROTHEUS_CONDUIT_DAEMON_ARGS;
    else process.env.PROTHEUS_CONDUIT_DAEMON_ARGS = prevArgs;
  });
});
