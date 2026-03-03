#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');

type AnyObj = Record<string, any>;

type EmbeddedVaultPolicyRule = {
  id: string;
  objective: string;
  zk_requirement: string;
  fhe_requirement: string;
  severity: string;
  fail_closed: boolean;
};

type EmbeddedVaultAutoRotatePolicy = {
  enabled: boolean;
  rotate_after_hours: number;
  max_key_age_hours: number;
  grace_window_minutes: number;
  quorum_required: number;
  emergency_rotate_on_tamper: boolean;
};

type EmbeddedVaultPolicy = {
  policy_id: string;
  version: number;
  key_domain: string;
  cryptographic_profile: string;
  attestation_chain: string[];
  auto_rotate: EmbeddedVaultAutoRotatePolicy;
  rules: EmbeddedVaultPolicyRule[];
};

type VaultOperationRequest = {
  operation_id: string;
  key_id: string;
  action: string;
  zk_proof?: string | null;
  ciphertext_digest?: string | null;
  fhe_noise_budget: number;
  key_age_hours: number;
  tamper_signal: boolean;
  operator_quorum: number;
  audit_receipt_nonce?: string | null;
};

const MIN_FHE_NOISE_BUDGET = 12;

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function hasValue(v: unknown) {
  return cleanText(v, 500).length > 0;
}

function defaultPolicy(): EmbeddedVaultPolicy {
  return {
    policy_id: 'vault_policy_primary',
    version: 1,
    key_domain: 'protheus_runtime_vault',
    cryptographic_profile: 'fhe_bfv+zkp_groth16',
    attestation_chain: [
      'hsm_root_attestation',
      'runtime_measurement_attestation',
      'operator_dual_control_attestation'
    ],
    auto_rotate: {
      enabled: true,
      rotate_after_hours: 24,
      max_key_age_hours: 72,
      grace_window_minutes: 20,
      quorum_required: 2,
      emergency_rotate_on_tamper: true
    },
    rules: [
      {
        id: 'vault.zk.required',
        objective: 'Every seal/unseal request carries non-interactive zero-knowledge proof.',
        zk_requirement: 'proof_required_for_key_open',
        fhe_requirement: 'ciphertext_only_in_compute_lane',
        severity: 'critical',
        fail_closed: true
      },
      {
        id: 'vault.fhe.policy',
        objective: 'Homomorphic operations remain bounded and deterministic.',
        zk_requirement: 'proof_links_ciphertext_to_policy',
        fhe_requirement: 'noise_budget_min_threshold',
        severity: 'high',
        fail_closed: true
      },
      {
        id: 'vault.rotation.window',
        objective: 'Automatic key rotation executes before max age or immediately after tamper signal.',
        zk_requirement: 'proof_of_previous_key_revocation',
        fhe_requirement: 'reencrypt_on_rotate',
        severity: 'critical',
        fail_closed: true
      },
      {
        id: 'vault.audit.trace',
        objective: 'Every key event emits signed immutable receipt.',
        zk_requirement: 'proof_receipt_binding',
        fhe_requirement: 'receipt_contains_ciphertext_digest',
        severity: 'medium',
        fail_closed: true
      }
    ]
  };
}

