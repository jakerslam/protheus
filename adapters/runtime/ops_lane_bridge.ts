'use strict';
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawn, spawnSync } = require('child_process');
function repoRoot(scriptDir) {
    let dir = path.resolve(scriptDir || process.cwd());
    while (true) {
        const cargo = path.join(dir, 'Cargo.toml');
        const coreOps = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
        const legacyOps = path.join(dir, 'crates', 'ops', 'Cargo.toml');
        if (fs.existsSync(cargo) && (fs.existsSync(coreOps) || fs.existsSync(legacyOps))) {
            return dir;
        }
        const parent = path.dirname(dir);
        if (parent === dir)
            break;
        dir = parent;
    }
    return path.resolve(scriptDir || process.cwd(), '..', '..', '..');
}
function parseJsonPayload(stdout) {
    const raw = String(stdout || '').trim();
    if (!raw)
        return null;
    try {
        return JSON.parse(raw);
    }
    catch { }
    const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
    for (let i = lines.length - 1; i >= 0; i -= 1) {
        const line = lines[i];
        if (!line || line[0] !== '{')
            continue;
        try {
            return JSON.parse(line);
        }
        catch { }
    }
    return null;
}
function encodeBase64(value) {
    return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}
function normalizeStatus(v) {
    return Number.isFinite(Number(v)) ? Number(v) : 1;
}
function parseTimeoutMs(name, fallbackMs, minMs = 1000, maxMs = 300000) {
    const raw = Number(process.env[name]);
    if (!Number.isFinite(raw))
        return fallbackMs;
    return Math.max(minMs, Math.min(maxMs, Math.floor(raw)));
}
function statMtimeMs(filePath) {
    try {
        return fs.statSync(filePath).mtimeMs || 0;
    }
    catch {
        return 0;
    }
}
function opsSourceNewestMtimeMs(root) {
    const candidates = [
        path.join(root, 'core', 'layer0', 'ops', 'Cargo.toml'),
        path.join(root, 'core', 'layer0', 'ops', 'src')
    ];
    let newest = 0;
    const visit = (candidate) => {
        try {
            const stat = fs.statSync(candidate);
            newest = Math.max(newest, stat.mtimeMs || 0);
            if (!stat.isDirectory())
                return;
            for (const entry of fs.readdirSync(candidate)) {
                visit(path.join(candidate, entry));
            }
        }
        catch { }
    };
    for (const candidate of candidates) {
        visit(candidate);
    }
    return newest;
}
function binaryFreshEnough(root, binPath) {
    const binMtime = statMtimeMs(binPath);
    if (!binMtime)
        return false;
    const srcMtime = opsSourceNewestMtimeMs(root);
    if (!srcMtime)
        return true;
    return binMtime >= srcMtime;
}
function deferOnHostStallEnabled() {
    const raw = String(process.env.PROTHEUS_OPS_DEFER_ON_HOST_STALL || '0').trim().toLowerCase();
    return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}
