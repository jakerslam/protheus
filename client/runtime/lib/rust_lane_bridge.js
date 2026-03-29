'use strict';
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
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
function resolveProtheusOpsCommand(root, domain) {
    const preferCargo = String(process.env.PROTHEUS_OPS_PREFER_CARGO || '0').trim() === '1';
    const usePrebuiltOnly = String(process.env.PROTHEUS_OPS_USE_PREBUILT || '0').trim() === '1';
    const explicit = String(process.env.PROTHEUS_OPS_BIN || '').trim();
    if (explicit) {
        return {
            command: explicit,
            args: [domain]
        };
    }
    const release = path.join(root, 'target', 'release', 'protheus-ops');
    if (!preferCargo && fs.existsSync(release) && (usePrebuiltOnly || binaryFreshEnough(root, release))) {
        return {
            command: release,
            args: [domain]
        };
    }
    const debug = path.join(root, 'target', 'debug', 'protheus-ops');
    if (!preferCargo && fs.existsSync(debug) && (usePrebuiltOnly || binaryFreshEnough(root, debug))) {
        return {
            command: debug,
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
            'protheus-ops',
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
function runLocalOpsDomain(root, domain, passArgs, cliMode, inheritStdio) {
    const resolved = resolveProtheusOpsCommand(root, domain);
    const initial = runLocalOpsDomainOnce(root, domain, passArgs, cliMode, inheritStdio, resolved);
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
            'protheus-ops',
            '--',
            domain
        ]
    };
    const retried = runLocalOpsDomainOnce(root, domain, passArgs, cliMode, inheritStdio, cargoResolved);
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
