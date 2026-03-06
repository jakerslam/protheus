#!/usr/bin/env node
'use strict';

/**
 * Runtime lane for SYSTEMS-IDENTITY-IDENTITY-ANCHOR.
 * Native execution delegated through conduit to Rust kernel runtime.
 */

const fs = require('fs');
const path = require('path');

function findRepoRoot(startDir) {
  let dir = path.resolve(startDir || process.cwd());
  while (true) {
    if (fs.existsSync(path.join(dir, 'Cargo.toml')) && fs.existsSync(path.join(dir, 'crates', 'ops', 'Cargo.toml'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return process.cwd();
    dir = parent;
  }
}

const ROOT = findRepoRoot(__dirname);
const LANE_ID = 'SYSTEMS-IDENTITY-IDENTITY-ANCHOR';

function loadConduitClient() {
  try {
    return require(path.join(ROOT, 'systems', 'conduit', 'conduit-client.js'));
  } catch {
    return require(path.join(ROOT, 'systems', 'conduit', 'conduit-client.ts'));
  }
}

function daemonCommand() {
  if (process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND) {
    return process.env.PROTHEUS_CONDUIT_DAEMON_COMMAND;
  }
  const fast = path.join(ROOT, 'target', 'debug', 'conduit_daemon');
  return fs.existsSync(fast) ? fast : 'cargo';
}

function daemonArgs(command) {
  const raw = process.env.PROTHEUS_CONDUIT_DAEMON_ARGS;
  if (raw && String(raw).trim()) {
    return String(raw)
      .trim()
      .split(/\s+/)
      .filter(Boolean);
  }
  return command === 'cargo'
    ? ['run', '--quiet', '-p', 'conduit', '--bin', 'conduit_daemon']
    : [];
}

async function buildLaneReceipt() {
  const { ConduitClient } = loadConduitClient();
  const command = daemonCommand();
  const client = ConduitClient.overStdio(command, daemonArgs(command), ROOT);

  try {
    const requestId = `lane-${LANE_ID}-${Date.now()}`;
    const response = await client.send(
      { type: 'start_agent', agent_id: `lane:${LANE_ID}` },
      requestId,
    );
    const laneReceipt =
      response &&
      response.event &&
      response.event.type === 'system_feedback' &&
      response.event.detail &&
      typeof response.event.detail === 'object'
        ? response.event.detail.lane_receipt
        : null;

    if (laneReceipt && typeof laneReceipt === 'object') {
      return laneReceipt;
    }

    return {
      ok: false,
      type: 'conduit_lane_bridge_error',
      lane_id: LANE_ID,
      error: 'lane_receipt_missing',
      conduit_response: response,
    };
  } catch (err) {
    return {
      ok: false,
      type: 'conduit_lane_bridge_error',
      lane_id: LANE_ID,
      error: String(err && err.message ? err.message : err),
    };
  } finally {
    await client.close().catch(() => {});
  }
}

async function verifyLaneReceipt() {
  const row = await buildLaneReceipt();
  return row && row.ok === true && String(row.lane_id || '') === LANE_ID;
}

module.exports = {
  LANE_ID,
  buildLaneReceipt,
  verifyLaneReceipt,
};

if (require.main === module) {
  buildLaneReceipt()
    .then((row) => {
      console.log(JSON.stringify(row, null, 2));
      process.exit(row && row.ok === true ? 0 : 1);
    })
    .catch((err) => {
      console.error(
        JSON.stringify(
          {
            ok: false,
            type: 'conduit_lane_bridge_error',
            lane_id: LANE_ID,
            error: String(err && err.message ? err.message : err),
          },
          null,
          2,
        ),
      );
      process.exit(1);
    });
}

export {};
