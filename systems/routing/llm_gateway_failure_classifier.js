#!/usr/bin/env node
'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
const fs = require('fs');
const path = require('path');
const ROOT = process.env.LLM_GATEWAY_FAILURE_CLASSIFIER_ROOT
    ? path.resolve(process.env.LLM_GATEWAY_FAILURE_CLASSIFIER_ROOT)
    : path.resolve(__dirname, '..', '..');
const DEFAULT_POLICY_PATH = process.env.LLM_GATEWAY_FAILURE_CLASSIFIER_POLICY_PATH
    ? path.resolve(process.env.LLM_GATEWAY_FAILURE_CLASSIFIER_POLICY_PATH)
    : path.join(ROOT, 'config', 'llm_gateway_failure_classifier_policy.json');
function nowIso() {
    return new Date().toISOString();
}
function cleanText(v, maxLen = 320) {
    return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}
function normalizeToken(v, maxLen = 120) {
    return cleanText(v, maxLen)
        .toLowerCase()
        .replace(/[^a-z0-9_.:/-]+/g, '_')
        .replace(/_+/g, '_')
        .replace(/^_+|_+$/g, '');
}
function toBool(v, fallback = false) {
    if (v == null)
        return fallback;
    const raw = String(v).trim().toLowerCase();
    if (['1', 'true', 'yes', 'on'].includes(raw))
        return true;
    if (['0', 'false', 'no', 'off'].includes(raw))
        return false;
    return fallback;
}
function clampInt(v, lo, hi, fallback) {
    const n = Number(v);
    if (!Number.isFinite(n))
        return fallback;
    const i = Math.floor(n);
    if (i < lo)
        return lo;
    if (i > hi)
        return hi;
    return i;
}
function parseArgs(argv) {
    const out = { _: [] };
    for (const token of argv) {
        if (!String(token || '').startsWith('--')) {
            out._.push(String(token || ''));
            continue;
        }
        const idx = token.indexOf('=');
        if (idx < 0)
            out[String(token).slice(2)] = true;
        else
            out[String(token).slice(2, idx)] = String(token).slice(idx + 1);
    }
    return out;
}
function usage() {
    console.log('Usage:');
    console.log('  node systems/routing/llm_gateway_failure_classifier.js run [--hours=24] [--include-ok=0] [--strict=1|0] [--policy=<path>]');
    console.log('  node systems/routing/llm_gateway_failure_classifier.js status [--policy=<path>]');
}
function ensureDir(dirPath) {
    fs.mkdirSync(dirPath, { recursive: true });
}
function readJson(filePath, fallback = {}) {
    try {
        if (!fs.existsSync(filePath))
            return fallback;
        const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
        return parsed && typeof parsed === 'object' ? parsed : fallback;
    }
    catch {
        return fallback;
    }
}
function readJsonl(filePath) {
    try {
        if (!fs.existsSync(filePath))
            return [];
        return String(fs.readFileSync(filePath, 'utf8') || '')
            .split('\n')
            .filter(Boolean)
            .map((line) => {
            try {
                return JSON.parse(line);
            }
            catch {
                return null;
            }
        })
            .filter(Boolean);
    }
    catch {
        return [];
    }
}
function writeJsonAtomic(filePath, value) {
    ensureDir(path.dirname(filePath));
    const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
    fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
    fs.renameSync(tmp, filePath);
}
function appendJsonl(filePath, row) {
    ensureDir(path.dirname(filePath));
    fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}
function resolvePath(raw, fallbackRel) {
    const txt = cleanText(raw || '', 600);
    if (!txt)
        return path.join(ROOT, fallbackRel);
    return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}
