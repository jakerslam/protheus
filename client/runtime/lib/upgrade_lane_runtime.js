#!/usr/bin/env node
'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.
const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.js');
const { ROOT, nowIso, parseArgs, cleanText, normalizeToken, toBool, emit } = require('./queued_backlog_runtime.js');
process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'upgrade_lane_runtime', 'upgrade-lane-kernel');
function encodeBase64(value) {
    return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}
function invoke(command, payload = {}, opts = {}) {
    const out = bridge.run([
        command,
        `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
    ]);
    const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
    const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
        ? receipt.payload
        : receipt;
    if (out.status !== 0) {
        const message = payloadOut && typeof payloadOut.error === 'string'
            ? payloadOut.error
            : (out && out.stderr ? String(out.stderr).trim() : `upgrade_lane_kernel_${command}_failed`);
        if (opts.throwOnError !== false)
            throw new Error(message || `upgrade_lane_kernel_${command}_failed`);
        return { ok: false, error: message || `upgrade_lane_kernel_${command}_failed` };
    }
    if (!payloadOut || typeof payloadOut !== 'object') {
        const message = out && out.stderr
            ? String(out.stderr).trim() || `upgrade_lane_kernel_${command}_bridge_failed`
            : `upgrade_lane_kernel_${command}_bridge_failed`;
        if (opts.throwOnError !== false)
            throw new Error(message);
        return { ok: false, error: message };
    }
    return payloadOut;
}
function stripArgsMeta(args) {
    const out = {};
    for (const [key, value] of Object.entries(args || {})) {
        if (key === '_')
            continue;
        out[key] = value;
    }
    return out;
}
function defaultLaneType(scriptRel, laneId) {
    const fromScript = normalizeToken(path.basename(String(scriptRel || ''), '.js'), 120);
    if (fromScript)
        return fromScript;
    return normalizeToken(laneId || 'upgrade_lane', 120) || 'upgrade_lane';
}
function buildBasePayload(opts, args, laneId, laneType, cmd) {
    return {
        lane_id: laneId,
        lane_type: laneType,
        script_rel: cleanText(opts.script_rel || '', 260) || null,
        policy_path: String(opts.policy_path || ''),
        stream: cleanText(opts.stream || '', 180) || null,
        paths: opts.paths && typeof opts.paths === 'object' ? opts.paths : {},
        strict: args.strict,
        apply: args.apply,
        action: cmd
    };
}
function defaultConfigureRecord(cmdRecord, args) {
    return cmdRecord({}, {
        ...stripArgsMeta(args),
        action: 'configure',
        event: 'upgrade_lane_configure',
        payload_json: JSON.stringify({
            configured: true
        })
    });
}
function runStandardLane(opts) {
    const args = parseArgs(process.argv.slice(2));
    const laneId = cleanText(opts.lane_id || 'UNKNOWN-LANE', 120) || 'UNKNOWN-LANE';
    const laneType = defaultLaneType(opts.script_rel || '', laneId);
    const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
    if (args.help || cmd === 'help') {
        if (typeof opts.usage === 'function')
            opts.usage();
        emit({ ok: true, lane_id: laneId, type: `${laneType}_help`, action: 'help', ts: nowIso() }, 0);
    }
    const basePayload = buildBasePayload(opts, args, laneId, laneType, cmd);
    const strict = toBool(args.strict, true);
    const apply = toBool(args.apply, true);
    function cmdStatus() {
        return invoke('status', basePayload);
    }
    function cmdRecord(_policy, recordArgs) {
        return invoke('record', {
            ...basePayload,
            record_args: recordArgs && typeof recordArgs === 'object' ? recordArgs : {}
        });
    }
    const ctx = {
        cmdRecord,
        cmdStatus,
        ROOT,
        args,
        lane_id: laneId,
        lane_type: laneType,
        strict,
        apply
    };
    let result;
    if (cmd === 'status' && opts.handlers && typeof opts.handlers.status === 'function') {
        result = opts.handlers.status({}, args, ctx);
    }
    else if (cmd === 'status') {
        emit(cmdStatus(), 0);
    }
    else if (cmd === 'configure') {
        result = defaultConfigureRecord(cmdRecord, args);
    }
    else if (opts.handlers && typeof opts.handlers[cmd] === 'function') {
        result = opts.handlers[cmd]({}, args, ctx);
    }
    else {
        emit({
            ok: false,
            lane_id: laneId,
            type: `${laneType}_error`,
            action: cmd,
            ts: nowIso(),
            error: 'unsupported_command'
        }, 2);
    }
    if (result && typeof result.then === 'function') {
        result.then((resolved) => {
            const row = resolved && typeof resolved === 'object' ? resolved : { ok: false, error: 'handler_return_invalid' };
            const ok = row.ok !== false;
            emit(row, ok || !strict ? 0 : 1);
        }).catch((err) => {
            emit({
                ok: false,
                lane_id: laneId,
                type: `${laneType}_error`,
                action: cmd,
                ts: nowIso(),
                error: cleanText(err && err.message ? err.message : err, 260) || 'handler_failed'
            }, 1);
        });
        return;
    }
    const row = result && typeof result === 'object' ? result : { ok: false, error: 'handler_return_invalid' };
    const ok = row.ok !== false;
    emit(row, ok || !strict ? 0 : 1);
}
module.exports = {
    runStandardLane
};