function normalizePolicy(input: AnyObj): EmbeddedVaultPolicy {
  const base = defaultPolicy();
  const inRules = Array.isArray(input && input.rules) ? input.rules : base.rules;
  return {
    policy_id: cleanText(input && input.policy_id || base.policy_id, 160),
    version: Number.isFinite(Number(input && input.version)) ? Math.max(1, Math.floor(Number(input.version))) : base.version,
    key_domain: cleanText(input && input.key_domain || base.key_domain, 160),
    cryptographic_profile: cleanText(input && input.cryptographic_profile || base.cryptographic_profile, 160),
    attestation_chain: (Array.isArray(input && input.attestation_chain) ? input.attestation_chain : base.attestation_chain)
      .map((v: unknown) => cleanText(v, 160))
      .filter(Boolean),
    auto_rotate: {
      enabled: Boolean(input && input.auto_rotate && input.auto_rotate.enabled != null ? input.auto_rotate.enabled : base.auto_rotate.enabled),
      rotate_after_hours: Number.isFinite(Number(input && input.auto_rotate && input.auto_rotate.rotate_after_hours))
        ? Math.max(1, Math.floor(Number(input.auto_rotate.rotate_after_hours)))
        : base.auto_rotate.rotate_after_hours,
      max_key_age_hours: Number.isFinite(Number(input && input.auto_rotate && input.auto_rotate.max_key_age_hours))
        ? Math.max(1, Math.floor(Number(input.auto_rotate.max_key_age_hours)))
        : base.auto_rotate.max_key_age_hours,
      grace_window_minutes: Number.isFinite(Number(input && input.auto_rotate && input.auto_rotate.grace_window_minutes))
        ? Math.max(0, Math.floor(Number(input.auto_rotate.grace_window_minutes)))
        : base.auto_rotate.grace_window_minutes,
      quorum_required: Number.isFinite(Number(input && input.auto_rotate && input.auto_rotate.quorum_required))
        ? Math.max(1, Math.floor(Number(input.auto_rotate.quorum_required)))
        : base.auto_rotate.quorum_required,
      emergency_rotate_on_tamper: Boolean(input && input.auto_rotate && input.auto_rotate.emergency_rotate_on_tamper != null
        ? input.auto_rotate.emergency_rotate_on_tamper
        : base.auto_rotate.emergency_rotate_on_tamper)
    },
    rules: inRules.map((r: AnyObj) => ({
      id: cleanText(r && r.id, 120),
      objective: cleanText(r && r.objective, 260),
      zk_requirement: cleanText(r && r.zk_requirement, 160),
      fhe_requirement: cleanText(r && r.fhe_requirement, 160),
      severity: cleanText(r && r.severity, 40).toLowerCase(),
      fail_closed: Boolean(r && r.fail_closed)
    }))
  };
}

function normalizeRequest(raw: AnyObj): VaultOperationRequest {
  return {
    operation_id: cleanText(raw && raw.operation_id, 160),
    key_id: cleanText(raw && raw.key_id, 160),
    action: cleanText(raw && raw.action, 64).toLowerCase(),
    zk_proof: raw && raw.zk_proof != null ? cleanText(raw.zk_proof, 400) : null,
    ciphertext_digest: raw && raw.ciphertext_digest != null ? cleanText(raw.ciphertext_digest, 400) : null,
    fhe_noise_budget: Number.isFinite(Number(raw && raw.fhe_noise_budget)) ? Math.max(0, Math.floor(Number(raw.fhe_noise_budget))) : 0,
    key_age_hours: Number.isFinite(Number(raw && raw.key_age_hours)) ? Math.max(0, Math.floor(Number(raw.key_age_hours))) : 0,
    tamper_signal: Boolean(raw && raw.tamper_signal),
    operator_quorum: Number.isFinite(Number(raw && raw.operator_quorum)) ? Math.max(0, Math.floor(Number(raw.operator_quorum))) : 0,
    audit_receipt_nonce: raw && raw.audit_receipt_nonce != null ? cleanText(raw.audit_receipt_nonce, 200) : null
  };
}

function policyDigest(policy: EmbeddedVaultPolicy) {
  const parts: string[] = [
    policy.policy_id,
    String(policy.version),
    policy.key_domain,
    policy.cryptographic_profile,
    String(policy.auto_rotate.enabled),
    String(policy.auto_rotate.rotate_after_hours),
    String(policy.auto_rotate.max_key_age_hours),
    String(policy.auto_rotate.grace_window_minutes),
    String(policy.auto_rotate.quorum_required),
    String(policy.auto_rotate.emergency_rotate_on_tamper)
  ];
  for (const item of policy.attestation_chain) parts.push(item);
  for (const rule of policy.rules) {
    parts.push(rule.id);
    parts.push(rule.objective);
    parts.push(rule.zk_requirement);
    parts.push(rule.fhe_requirement);
    parts.push(rule.severity);
    parts.push(String(rule.fail_closed));
  }
  return crypto.createHash('sha256').update(parts.join('|'), 'utf8').digest('hex');
}