function isTimeoutLikeSpawnError(err) {
    if (!err)
        return false;
    const code = String(err.code || '');
    if (code.toUpperCase() === 'ETIMEDOUT')
        return true;
    const msg = String(err.message || err);
    return /\b(etimedout|timed out|timeout)\b/i.test(msg);
}
function defaultEnv() {
    return {
        ...process.env,
        PROTHEUS_NODE_BINARY: process.execPath || 'node'
    };
}
function envBool(names, fallback = false) {
    for (const name of names) {
        const raw = String(process.env[name] || '').trim().toLowerCase();
        if (!raw)
            continue;
        if (raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on')
            return true;
        if (raw === '0' || raw === 'false' || raw === 'no' || raw === 'off')
            return false;
    }
    return fallback;
}
function releaseChannel(env = process.env) {
    const raw = String(env.INFRING_RELEASE_CHANNEL || env.PROTHEUS_RELEASE_CHANNEL || '').trim().toLowerCase();
    return raw || 'stable';
}
function isProductionReleaseChannel(channel) {
    const normalized = String(channel || '').trim().toLowerCase();
    return (normalized === 'stable'
        || normalized === 'production'
        || normalized === 'prod'
        || normalized === 'ga'
        || normalized === 'release');
}
function processFallbackPolicy() {
    const requested = envBool(['INFRING_OPS_ALLOW_PROCESS_FALLBACK', 'PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK'], false);
    if (!requested) {
        return {
            enabled: false,
            reason: 'process_fallback_disabled',
            release_channel: releaseChannel()
        };
    }
    const channel = releaseChannel();
    if (isProductionReleaseChannel(channel)) {
        return {
            enabled: false,
            reason: 'process_fallback_forbidden_in_production',
            release_channel: channel
        };
    }
    return {
        enabled: true,
        reason: 'process_fallback_enabled',
        release_channel: channel
    };
}
function sleepMs(ms) {
    const timeout = Math.max(1, Math.floor(Number(ms) || 0));
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, timeout);
}
function pruneIpcQueueFiles(queueDir, maxAgeMs) {
    const threshold = Date.now() - Math.max(1000, Math.floor(Number(maxAgeMs) || 0));
    const targets = [path.join(queueDir, 'requests'), path.join(queueDir, 'responses')];
    for (const target of targets) {
        let entries = [];
        try {
            entries = fs.readdirSync(target);
        }
        catch {
            continue;
        }
        for (const entry of entries) {
            if (!entry.endsWith('.json')) {
                continue;
            }
            const abs = path.join(target, entry);
            try {
                const stat = fs.statSync(abs);
                if ((stat.mtimeMs || 0) < threshold) {
                    fs.rmSync(abs, { force: true });
                }
            }
            catch { }
        }
    }
}
function ipcBridgeEnabled() {
    return envBool(['INFRING_OPS_IPC_DAEMON', 'PROTHEUS_OPS_IPC_DAEMON'], true);
}
function ipcStrictModeEnabled() {
    return envBool(['INFRING_OPS_IPC_STRICT', 'PROTHEUS_OPS_IPC_STRICT'], true);
}
function processFallbackEnabled() {
    return processFallbackPolicy().enabled;
}
function queueRootForRepo(root) {
    const hash = crypto.createHash('sha256').update(String(root || '')).digest('hex').slice(0, 16);
    return path.join(root, 'local', 'state', 'tools', 'ops_bridge_ipc', hash);
}
const ipcDaemonRegistry = new Map();
function daemonHeartbeatFile(queueDir) {
    return path.join(queueDir, 'daemon.heartbeat.json');
}
function daemonPidFile(queueDir) {
    return path.join(queueDir, 'daemon.pid.json');
}
function heartbeatFresh(queueDir, ttlMs) {
    try {
        const stat = fs.statSync(daemonHeartbeatFile(queueDir));
        const ageMs = Date.now() - Number(stat.mtimeMs || 0);
        return ageMs >= 0 && ageMs <= Math.max(500, Math.floor(Number(ttlMs) || 0));
    }
    catch {
        return false;
    }
}
function heartbeatRequiredHealth(queueDir, ttlMs) {
    const heartbeatPath = daemonHeartbeatFile(queueDir);
    if (!fs.existsSync(heartbeatPath)) {
        return true;
    }
    return heartbeatFresh(queueDir, ttlMs);
}
function waitForHeartbeat(queueDir, maxWaitMs, ttlMs) {
    const deadline = Date.now() + Math.max(100, Math.floor(Number(maxWaitMs) || 0));
    while (Date.now() <= deadline) {
        if (heartbeatFresh(queueDir, ttlMs)) {
            return true;
        }
        sleepMs(25);
    }
    return false;
}
function pidAlive(pid) {
    const target = Number(pid || 0);
    if (!Number.isFinite(target) || target <= 0) {
        return false;
    }
    try {
        process.kill(target, 0);
        return true;
    }
    catch {
        return false;
    }
}
function readDaemonPidState(queueDir) {
    const pidPath = daemonPidFile(queueDir);
    try {
        const raw = fs.readFileSync(pidPath, 'utf8');
        const parsed = JSON.parse(String(raw || '{}'));
        const pid = Number(parsed && parsed.pid);
        if (!pidAlive(pid)) {
            return null;
        }
        return {
            pid,
            started_at_ms: Number(parsed.started_at_ms || 0),
            rust_command: String(parsed.rust_command || ''),
            rust_args: Array.isArray(parsed.rust_args) ? parsed.rust_args.map((row) => String(row || '')) : []
        };
    }
    catch {
        return null;
    }
}
function writeDaemonPidState(queueDir, state) {
    const payload = {
        pid: Number(state && state.pid ? state.pid : 0),
        started_at_ms: Date.now(),
        rust_command: String(state && state.rust_command ? state.rust_command : ''),
        rust_args: Array.isArray(state && state.rust_args) ? state.rust_args : []
    };
    try {
        fs.writeFileSync(daemonPidFile(queueDir), `${JSON.stringify(payload)}\n`, 'utf8');
    }
    catch { }
}
function removeDaemonPidState(queueDir) {
    try {
        fs.rmSync(daemonPidFile(queueDir), { force: true });
    }
    catch { }
    try {
        fs.rmSync(daemonHeartbeatFile(queueDir), { force: true });
    }
    catch { }
}
function makeDaemonState(resolved, queueDir, pollMs, pid) {
    return {
        queueDir,
        requestsDir: path.join(queueDir, 'requests'),
        responsesDir: path.join(queueDir, 'responses'),
        pollMs,
        pid: Number(pid || 0),
        rust_command: resolved.command,
        rust_args: [resolved.command].concat(resolved.args.concat([
            'ipc-daemon',
            `--queue-dir=${queueDir}`,
            `--poll-ms=${pollMs}`
        ]))
    };
}
function stopDaemonPid(pid) {
    const target = Number(pid || 0);
    if (!Number.isFinite(target) || target <= 0) {
        return;
    }
    try {
        process.kill(target, 'SIGTERM');
    }
    catch { }
}
function clearOpsIpcDaemon(root, terminate = false) {
    const key = String(root || '');
    const state = ipcDaemonRegistry.get(key);
    if (state && terminate) {
        stopDaemonPid(state.pid);
        removeDaemonPidState(state.queueDir);
    }
    if (!state && terminate) {
        const queueDir = queueRootForRepo(root);
        const pidState = readDaemonPidState(queueDir);
        if (pidState) {
            stopDaemonPid(pidState.pid);
        }
        removeDaemonPidState(queueDir);
    }
    ipcDaemonRegistry.delete(key);
}
function ensureOpsIpcDaemon(root, options = {}) {
    const forceRestart = options && options.forceRestart === true;
    const key = String(root || '');
    if (forceRestart) {
        clearOpsIpcDaemon(root, true);
    }
    const resolved = resolveProtheusOpsCommand(root, 'ops-domain-conduit-runner-kernel');
    const pollMs = parseTimeoutMs('INFRING_OPS_IPC_POLL_MS', 20, 5, 1000);
    const heartbeatTtlMs = parseTimeoutMs('INFRING_OPS_IPC_HEARTBEAT_TTL_MS', 5000, 500, 60000);
    const queueDir = queueRootForRepo(root);
    const requestsDir = path.join(queueDir, 'requests');
    const responsesDir = path.join(queueDir, 'responses');
    fs.mkdirSync(requestsDir, { recursive: true });
    fs.mkdirSync(responsesDir, { recursive: true });
    const staleMs = parseTimeoutMs('INFRING_OPS_IPC_STALE_MS', 600000, 1000, 86400000);
    pruneIpcQueueFiles(queueDir, staleMs);
    const cached = ipcDaemonRegistry.get(key);
    if (cached
        && cached.queueDir
        && cached.requestsDir
        && cached.responsesDir
        && pidAlive(cached.pid)
        && heartbeatRequiredHealth(cached.queueDir, heartbeatTtlMs)) {
        return cached;
    }
    if (cached && !pidAlive(cached.pid)) {
        ipcDaemonRegistry.delete(key);
    }
    const pidState = readDaemonPidState(queueDir);
    if (pidState && pidAlive(pidState.pid) && heartbeatRequiredHealth(queueDir, heartbeatTtlMs)) {
        const state = makeDaemonState(resolved, queueDir, pollMs, pidState.pid);
        ipcDaemonRegistry.set(key, state);
        return state;
    }
    removeDaemonPidState(queueDir);
    const daemonArgs = resolved.args.concat([
        'ipc-daemon',
        `--queue-dir=${queueDir}`,
        `--poll-ms=${pollMs}`
    ]);
    const child = spawn(resolved.command, daemonArgs, {
        cwd: root,
        env: defaultEnv(),
        stdio: 'ignore',
        detached: true
    });
    child.unref();
    waitForHeartbeat(queueDir, parseTimeoutMs('INFRING_OPS_IPC_BOOT_WAIT_MS', 1200, 100, 10000), heartbeatTtlMs);
    const state = makeDaemonState(resolved, queueDir, pollMs, child.pid || 0);
    writeDaemonPidState(queueDir, state);
    ipcDaemonRegistry.set(key, state);
    return state;
}
function shouldRetryIpc(result) {
    if (!result) {
        return true;
    }
    if (result.ok || result.status === 0) {
        return false;
    }
    const type = String(result.payload && result.payload.type ? result.payload.type : '').toLowerCase();
    return type === 'ops_domain_ipc_timeout'
        || type === 'ops_domain_ipc_daemon_unavailable'
        || type === 'ops_domain_ipc_request_write_failed'
        || type === 'ops_domain_ipc_response_read_failed';
}
function runLocalOpsDomainViaIpcOnce(root, domain, passArgs, cliMode, inheritStdio, forceRestart = false) {
    if (cliMode && inheritStdio) {
        return null;
    }
    let daemon;
    try {
        daemon = ensureOpsIpcDaemon(root, { forceRestart });
    }
    catch (err) {
        return {
            ok: false,
            status: 1,
            stdout: '',
            stderr: String(err && err.message ? err.message : err),
            payload: {
                ok: false,
                type: 'ops_domain_ipc_daemon_unavailable',
                reason: String(err && err.message ? err.message : err),
                domain
            },
            error: err,
            rust_command: null,
            rust_args: [],
            timeout_ms: parseTimeoutMs('PROTHEUS_OPS_LOCAL_TIMEOUT_MS', 45000),
            routed_via: 'ipc_daemon'
        };
    }
    const timeoutMs = parseTimeoutMs('PROTHEUS_OPS_LOCAL_TIMEOUT_MS', 45000);
    const requestId = `req_${Date.now()}_${process.pid}_${Math.floor(Math.random() * 1_000_000_000)}`;
    const requestPath = path.join(daemon.requestsDir, `${requestId}.json`);
    const responsePath = path.join(daemon.responsesDir, `${requestId}.json`);
    const request = {
        id: requestId,
        domain: String(domain || ''),
        args: Array.isArray(passArgs) ? passArgs.slice(0) : []
    };
    try {
        fs.writeFileSync(requestPath, `${JSON.stringify(request)}\n`, 'utf8');
    }
    catch (err) {
        return {
            ok: false,
            status: 1,
            stdout: '',
            stderr: String(err && err.message ? err.message : err),
            payload: {
                ok: false,
                type: 'ops_domain_ipc_request_write_failed',
                reason: String(err && err.message ? err.message : err),
                domain
            },
            error: err,
            rust_command: daemon.rust_command,
            rust_args: daemon.rust_args,
            timeout_ms: timeoutMs,
            routed_via: 'ipc_daemon'
        };
    }
    const deadline = Date.now() + timeoutMs;
    while (Date.now() <= deadline) {
        if (fs.existsSync(responsePath)) {
            let raw = '';
            try {
                raw = String(fs.readFileSync(responsePath, 'utf8') || '');
            }
            catch (err) {
                return {
                    ok: false,
                    status: 1,
                    stdout: '',
                    stderr: String(err && err.message ? err.message : err),
                    payload: {
                        ok: false,
                        type: 'ops_domain_ipc_response_read_failed',
                        reason: String(err && err.message ? err.message : err),
                        domain
                    },
                    error: err,
                    rust_command: daemon.rust_command,
                    rust_args: daemon.rust_args,
                    timeout_ms: timeoutMs,
                    routed_via: 'ipc_daemon'
                };
            }
            finally {
                try {
                    fs.rmSync(responsePath, { force: true });
                }
                catch { }
            }
            const parsed = parseJsonPayload(raw) || {};
            const response = parsed.response && typeof parsed.response === 'object'
                ? parsed.response
                : parsed;
            const status = normalizeStatus(response.status);
            const payload = response.payload && typeof response.payload === 'object'
                ? response.payload
                : {
                    ok: status === 0,
                    type: status === 0 ? 'ops_domain_ipc_result' : 'ops_domain_ipc_error',
                    reason: status === 0 ? 'ok' : 'missing_payload',
                    domain
                };
            return {
                ok: status === 0 && payload.ok !== false,
                status,
                stdout: `${JSON.stringify(payload)}\n`,
                stderr: '',
                payload,
                error: null,
                rust_command: daemon.rust_command,
                rust_args: daemon.rust_args,
                timeout_ms: timeoutMs,
                routed_via: 'ipc_daemon'
            };
        }
        sleepMs(Math.max(5, Math.min(100, Number(daemon.pollMs || 5))));
    }
    try {
        fs.rmSync(requestPath, { force: true });
    }
    catch { }
    return {
        ok: false,
        status: 1,
        stdout: '',
        stderr: 'ops_domain_ipc_timeout',
        payload: {
            ok: false,
            type: 'ops_domain_ipc_timeout',
            reason: `timed out waiting for daemon response after ${timeoutMs}ms`,
            domain,
            timeout_ms: timeoutMs
        },
        error: null,
        rust_command: daemon.rust_command,
        rust_args: daemon.rust_args,
        timeout_ms: timeoutMs,
        routed_via: 'ipc_daemon'
    };
}
function runLocalOpsDomainViaIpc(root, domain, passArgs, cliMode, inheritStdio) {
    const initial = runLocalOpsDomainViaIpcOnce(root, domain, passArgs, cliMode, inheritStdio, false);
    if (!initial || !shouldRetryIpc(initial)) {
        return initial;
    }
    const retry = runLocalOpsDomainViaIpcOnce(root, domain, passArgs, cliMode, inheritStdio, true);
    if (retry && !retry.ok && retry.payload && typeof retry.payload === 'object') {
        retry.payload.retry_after_restart = true;
    }
    return retry || initial;
}
function resolveProtheusOpsCommand(root, domain) {
    const preferCargo = envBool(['INFRING_OPS_PREFER_CARGO', 'PROTHEUS_OPS_PREFER_CARGO'], false);
    const usePrebuiltOnly = envBool(['INFRING_OPS_USE_PREBUILT', 'PROTHEUS_OPS_USE_PREBUILT'], false);
    const allowCargoFallback = envBool(['INFRING_OPS_ALLOW_CARGO_FALLBACK', 'PROTHEUS_OPS_ALLOW_CARGO_FALLBACK'], true);
    const explicit = String(process.env.INFRING_OPS_BIN || process.env.PROTHEUS_OPS_BIN || '').trim();
    if (explicit) {
        return {
            command: explicit,
            args: [domain]
        };
    }
    const prebuiltCandidates = [
        path.join(root, 'target', 'release-speed', 'infring-ops'),
        path.join(root, 'target', 'release', 'infring-ops'),
        path.join(root, 'target', 'release-speed', 'protheus-ops'),
        path.join(root, 'target', 'release', 'protheus-ops'),
        path.join(root, 'target', 'debug', 'infring-ops'),
        path.join(root, 'target', 'debug', 'protheus-ops')
    ];
    if (!preferCargo) {
        for (const candidate of prebuiltCandidates) {
            if (!fs.existsSync(candidate))
                continue;
            if (usePrebuiltOnly || binaryFreshEnough(root, candidate)) {
                return {
                    command: candidate,
                    args: [domain]
                };
            }
        }
    }
    if (!allowCargoFallback) {
        return {
            command: path.join(root, 'target', 'release', 'infring-ops'),
            args: [domain]
        };
    }
    return {
        command: 'cargo',
        args: [
            'run',
            '--quiet',
            '--manifest-path',
            'core/layer0/ops/Cargo.toml',
            '--bin',
            'infring-ops',
            '--',
            domain
        ]
    };
}
function runLocalOpsDomainOnce(root, domain, passArgs, cliMode, inheritStdio, resolved) {
    const commandArgs = resolved.args.concat(Array.isArray(passArgs) ? passArgs : []);
    const timeoutMs = parseTimeoutMs('PROTHEUS_OPS_LOCAL_TIMEOUT_MS', 45000);
    const run = spawnSync(resolved.command, commandArgs, {
        cwd: root,
        encoding: 'utf8',
        env: defaultEnv(),
        stdio: cliMode && inheritStdio ? 'inherit' : undefined,
        timeout: timeoutMs,
        maxBuffer: 1024 * 1024 * 4
    });
    if (deferOnHostStallEnabled() && isTimeoutLikeSpawnError(run.error)) {
        const payload = {
            ok: true,
            type: 'ops_domain_deferred_host_stall',
            reason_code: 'deferred_host_stall',
            raw_error_code: String(run.error.code || ''),
            domain,
            timeout_ms: timeoutMs
        };
        return {
            ok: true,
            status: 0,
            stdout: cliMode && inheritStdio ? '' : `${JSON.stringify(payload)}\n`,
            stderr: String(run.error && run.error.message ? run.error.message : run.error),
            payload,
            rust_command: resolved.command,
            rust_args: [resolved.command, ...commandArgs],
            timeout_ms: timeoutMs,
            routed_via: 'core_local',
            deferred_host_stall: true
        };
    }
    const status = run.error ? 1 : normalizeStatus(run.status);
    const stdout = run.stdout || '';
    const stderr = `${run.stderr || ''}${run.error ? `\n${String(run.error && run.error.message ? run.error.message : run.error)}` : ''}`;
    const payload = cliMode && inheritStdio ? null : parseJsonPayload(stdout);
    if (!payload && run.error) {
        return {
            ok: false,
            status,
            stdout,
            stderr,
            payload: {
                ok: false,
                type: 'ops_domain_spawn_error',
                reason: String(run.error && run.error.message ? run.error.message : run.error),
                raw_error_code: String(run.error.code || ''),
                domain
            },
            error: run.error,
            rust_command: resolved.command,
            rust_args: [resolved.command, ...commandArgs],
            timeout_ms: timeoutMs,
            routed_via: 'core_local'
        };
    }
    return {
        ok: status === 0,
        status,
        stdout,
        stderr,
        payload,
        error: run.error || null,
        rust_command: resolved.command,
        rust_args: [resolved.command, ...commandArgs],
        timeout_ms: timeoutMs,
        routed_via: 'core_local'
    };
}
function shouldRetryWithCargo(result) {
    if (!result || result.status === 0)
        return false;
    const rawErrorCode = String((result.payload && result.payload.raw_error_code)
        || (result.error && result.error.code)
        || '').toLowerCase();
    if (rawErrorCode === 'enoent' || rawErrorCode === 'eacces') {
        return true;
    }
    const reason = String((result.payload && result.payload.reason)
        || (result.payload && result.payload.error)
        || result.stderr
        || '').toLowerCase();
    return reason.includes('unknown_domain') || reason.includes('unknown_command');
}
function markProcessTransportFallback(result, reason) {
    if (!result || typeof result !== 'object') {
        return result;
    }
    const normalizedReason = String(reason || 'ipc_unavailable');
    const payload = result.payload && typeof result.payload === 'object'
        ? result.payload
        : {};
    result.payload = {
        ...payload,
        process_transport_fallback: true,
        process_transport_reason: normalizedReason,
        transport_mode: 'spawn_sync_process'
    };
    result.transport_mode = 'spawn_sync_process';
    result.process_transport_fallback = true;
    result.process_transport_reason = normalizedReason;
    return result;
}
function runLocalOpsDomain(root, domain, passArgs, cliMode, inheritStdio) {
    const fallbackPolicy = processFallbackPolicy();
    const fallbackAllowed = fallbackPolicy.enabled;
    const fallbackDeniedReason = fallbackPolicy.reason;
    let processFallbackReason = 'ipc_disabled';
    if (ipcBridgeEnabled()) {
        const viaIpc = runLocalOpsDomainViaIpc(root, domain, passArgs, cliMode, inheritStdio);
        if (viaIpc && (viaIpc.ok || viaIpc.status === 0 || (ipcStrictModeEnabled() && !fallbackAllowed))) {
            return viaIpc;
        }
        if (!fallbackAllowed) {
            if (viaIpc) {
                viaIpc.payload = viaIpc.payload && typeof viaIpc.payload === 'object'
                    ? {
                        ...viaIpc.payload,
                        fallback_blocked: true,
                        reason: fallbackDeniedReason,
                        release_channel: fallbackPolicy.release_channel
                    }
                    : {
                        ok: false,
                        type: 'ipc_transport_failed',
                        reason: fallbackDeniedReason,
                        release_channel: fallbackPolicy.release_channel,
                        fallback_blocked: true
                    };
                viaIpc.stderr = `${String(viaIpc.stderr || '')}\n${fallbackDeniedReason}`;
                viaIpc.routed_via = viaIpc.routed_via || 'ipc_only';
                return viaIpc;
            }
            const payload = {
                ok: false,
                type: 'ipc_transport_unavailable',
                reason: fallbackDeniedReason,
                release_channel: fallbackPolicy.release_channel,
                domain,
                routed_via: 'ipc_only'
            };
            return {
                ok: false,
                status: 1,
                stdout: cliMode && inheritStdio ? '' : `${JSON.stringify(payload)}\n`,
                stderr: `ipc_transport_unavailable_${fallbackDeniedReason}`,
                payload,
                rust_command: null,
                rust_args: [],
                routed_via: 'ipc_only'
            };
        }
        processFallbackReason = viaIpc ? 'ipc_failed' : 'ipc_unavailable';
    }
    else if (!fallbackAllowed) {
        const payload = {
            ok: false,
            type: 'ipc_transport_disabled',
            reason: fallbackDeniedReason,
            release_channel: fallbackPolicy.release_channel,
            domain,
            routed_via: 'ipc_only'
        };
        return {
            ok: false,
            status: 1,
            stdout: cliMode && inheritStdio ? '' : `${JSON.stringify(payload)}\n`,
            stderr: `ipc_transport_disabled_${fallbackDeniedReason}`,
            payload,
            rust_command: null,
            rust_args: [],
            routed_via: 'ipc_only'
        };
    }
    const resolved = resolveProtheusOpsCommand(root, domain);
    const initial = markProcessTransportFallback(runLocalOpsDomainOnce(root, domain, passArgs, cliMode, inheritStdio, resolved), processFallbackReason);
    if (resolved.command === 'cargo' || !shouldRetryWithCargo(initial)) {
        return initial;
    }
    const cargoResolved = {
        command: 'cargo',
        args: [
            'run',
            '--quiet',
            '--manifest-path',
            'core/layer0/ops/Cargo.toml',
            '--bin',
            'infring-ops',
            '--',
            domain
        ]
    };
    const retried = markProcessTransportFallback(runLocalOpsDomainOnce(root, domain, passArgs, cliMode, inheritStdio, cargoResolved), processFallbackReason);
    if (retried.ok || retried.status === 0) {
        retried.fallback_reason = 'stale_prebuilt_retry';
        return retried;
    }
    return initial;
}
function runBridge(config, args = [], cliMode = false) {
    const root = repoRoot(config.scriptDir);
    const passArgs = Array.isArray(args) ? args.slice(0) : [];
    if (config.mode === 'ops_domain') {
        if (config.preferLocalCore === true) {
            const local = runLocalOpsDomain(root, config.domain, passArgs, cliMode, config.inheritStdio);
            return {
                ...local,
                lane: config.lane
            };
        }
        const kernelPayload = encodeBase64(JSON.stringify({
            argv: ['--domain', config.domain].concat(passArgs)
        }));
        const kernelRun = runLocalOpsDomain(
            root,
            'ops-domain-conduit-runner-kernel',
            ['run', `--payload-base64=${kernelPayload}`],
            cliMode,
            config.inheritStdio
        );
        const nested = kernelRun
            && kernelRun.payload
            && typeof kernelRun.payload === 'object'
            && kernelRun.payload.payload
            && typeof kernelRun.payload.payload === 'object'
            ? kernelRun.payload.payload
            : null;
        if (!nested) {
            return {
                ...kernelRun,
                lane: config.lane,
                routed_via: 'conduit'
            };
        }
        const nestedStatus = Number.isFinite(Number(nested.status))
            ? Number(nested.status)
            : normalizeStatus(kernelRun.status);
        const nestedPayload = nested.payload && typeof nested.payload === 'object'
            ? nested.payload
            : {
                ok: nestedStatus === 0,
                type: nestedStatus === 0 ? 'ops_domain_conduit_bridge_result' : 'ops_domain_conduit_bridge_error',
                reason: nestedStatus === 0 ? 'ok' : 'missing_result_payload',
                routed_via: 'core_local'
            };
        return {
            ...kernelRun,
            ok: nestedStatus === 0 && nestedPayload.ok !== false,
            status: nestedStatus,
            payload: nestedPayload,
            lane: config.lane,
            routed_via: 'conduit'
        };
    }
    if (config.mode === 'manifest_binary') {
        const payload = {
            ok: false,
            type: 'conduit_only_enforced',
            reason: 'direct_manifest_binary_execution_blocked_route_via_conduit',
            lane: config.lane,
            manifest_path: config.manifestPath,
            binary_name: config.binaryName
        };
        return {
            ok: false,
            status: 1,
            stdout: cliMode && config.inheritStdio ? '' : JSON.stringify(payload),
            stderr: 'conduit_only_enforced',
            payload,
            lane: config.lane,
            rust_command: null,
            rust_args: [],
            routed_via: 'conduit_policy'
        };
    }
    throw new Error('invalid_rust_lane_bridge_config');
}
function runCliWithOutput(out, inheritStdio) {
    if (!inheritStdio) {
        if (out.stdout)
            process.stdout.write(out.stdout);
        if (out.stderr)
            process.stderr.write(out.stderr);
    }
    process.exit(out.status);
}
function createOpsLaneBridge(scriptDir, lane, domain, opts = {}) {
    process.env.PROTHEUS_OPS_USE_PREBUILT =
        process.env.PROTHEUS_OPS_USE_PREBUILT || '1';
    process.env.PROTHEUS_OPS_DEFER_ON_HOST_STALL =
        process.env.PROTHEUS_OPS_DEFER_ON_HOST_STALL || '0';
    process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
        process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '20000';
    const config = {
        scriptDir,
        lane,
        domain: String(domain || '').trim(),
        mode: 'ops_domain',
        inheritStdio: opts.inheritStdio === true,
        preferLocalCore: opts.preferLocalCore === true
    };
    function run(args = []) {
        return runBridge(config, args, false);
    }
    function runCli(args = []) {
        const out = runBridge(config, args, config.inheritStdio === true);
        runCliWithOutput(out, config.inheritStdio);
    }
    return {
        lane,
        run,
        runCli
    };
}
function createManifestLaneBridge(scriptDir, lane, options) {
    const config = {
        scriptDir,
        lane,
        manifestPath: options.manifestPath,
        binaryName: options.binaryName,
        binaryEnvVar: options.binaryEnvVar,
        preArgs: options.preArgs || [],
        mode: 'manifest_binary',
        inheritStdio: options.inheritStdio === true
    };
    function run(args = []) {
        return runBridge(config, args, false);
    }
    function runCli(args = []) {
        const out = runBridge(config, args, config.inheritStdio === true);
        runCliWithOutput(out, config.inheritStdio);
    }
    return {
        lane,
        run,
        runCli
    };
}
module.exports = {
    createOpsLaneBridge,
    createManifestLaneBridge
};
