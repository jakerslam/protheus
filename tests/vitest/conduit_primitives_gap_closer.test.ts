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

  test.skip('overUnixSocket path works for single roundtrip', async () => {
    if (process.platform === 'win32') return;
    const conduit = await import(pathToFileURL(path.join(ROOT, 'client/runtime/systems/conduit/conduit-client.ts')).href);
    const socketPath = path.join(os.tmpdir(), `pc-${process.pid}-${Date.now()}.sock`);
    if (fs.existsSync(socketPath)) {
      fs.unlinkSync(socketPath);
    }

    const server = net.createServer((socket) => {
      let buffer = '';
      socket.setEncoding('utf8');
      socket.on('data', (chunk) => {
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
        socket.write(JSON.stringify(response) + '\\n');
      });
    });

    await new Promise<void>((resolve, reject) => {
      server.listen(socketPath, () => resolve());
      server.once('error', reject);
    });

    const client = conduit.ConduitClient.overUnixSocket(socketPath);
    const response = await client.send({ type: 'get_system_status' }, 'req-socket-1');
    await client.close();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    if (fs.existsSync(socketPath)) {
      fs.unlinkSync(socketPath);
    }

    expect(response.request_id).toBe('req-socket-1');
    expect((response.event as any).detail.command_type).toBe('get_system_status');
  }, 10_000);
});

describe('direct conduit lane bridge coverage paths', () => {
  test('findRepoRoot resolves workspace root from nested directory', async () => {
    const bridge = await import(pathToFileURL(path.join(ROOT, 'client/runtime/lib/direct_conduit_lane_bridge.ts')).href);
    const found = bridge.findRepoRoot(path.join(ROOT, 'client', 'runtime', 'systems', 'ops'));
    expect(found).toBe(ROOT);
  });

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
