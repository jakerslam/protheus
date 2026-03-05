#!/usr/bin/env node
'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const ROOT = path.resolve(__dirname, '..', '..');
const MANIFEST = path.join(ROOT, 'crates', 'ops', 'Cargo.toml');
function cleanText(v, maxLen = 260) {
    return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}
function parseJsonPayload(raw) {
    const text = String(raw == null ? '' : raw).trim();
    if (!text)
        return null;
    try {
        return JSON.parse(text);
    }
    catch { }
    const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
    for (let i = lines.length - 1; i >= 0; i -= 1) {
        try {
            return JSON.parse(lines[i]);
        }
        catch { }
    }
    return null;
}
function binaryCandidates() {
    const explicit = cleanText(process.env.PROTHEUS_PERSONAS_RUST_BIN || '', 500);
    const out = [
        explicit,
        path.join(ROOT, 'target', 'release', 'personas_core'),
        path.join(ROOT, 'target', 'debug', 'personas_core'),
        path.join(ROOT, 'crates', 'ops', 'target', 'release', 'personas_core'),
        path.join(ROOT, 'crates', 'ops', 'target', 'debug', 'personas_core')
    ].filter(Boolean);
    return Array.from(new Set(out));
}
function runViaRustBinary(payloadBase64) {
    for (const candidate of binaryCandidates()) {
        try {
            if (!fs.existsSync(candidate))
                continue;
            const out = spawnSync(candidate, ['primitive', `--payload-base64=${payloadBase64}`], {
                cwd: ROOT,
                encoding: 'utf8',
                maxBuffer: 10 * 1024 * 1024
            });
            const payload = parseJsonPayload(out.stdout);
            if (out.status === 0 && payload && typeof payload === 'object') {
                return {
                    ok: true,
                    engine: 'rust_bin',
                    binary_path: candidate,
                    payload
                };
            }
        }
        catch {
            // continue
        }
    }
    return { ok: false, error: 'rust_binary_unavailable' };
}
function runViaCargo(payloadBase64) {
    const args = [
        'run',
        '--quiet',
        '--manifest-path',
        MANIFEST,
        '--bin',
        'personas_core',
        '--',
        'primitive',
        `--payload-base64=${payloadBase64}`
    ];
    const out = spawnSync('cargo', args, {
        cwd: ROOT,
        encoding: 'utf8',
        maxBuffer: 10 * 1024 * 1024
    });
    const payload = parseJsonPayload(out.stdout);
    if (Number(out.status) === 0 && payload && typeof payload === 'object') {
        return {
            ok: true,
            engine: 'rust_cargo',
            payload
        };
    }
    return {
        ok: false,
        error: `cargo_run_failed:${cleanText(out.stderr || out.stdout || '', 260)}`
    };
}
function runPersonasPrimitive(mode, data = {}, opts = {}) {
    const normalizedMode = cleanText(mode || '', 80).toLowerCase();
    if (!normalizedMode)
        return { ok: false, error: 'personas_mode_missing' };
    const request = {
        mode: normalizedMode,
        input: data && typeof data === 'object' ? data : {}
    };
    const payloadBase64 = Buffer.from(JSON.stringify(request), 'utf8').toString('base64');
    const bin = runViaRustBinary(payloadBase64);
    if (bin.ok)
        return bin;
    if (opts.allow_cli_fallback === false)
        return bin;
    return runViaCargo(payloadBase64);
}
module.exports = {
    runPersonasPrimitive,
    runViaRustBinary,
    runViaCargo
};