function rel(absPath) {
    return path.relative(ROOT, absPath).replace(/\\/g, '/');
}
function defaultPolicy() {
    return {
        version: '1.0',
        enabled: true,
        default_hours: 24,
        include_ok_default: false,
        gateway_log_path: 'state/routing/llm_gateway_calls.jsonl',
        canonical_events_dir: 'state/runtime/canonical_events',
        latest_path: 'state/routing/llm_gateway_failure_classifier/latest.json',
        receipts_path: 'state/routing/llm_gateway_failure_classifier/receipts.jsonl'
    };
}
function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
    const base = defaultPolicy();
    const raw = readJson(policyPath, {});
    return {
        version: cleanText(raw.version || base.version, 24) || base.version,
        enabled: raw.enabled !== false,
        default_hours: clampInt(raw.default_hours, 1, 24 * 180, base.default_hours),
        include_ok_default: toBool(raw.include_ok_default, base.include_ok_default),
        gateway_log_path: resolvePath(raw.gateway_log_path || base.gateway_log_path, base.gateway_log_path),
        canonical_events_dir: resolvePath(raw.canonical_events_dir || base.canonical_events_dir, base.canonical_events_dir),
        latest_path: resolvePath(raw.latest_path || base.latest_path, base.latest_path),
        receipts_path: resolvePath(raw.receipts_path || base.receipts_path, base.receipts_path),
        policy_path: path.resolve(policyPath)
    };
}
function classifyGateway(row) {
    const code = normalizeToken(row.error_code || row.error || row.block_reason || row.stderr || '', 160);
    const blocked = row.blocked === true || row.ok === false;
    if (!blocked)
        return { category: 'ok', reason: 'ok' };
    if (code.includes('test_opacity'))
        return { category: 'opacity_gate', reason: code || 'test_opacity_blocked' };
    if (code.includes('timeout'))
        return { category: 'timeout', reason: code };
    if (code.includes('budget') || code.includes('burn'))
        return { category: 'budget_gate', reason: code };
    if (code.includes('provider') || code.includes('api'))
        return { category: 'provider_error', reason: code };
    if (code.includes('policy') || code.includes('deny'))
        return { category: 'policy_denied', reason: code };
    return { category: 'gateway_unknown', reason: code || 'gateway_unknown' };
}
function classifyCanonical(row) {
    const payload = row && row.payload && typeof row.payload === 'object' ? row.payload : {};
    const opcode = normalizeToken(row.opcode || '', 80).toUpperCase() || 'UNKNOWN';
    const adapter = normalizeToken(payload.adapter_kind || '', 120) || 'unknown';
    const policyDecision = normalizeToken(payload.policy_decision || '', 40) || 'unknown';
    if (payload && payload.dry_run === true && row.ok === false) {
        return { category: 'dry_run_non_success', reason: `${opcode}:${adapter}:dry_run_false`, opcode, adapter, policy_decision: policyDecision };
    }
    if (policyDecision === 'deny') {
        return { category: 'policy_denied', reason: `${opcode}:${adapter}:policy_denied`, opcode, adapter, policy_decision: policyDecision };
    }
    return { category: 'primitive_failure', reason: `${opcode}:${adapter}:unknown`, opcode, adapter, policy_decision: policyDecision };
}
function listCanonicalFiles(dirPath) {
    if (!fs.existsSync(dirPath))
        return [];
    return fs.readdirSync(dirPath)
        .filter((name) => name.endsWith('.jsonl'))
        .map((name) => path.join(dirPath, name))
        .filter((absPath) => {
        try {
            return fs.statSync(absPath).isFile();
        }
        catch {
            return false;
        }
    })
        .sort((a, b) => a.localeCompare(b));
}
function runClassifier(args) {
    const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
    const hours = clampInt(args.hours, 1, 24 * 180, policy.default_hours);
    const includeOk = toBool(args['include-ok'] ?? args.include_ok, policy.include_ok_default);
    const sinceMs = Date.now() - (hours * 3600000);
    const categoryCounts = {};
    const reasonCounts = {};
    const fingerprints = {};
    const samples = [];
    let scannedGateway = 0;
    let scannedCanonical = 0;
    const gatewayRows = readJsonl(policy.gateway_log_path);
    for (const row of gatewayRows) {
        const tsMs = Date.parse(String(row && row.ts || ''));
        if (!Number.isFinite(tsMs) || tsMs < sinceMs)
            continue;
        scannedGateway += 1;
        const cls = classifyGateway(row || {});
        if (!includeOk && cls.category === 'ok')
            continue;
        categoryCounts[cls.category] = Number(categoryCounts[cls.category] || 0) + 1;
        reasonCounts[cls.reason] = Number(reasonCounts[cls.reason] || 0) + 1;
        const fp = `${cls.category}|${cls.reason}`;
        fingerprints[fp] = Number(fingerprints[fp] || 0) + 1;
        if (samples.length < 32) {
            samples.push({
                source: 'llm_gateway',
                ts: row.ts || null,
                category: cls.category,
                reason: cls.reason,
                call_id: cleanText(row.call_id || '', 120) || null,
                model: cleanText(row.model || '', 80) || null
            });
        }
    }
    const canonicalFiles = listCanonicalFiles(policy.canonical_events_dir);
    for (const filePath of canonicalFiles) {
        const rows = readJsonl(filePath);
        for (const row of rows) {
            const tsMs = Date.parse(String(row && row.ts || ''));
            if (!Number.isFinite(tsMs) || tsMs < sinceMs)
                continue;
            if (String(row.type || '') !== 'primitive_execution')
                continue;
            if (String(row.phase || '') !== 'finish')
                continue;
            scannedCanonical += 1;
            if (row.ok !== false)
                continue;
            const cls = classifyCanonical(row || {});
            categoryCounts[cls.category] = Number(categoryCounts[cls.category] || 0) + 1;
            reasonCounts[cls.reason] = Number(reasonCounts[cls.reason] || 0) + 1;
            const fp = `${cls.category}|${cls.reason}`;
            fingerprints[fp] = Number(fingerprints[fp] || 0) + 1;
            if (samples.length < 32) {
                samples.push({
                    source: 'canonical_event',
                    ts: row.ts || null,
                    category: cls.category,
                    reason: cls.reason,
                    event_id: row.event_id || null,
                    opcode: row.opcode || null
                });
            }
        }
    }
    const totalIssues = Object.values(categoryCounts).reduce((acc, n) => acc + Number(n || 0), 0);
    const topCategories = Object.entries(categoryCounts)
        .sort((a, b) => Number(b[1]) - Number(a[1]))
        .slice(0, 12)
        .map(([category, count]) => ({ category, count }));
    const topReasons = Object.entries(reasonCounts)
        .sort((a, b) => Number(b[1]) - Number(a[1]))
        .slice(0, 12)
        .map(([reason, count]) => ({ reason, count }));
    const topFingerprints = Object.entries(fingerprints)
        .sort((a, b) => Number(b[1]) - Number(a[1]))
        .slice(0, 12)
        .map(([fingerprint, count]) => ({ fingerprint, count }));
    const recommendations = [];
    if (topCategories.some((row) => row.category === 'dry_run_non_success')) {
        recommendations.push('Treat dry-run finish=false as advisory telemetry unless adapter marks fatal_error=true.');
    }
    if (topCategories.some((row) => row.category === 'opacity_gate')) {
        recommendations.push('Review opaque-test prompts and lockout windows to reduce false-positive test opacity blocks.');
    }
    if (topCategories.some((row) => row.category === 'budget_gate')) {
        recommendations.push('Rebalance budget gates or queue policy for sustained deny pressure.');
    }
    if (topCategories.some((row) => row.category === 'provider_error')) {
        recommendations.push('Probe provider readiness and circuit-breaker thresholds for failing upstream models.');
    }
    const out = {
        ok: true,
        type: 'llm_gateway_failure_classifier',
        ts: nowIso(),
        hours,
        include_ok: includeOk,
        totals: {
            scanned_gateway_rows: scannedGateway,
            scanned_canonical_rows: scannedCanonical,
            classified_issues: totalIssues
        },
        top_categories: topCategories,
        top_reasons: topReasons,
        top_fingerprints: topFingerprints,
        recommendations,
        samples,
        paths: {
            gateway_log_path: rel(policy.gateway_log_path),
            canonical_events_dir: rel(policy.canonical_events_dir),
            latest_path: rel(policy.latest_path),
            receipts_path: rel(policy.receipts_path),
            policy_path: rel(policy.policy_path)
        }
    };
    writeJsonAtomic(policy.latest_path, out);
    appendJsonl(policy.receipts_path, out);
    return out;
}
function cmdStatus(args) {
    const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
    const latest = readJson(policy.latest_path, null);
    if (!latest || typeof latest !== 'object') {
        return {
            ok: false,
            type: 'llm_gateway_failure_classifier_status',
            reason: 'status_not_found',
            latest_path: rel(policy.latest_path)
        };
    }
    return {
        ok: true,
        type: 'llm_gateway_failure_classifier_status',
        ts: nowIso(),
        latest,
        latest_path: rel(policy.latest_path),
        receipts_path: rel(policy.receipts_path)
    };
}
function main() {
    const args = parseArgs(process.argv.slice(2));
    const cmd = normalizeToken(args._[0] || '', 64);
    if (!cmd || cmd === 'help' || args.help) {
        usage();
        process.exit(0);
    }
    if (cmd === 'run') {
        const out = runClassifier(args);
        process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
        return;
    }
    if (cmd === 'status') {
        const out = cmdStatus(args);
        process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
        if (!out.ok)
            process.exit(1);
        return;
    }
    usage();
    process.exit(2);
}
if (require.main === module) {
    main();
}
module.exports = {
    DEFAULT_POLICY_PATH,
    loadPolicy,
    runClassifier,
    cmdStatus
};
