#!/usr/bin/env node
'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.
const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');
normalizeOpsBridgeEnvAliases();
function resolveDirectivesDir() {
    const explicit = String(process.env.DIRECTIVE_RESOLVER_DIRECTIVES_DIR
        || process.env.INFRING_DIRECTIVE_RESOLVER_DIRECTIVES_DIR
        || process.env.INFRING_DIRECTIVE_RESOLVER_DIRECTIVES_DIR
        || '').trim();
    if (explicit)
        return path.resolve(explicit);
    return path.join(__dirname, '..', 'config', 'directives');
}
const DIRECTIVES_DIR = resolveDirectivesDir();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'directive_resolver', 'directive-kernel');
function encodeBase64(value) {
    return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}
function normalizeConstraintsShape(value) {
    const base = value && typeof value === 'object' ? { ...value } : {};
    const domains = Array.isArray(base.high_stakes_domains) ? base.high_stakes_domains : [];
    return {
        tier: Number.isFinite(Number(base.tier)) ? Number(base.tier) : 0,
        hard_blocks: Array.isArray(base.hard_blocks) ? base.hard_blocks : [],
        approval_required: Array.isArray(base.approval_required) ? base.approval_required : [],
        risk_limits: base.risk_limits && typeof base.risk_limits === 'object' ? base.risk_limits : {},
        high_stakes_domains: new Set(domains.map((entry) => String(entry || '').toLowerCase()).filter(Boolean))
    };
}
function failClosedValidation(actionEnvelope, reason) {
    const envelope = actionEnvelope && typeof actionEnvelope === 'object' ? actionEnvelope : {};
    return {
        allowed: false,
        requires_approval: false,
        blocked_reason: String(reason || 'directive_kernel_bridge_failed'),
        approval_reason: null,
        effective_constraints: normalizeConstraintsShape(null),
        action_id: envelope.action_id || null,
        tier: Number.isFinite(Number(envelope.tier)) ? Number(envelope.tier) : 2,
        fail_closed: true
    };
}
function invokeDirectiveKernel(command, flags = {}) {
    const args = [command];
    for (const [key, value] of Object.entries(flags || {})) {
        if (value == null)
            continue;
        args.push(`--${key}=${value}`);
    }
    const out = bridge.run(args);
    const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
    return {
        ok: !!(payload && payload.ok === true),
        out,
        payload
    };
}
function parseYaml(text) {
    const call = invokeDirectiveKernel('parse-yaml', {
        'text-base64': encodeBase64(text)
    });
    if (!call.payload || !call.payload.parsed) {
        throw new Error(call.payload && (call.payload.error || call.payload.reason)
            ? String(call.payload.error || call.payload.reason)
            : 'directive_kernel_parse_yaml_failed');
    }
    return call.payload.parsed;
}
function validateTier1DirectiveQuality(content, directiveId = 'unknown') {
    const call = invokeDirectiveKernel('validate-tier1-quality', {
        'directive-id': directiveId,
        'text-base64': encodeBase64(content)
    });
    if (call.payload && call.payload.validation)
        return call.payload.validation;
    return {
        ok: false,
        directive_id: directiveId,
        missing: [],
        questions: [],
        error: call.payload && (call.payload.error || call.payload.reason)
            ? String(call.payload.error || call.payload.reason)
            : 'directive_kernel_validate_tier1_quality_failed'
    };
}
function loadActiveDirectives(options = {}) {
    const opts = options && typeof options === 'object' ? options : {};
    const call = invokeDirectiveKernel('active-directives', {
        'allow-missing': opts.allowMissing ? '1' : '0',
        'allow-weak-tier1': opts.allowWeakTier1 ? '1' : '0'
    });
    if (!call.payload || !Array.isArray(call.payload.directives)) {
        throw new Error(call.payload && (call.payload.error || call.payload.reason)
            ? String(call.payload.error || call.payload.reason)
            : 'directive_kernel_active_directives_failed');
    }
    return call.payload.directives;
}
function mergeConstraints(directives) {
    const call = invokeDirectiveKernel('merge-constraints', {
        'payload-base64': encodeBase64(JSON.stringify({
            directives: Array.isArray(directives) ? directives : []
        }))
    });
    if (!call.payload || !call.payload.constraints) {
        throw new Error(call.payload && (call.payload.error || call.payload.reason)
            ? String(call.payload.error || call.payload.reason)
            : 'directive_kernel_merge_constraints_failed');
    }
    return normalizeConstraintsShape(call.payload.constraints);
}
function validateAction(actionEnvelope) {
    const envelope = actionEnvelope && typeof actionEnvelope === 'object' ? actionEnvelope : {};
    const call = invokeDirectiveKernel('validate-action-envelope', {
        'payload-base64': encodeBase64(JSON.stringify({ action_envelope: envelope }))
    });
    if (!call.payload || !call.payload.validation) {
        return failClosedValidation(envelope, call.payload && (call.payload.error || call.payload.reason)
            ? String(call.payload.error || call.payload.reason)
            : 'directive_kernel_validate_action_envelope_failed');
    }
    const validation = call.payload.validation && typeof call.payload.validation === 'object'
        ? { ...call.payload.validation }
        : {};
    validation.effective_constraints = normalizeConstraintsShape(validation.effective_constraints);
    return validation;
}
function checkTierConflict(lowerTierAction, higherTierDirective) {
    const call = invokeDirectiveKernel('tier-conflict', {
        'payload-base64': encodeBase64(JSON.stringify({
            lower_tier_action: lowerTierAction && typeof lowerTierAction === 'object' ? lowerTierAction : {},
            higher_tier_directive: higherTierDirective && typeof higherTierDirective === 'object' ? higherTierDirective : {}
        }))
    });
    if (call.payload && call.payload.conflict)
        return call.payload.conflict;
    return {
        is_conflict: false
    };
}
module.exports = {
    loadActiveDirectives,
    mergeConstraints,
    validateAction,
    checkTierConflict,
    parseYaml,
    DIRECTIVES_DIR,
    validateTier1DirectiveQuality
};