function autoRotateSignal(policy: EmbeddedVaultPolicy, request: VaultOperationRequest) {
  if (!policy.auto_rotate.enabled) return { should_rotate: false, rotate_reason: null };
  if (request.tamper_signal && policy.auto_rotate.emergency_rotate_on_tamper) {
    return { should_rotate: true, rotate_reason: 'tamper_signal_detected' };
  }
  if (request.key_age_hours >= policy.auto_rotate.rotate_after_hours) {
    return {
      should_rotate: true,
      rotate_reason: `key_age_exceeds_rotate_after:${policy.auto_rotate.rotate_after_hours}h`
    };
  }
  return { should_rotate: false, rotate_reason: null };
}

function evaluateVaultPolicyLegacy(inputReq: AnyObj, inputPolicy?: AnyObj) {
  const policy = normalizePolicy(inputPolicy && typeof inputPolicy === 'object' ? inputPolicy : defaultPolicy());
  const request = normalizeRequest(inputReq && typeof inputReq === 'object' ? inputReq : {});
  const rotateSignal = autoRotateSignal(policy, request);

  const rule_results: AnyObj[] = [];
  const reasons: string[] = [];
  let should_rotate = rotateSignal.should_rotate;

  for (const rule of policy.rules) {
    let passed = true;
    let reason = 'unrecognized_rule_treated_as_pass';

    if (rule.id === 'vault.zk.required') {
      const requiresZk = ['seal', 'unseal', 'rotate'].includes(request.action);
      passed = !requiresZk || hasValue(request.zk_proof);
      reason = passed ? 'zk_proof_validated' : 'zk_proof_missing';
    } else if (rule.id === 'vault.fhe.policy') {
      const hasCipher = hasValue(request.ciphertext_digest);
      const noiseOk = request.fhe_noise_budget >= MIN_FHE_NOISE_BUDGET;
      passed = hasCipher && noiseOk;
      reason = passed
        ? 'fhe_constraints_satisfied'
        : (!hasCipher ? 'ciphertext_digest_missing' : `fhe_noise_budget_below_min:${request.fhe_noise_budget}<${MIN_FHE_NOISE_BUDGET}`);
    } else if (rule.id === 'vault.rotation.window') {
      const exceedsMaxAge = request.key_age_hours > policy.auto_rotate.max_key_age_hours;
      const quorumOk = request.operator_quorum >= policy.auto_rotate.quorum_required;
      const rotateAction = request.action === 'rotate';

      if (exceedsMaxAge && !rotateAction) {
        should_rotate = true;
        passed = false;
        reason = `key_age_exceeds_max_without_rotate:${request.key_age_hours}>${policy.auto_rotate.max_key_age_hours}`;
      } else if (request.tamper_signal && !rotateAction) {
        should_rotate = true;
        passed = false;
        reason = 'tamper_requires_immediate_rotate';
      } else if (rotateSignal.should_rotate && !quorumOk) {
        passed = false;
        reason = `rotate_quorum_insufficient:${request.operator_quorum}<${policy.auto_rotate.quorum_required}`;
      } else {
        reason = rotateSignal.should_rotate ? 'rotation_window_enforced' : 'rotation_not_required';
      }
    } else if (rule.id === 'vault.audit.trace') {
      passed = hasValue(request.audit_receipt_nonce);
      reason = passed ? 'audit_receipt_bound' : 'audit_receipt_nonce_missing';
    }

    if (!passed) reasons.push(`${rule.id}:${reason}`);
    rule_results.push({
      rule_id: rule.id,
      passed,
      fail_closed: Boolean(rule.fail_closed),
      reason
    });
  }

  const allowed = rule_results.every((r) => r.passed === true);
  const fail_closed = rule_results.some((r) => r.passed !== true && r.fail_closed === true);
  const status = allowed ? 'allow' : (fail_closed ? 'deny_fail_closed' : 'deny_soft');

  return {
    policy_id: policy.policy_id,
    policy_digest: policyDigest(policy),
    operation_id: request.operation_id,
    key_id: request.key_id,
    action: request.action,
    allowed,
    fail_closed,
    status,
    should_rotate,
    rotate_reason: rotateSignal.rotate_reason,
    reasons,
    rule_results
  };
}

function loadLegacyVaultPolicy(input?: AnyObj) {
  return normalizePolicy(input && typeof input === 'object' ? input : defaultPolicy());
}

module.exports = {
  loadLegacyVaultPolicy,
  evaluateVaultPolicyLegacy,
  policyDigest
};
