#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');
const { writeArtifactSet } = require('../../lib/state_artifact_contract');

const DEFAULT_POLICY_PATH = process.env.ILLUSION_INTEGRITY_AUDITOR_POLICY_PATH
  ? path.resolve(process.env.ILLUSION_INTEGRITY_AUDITOR_POLICY_PATH)
  : path.join(ROOT, 'config', 'illusion_integrity_auditor_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/self_audit/illusion_integrity_lane.js run [--trigger=manual|startup|promotion] [--strict=1|0] [--apply=0|1] [--approval-note="..."] [--consent-token=...] [--policy=<path>]');
  console.log('  node systems/self_audit/illusion_integrity_lane.js status [--policy=<path>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: false,
    lane_id: 'V4-SELF-001',
    signing_secret_env: 'ILLUSION_AUDIT_SIGNING_SECRET',
    signing_secret: 'illusion_audit_dev_secret',
    paths: {
      state_path: 'state/self_audit/illusion_integrity/state.json',
      latest_path: 'state/self_audit/illusion_integrity/latest.json',
      receipts_path: 'state/self_audit/illusion_integrity/receipts.jsonl',
      history_path: 'state/self_audit/illusion_integrity/history.jsonl',
      reports_dir: 'state/self_audit/illusion_integrity/reports',
      patches_dir: 'state/self_audit/illusion_integrity/patches'
    },
    engine: {
      mode: 'auto',
      timeout_ms: 90000,
      rust_manifest_path: 'systems/self_audit/rust/Cargo.toml',
      rust_binary_name: 'illusion-integrity-auditor',
      rust_source_path: 'systems/self_audit/illusion_integrity_auditor.rs',
      allow_ts_fallback: true
    },
    triggers: {
      startup: { enabled: true, strict: false },
      promotion: { enabled: true, strict: false },
      manual: { enabled: true, strict: false }
    },
    thresholds: {
      fail_score: 70,
      warn_score: 40,
      max_high_findings_before_fail: 2,
      min_ui_score: 80,
      min_scientific_score: 70
    },
    checks: {
      required_files: [
        'README.md',
        'CHANGELOG.md',
        'docs/ONBOARDING_PLAYBOOK.md',
        'docs/UI_SURFACE_MATURITY_MATRIX.md',
        'docs/HISTORY_CLEANLINESS.md',
        'docs/CLAIM_EVIDENCE_POLICY.md',
        'docs/PUBLIC_COLLABORATION_TRIAGE.md',
        '.github/ISSUE_TEMPLATE/bug_report.md',
        '.github/ISSUE_TEMPLATE/feature_request.md',
        '.github/ISSUE_TEMPLATE/security_report.md'
      ],
      ui_required_files: [
        'README.md',
        'docs/UI_SURFACE_MATURITY_MATRIX.md',
        'docs/ONBOARDING_PLAYBOOK.md',
        'docs/HISTORY_CLEANLINESS.md',
        '.github/ISSUE_TEMPLATE/bug_report.md',
        '.github/ISSUE_TEMPLATE/feature_request.md',
        '.github/ISSUE_TEMPLATE/security_report.md'
      ],
      scientific_required_files: [
        'systems/research/research_organ.ts',
        'systems/forge/forge_organ.ts',
        'systems/workflow/orchestron_controller.ts'
      ],
      suspicious_root_names: ['tmp', 'scratch', 'draft', 'personal', 'my', 'jay', '1', '2', '3']
    },
    git_metadata: {
      days_window: 14,
      min_commits_for_author_concentration_signal: 20,
      burst_window_minutes: 10,
      burst_threshold: 12
    },
    backlog_check: {
      script: 'systems/ops/backlog_registry.js',
      strict: false
    },
    autofix: {
      allow_apply: false,
      require_human_consent: true,
      required_approval_min_len: 12,
      required_token_prefix: 'consent_'
    }
  };
}

function asArrayTokens(input: any, maxLen = 260): string[] {
  if (!Array.isArray(input)) return [];
  return input
    .map((row) => cleanText(row, maxLen))
    .filter(Boolean);
}

function normalizePolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const engine = raw.engine && typeof raw.engine === 'object' ? raw.engine : {};
  const triggers = raw.triggers && typeof raw.triggers === 'object' ? raw.triggers : {};
  const thresholds = raw.thresholds && typeof raw.thresholds === 'object' ? raw.thresholds : {};
  const checks = raw.checks && typeof raw.checks === 'object' ? raw.checks : {};
  const gitMeta = raw.git_metadata && typeof raw.git_metadata === 'object' ? raw.git_metadata : {};
  const backlogCheck = raw.backlog_check && typeof raw.backlog_check === 'object' ? raw.backlog_check : {};
  const autofix = raw.autofix && typeof raw.autofix === 'object' ? raw.autofix : {};
  const triggerFor = (key: string, src: any, fallback: any) => {
    const row = src && typeof src[key] === 'object' ? src[key] : {};
    return {
      enabled: toBool(row.enabled, fallback.enabled !== false),
      strict: toBool(row.strict, !!fallback.strict)
    };
  };
  const modeRaw = normalizeToken(engine.mode || base.engine.mode, 32).toLowerCase();
  const mode = ['auto', 'rust_only', 'ts_only'].includes(modeRaw) ? modeRaw : base.engine.mode;
  return {
    version: cleanText(raw.version || base.version, 32) || '1.0',
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    lane_id: cleanText(raw.lane_id || base.lane_id, 120) || base.lane_id,
    signing_secret_env: cleanText(raw.signing_secret_env || base.signing_secret_env, 120) || base.signing_secret_env,
    signing_secret: cleanText(raw.signing_secret || base.signing_secret, 200) || base.signing_secret,
    paths: {
      state_path: resolvePath(paths.state_path || base.paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path || base.paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path || base.paths.receipts_path, base.paths.receipts_path),
      history_path: resolvePath(paths.history_path || base.paths.history_path, base.paths.history_path),
      reports_dir: resolvePath(paths.reports_dir || base.paths.reports_dir, base.paths.reports_dir),
      patches_dir: resolvePath(paths.patches_dir || base.paths.patches_dir, base.paths.patches_dir)
    },
    engine: {
      mode,
      timeout_ms: clampInt(engine.timeout_ms, 1000, 10 * 60 * 1000, base.engine.timeout_ms),
      rust_manifest_path: resolvePath(
        engine.rust_manifest_path || base.engine.rust_manifest_path,
        base.engine.rust_manifest_path
      ),
      rust_binary_name: cleanText(engine.rust_binary_name || base.engine.rust_binary_name, 120) || base.engine.rust_binary_name,
      rust_source_path: resolvePath(
        engine.rust_source_path || base.engine.rust_source_path,
        base.engine.rust_source_path
      ),
      allow_ts_fallback: toBool(engine.allow_ts_fallback, base.engine.allow_ts_fallback)
    },
    triggers: {
      startup: triggerFor('startup', triggers, base.triggers.startup),
      promotion: triggerFor('promotion', triggers, base.triggers.promotion),
      manual: triggerFor('manual', triggers, base.triggers.manual)
    },
    thresholds: {
      fail_score: clampInt(thresholds.fail_score, 1, 100, base.thresholds.fail_score),
      warn_score: clampInt(thresholds.warn_score, 1, 100, base.thresholds.warn_score),
      max_high_findings_before_fail: clampInt(
        thresholds.max_high_findings_before_fail,
        0,
        1000,
        base.thresholds.max_high_findings_before_fail
      ),
      min_ui_score: clampInt(thresholds.min_ui_score, 0, 100, base.thresholds.min_ui_score),
      min_scientific_score: clampInt(thresholds.min_scientific_score, 0, 100, base.thresholds.min_scientific_score)
    },
    checks: {
      required_files: asArrayTokens(checks.required_files).length
        ? asArrayTokens(checks.required_files)
        : base.checks.required_files.slice(0),
      ui_required_files: asArrayTokens(checks.ui_required_files).length
        ? asArrayTokens(checks.ui_required_files)
        : base.checks.ui_required_files.slice(0),
      scientific_required_files: asArrayTokens(checks.scientific_required_files).length
        ? asArrayTokens(checks.scientific_required_files)
        : base.checks.scientific_required_files.slice(0),
      suspicious_root_names: asArrayTokens(checks.suspicious_root_names, 120).length
        ? asArrayTokens(checks.suspicious_root_names, 120)
        : base.checks.suspicious_root_names.slice(0)
    },
    git_metadata: {
      days_window: clampInt(gitMeta.days_window, 1, 365, base.git_metadata.days_window),
      min_commits_for_author_concentration_signal: clampInt(
        gitMeta.min_commits_for_author_concentration_signal,
        1,
        100000,
        base.git_metadata.min_commits_for_author_concentration_signal
      ),
      burst_window_minutes: clampInt(gitMeta.burst_window_minutes, 1, 120, base.git_metadata.burst_window_minutes),
      burst_threshold: clampInt(gitMeta.burst_threshold, 1, 100000, base.git_metadata.burst_threshold)
    },
    backlog_check: {
      script: resolvePath(backlogCheck.script || base.backlog_check.script, base.backlog_check.script),
      strict: toBool(backlogCheck.strict, base.backlog_check.strict)
    },
    autofix: {
      allow_apply: toBool(autofix.allow_apply, base.autofix.allow_apply),
      require_human_consent: toBool(autofix.require_human_consent, base.autofix.require_human_consent),
      required_approval_min_len: clampInt(
        autofix.required_approval_min_len,
        1,
        1000,
        base.autofix.required_approval_min_len
      ),
      required_token_prefix: cleanText(
        autofix.required_token_prefix || base.autofix.required_token_prefix,
        120
      ) || base.autofix.required_token_prefix
    },
    policy_path: path.resolve(policyPath)
  };
}

function severityToScore(severity: string) {
  const token = String(severity || '').trim().toLowerCase();
  if (token === 'high') return 85;
  if (token === 'medium') return 60;
  return 35;
}

function scientificReasoningForFinding(finding: any) {
  const evidenceCount = Array.isArray(finding && finding.evidence) ? finding.evidence.length : 0;
  const severity = String(finding && finding.severity || 'low').toLowerCase();
  const priorRisk = severity === 'high' ? 0.8 : (severity === 'medium' ? 0.55 : 0.3);
  const confidence = Math.max(0.2, Math.min(0.98, 0.35 + (evidenceCount * 0.12)));
  const posterior = Math.max(0, Math.min(1, (priorRisk * 0.65) + (confidence * 0.35)));
  return {
    model: 'scientific_reasoning_v1',
    observe: {
      evidence_count: evidenceCount
    },
    hypothesize: {
      hypothesis: 'finding_represents_professional_surface_risk',
      prior_risk: Number(priorRisk.toFixed(4))
    },
    test: {
      confidence: Number(confidence.toFixed(4)),
      posterior_risk: Number(posterior.toFixed(4))
    },
    conclude: {
      severity_score: Math.round(posterior * 100),
      recommendation: severity === 'high' ? 'address_before_promotion' : 'queue_for_polish_cycle'
    }
  };
}

function normalizeFinding(row: any, source = 'ts') {
  const severity = String(row && row.severity || 'low').toLowerCase();
  const base = {
    id: cleanText(row && row.id || `finding_${Date.now()}`, 120),
    category: cleanText(row && row.category || 'general', 80) || 'general',
    title: cleanText(row && row.title || 'Unnamed finding', 180) || 'Unnamed finding',
    severity: ['high', 'medium', 'low'].includes(severity) ? severity : 'low',
    score: clampInt(row && row.score, 0, 100, severityToScore(severity)),
    summary: cleanText(row && row.summary || row && row.description || '', 400),
    path: row && row.path ? cleanText(row.path, 300) : null,
    evidence: Array.isArray(row && row.evidence)
      ? row.evidence.map((item: unknown) => cleanText(item, 220)).filter(Boolean)
      : [],
    safe_autofix: row && row.safe_autofix === true,
    patch_preview: row && row.patch_preview ? cleanText(row.patch_preview, 800) : null,
    source
  };
  return {
    ...base,
    scientific_reasoning: scientificReasoningForFinding(base)
  };
}

function runJsonCommand(command: string, args: string[], opts: any = {}) {
  const run = spawnSync(command, args, {
    cwd: opts.cwd || ROOT,
    encoding: 'utf8',
    timeout: opts.timeout
  });
  const stdout = String(run.stdout || '').trim();
  const stderr = String(run.stderr || '').trim();
  let payload = null;
  try { payload = stdout ? JSON.parse(stdout) : null; } catch {}
  return {
    ok: Number(run.status || 0) === 0,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    stdout,
    stderr,
    payload
  };
}

function runRustScan(policy: any, rootPath: string, trigger: string, args: any) {
  const mockPath = cleanText(args['mock-rust-file'] || args.mock_rust_file || '', 400);
  if (mockPath) {
    const abs = path.isAbsolute(mockPath) ? mockPath : path.join(ROOT, mockPath);
    const payload = readJson(abs, null);
    if (payload && typeof payload === 'object') {
      return {
        ok: true,
        payload,
        engine_used: 'rust_mock',
        transport: 'mock_file'
      };
    }
    return {
      ok: false,
      payload: null,
      engine_used: 'rust_mock',
      transport: 'mock_file',
      reason: 'mock_file_invalid'
    };
  }

  if (policy.engine.mode === 'ts_only') {
    return {
      ok: false,
      payload: null,
      engine_used: 'ts_only',
      transport: 'disabled',
      reason: 'rust_engine_disabled'
    };
  }

  const manifestPath = policy.engine.rust_manifest_path;
  const crateDir = path.dirname(manifestPath);
  const binaryPath = path.join(
    crateDir,
    'target',
    'release',
    process.platform === 'win32'
      ? `${policy.engine.rust_binary_name}.exe`
      : policy.engine.rust_binary_name
  );
  const requiredFilesArg = (policy.checks.required_files || []).join(',');
  const suspiciousArg = (policy.checks.suspicious_root_names || []).join(',');
  const baseArgs = [
    'scan',
    `--root=${rootPath}`,
    `--trigger=${trigger}`,
    `--required-files=${requiredFilesArg}`,
    `--suspicious-root-names=${suspiciousArg}`
  ];
  if (fs.existsSync(binaryPath)) {
    const run = runJsonCommand(binaryPath, baseArgs, { timeout: policy.engine.timeout_ms, cwd: ROOT });
    return {
      ok: run.ok && !!run.payload && run.payload.ok === true,
      payload: run.payload,
      engine_used: 'rust',
      transport: 'binary',
      status: run.status,
      reason: !run.ok ? (run.stderr || run.stdout || `rust_binary_exit_${run.status}`) : null
    };
  }
  const cargoArgs = ['run', '--quiet', '--manifest-path', manifestPath, '--', ...baseArgs];
  const run = runJsonCommand('cargo', cargoArgs, { timeout: policy.engine.timeout_ms, cwd: ROOT });
  return {
    ok: run.ok && !!run.payload && run.payload.ok === true,
    payload: run.payload,
    engine_used: 'rust',
    transport: 'cargo_run',
    status: run.status,
    reason: !run.ok ? (run.stderr || run.stdout || `rust_cargo_exit_${run.status}`) : null
  };
}

function listRootEntries(rootPath: string) {
  try {
    return fs.readdirSync(rootPath).map((name: string) => String(name || '')).filter(Boolean);
  } catch {
    return [];
  }
}

function fileExists(rootPath: string, relPath: string) {
  try {
    return fs.existsSync(path.join(rootPath, relPath));
  } catch {
    return false;
  }
}

function assessUiConsistency(rootPath: string, policy: any) {
  const required = Array.isArray(policy.checks.ui_required_files) ? policy.checks.ui_required_files : [];
  const present = required.filter((rel: string) => fileExists(rootPath, rel));
  const score = required.length === 0 ? 100 : Math.round((present.length / required.length) * 100);
  return {
    required_count: required.length,
    present_count: present.length,
    score
  };
}

function assessScientificCompleteness(rootPath: string, policy: any) {
  const required = Array.isArray(policy.checks.scientific_required_files) ? policy.checks.scientific_required_files : [];
  const present = required.filter((rel: string) => fileExists(rootPath, rel));
  const score = required.length === 0 ? 100 : Math.round((present.length / required.length) * 100);
  return {
    required_count: required.length,
    present_count: present.length,
    score
  };
}

function checkBacklogDrift(policy: any) {
  const script = policy.backlog_check.script;
  if (!fs.existsSync(script)) {
    return {
      ok: false,
      finding: normalizeFinding({
        id: 'backlog_check_script_missing',
        category: 'backlog_drift',
        title: 'Backlog drift checker script missing',
        severity: 'medium',
        summary: 'Backlog drift check script is missing from expected path.',
        path: path.relative(ROOT, script).replace(/\\/g, '/'),
        evidence: [`missing_script=${path.relative(ROOT, script).replace(/\\/g, '/')}`],
        safe_autofix: false
      }, 'ts')
    };
  }
  const args = [script, 'check', '--strict=0'];
  const run = runJsonCommand('node', args, { timeout: 120000, cwd: ROOT });
  const payload = run.payload && typeof run.payload === 'object' ? run.payload : null;
  if (!run.ok || !payload) {
    return {
      ok: false,
      finding: normalizeFinding({
        id: 'backlog_check_unavailable',
        category: 'backlog_drift',
        title: 'Backlog drift check unavailable',
        severity: 'medium',
        summary: 'Backlog drift check could not be executed successfully.',
        evidence: [run.stderr || run.stdout || `exit_${run.status}`],
        safe_autofix: false
      }, 'ts')
    };
  }
  if (Number(payload.drift_count || 0) > 0) {
    return {
      ok: true,
      finding: normalizeFinding({
        id: 'backlog_registry_drift_detected',
        category: 'backlog_drift',
        title: 'Backlog generated artifacts drift from source backlog',
        severity: 'high',
        summary: 'Backlog registry/view artifacts are out of sync with source backlog.',
        evidence: [`drift_count=${Number(payload.drift_count || 0)}`],
        safe_autofix: false,
        patch_preview: 'Run: node systems/ops/backlog_registry.js sync'
      }, 'ts'),
      payload
    };
  }
  return { ok: true, finding: null, payload };
}

function checkGitMetadata(rootPath: string, policy: any) {
  const days = clampInt(policy.git_metadata.days_window, 1, 365, 14);
  const sinceArg = `--since=${days}.days`;
  const run = runJsonCommand('git', ['-C', rootPath, 'log', sinceArg, '--pretty=format:%an|%at'], { timeout: 30000, cwd: ROOT });
  if (!run.ok) {
    return [{
      id: 'git_metadata_unavailable',
      category: 'metadata_surface',
      title: 'Git metadata sampling unavailable',
      severity: 'low',
      summary: 'Git metadata could not be sampled for perception leak analysis.',
      evidence: [run.stderr || run.stdout || `exit_${run.status}`],
      safe_autofix: false
    }];
  }
  const lines = String(run.stdout || '').split('\n').map((line) => line.trim()).filter(Boolean);
  const authors = new Set<string>();
  const buckets = new Map<number, number>();
  for (const line of lines) {
    const parts = line.split('|');
    if (parts.length < 2) continue;
    const author = cleanText(parts[0], 120);
    const epoch = Number(parts[1]);
    if (author) authors.add(author);
    if (Number.isFinite(epoch) && epoch > 0) {
      const bucketSizeSec = Math.max(60, clampInt(policy.git_metadata.burst_window_minutes, 1, 120, 10) * 60);
      const bucket = Math.floor(epoch / bucketSizeSec);
      buckets.set(bucket, Number(buckets.get(bucket) || 0) + 1);
    }
  }
  const findings: any[] = [];
  const minCommits = clampInt(policy.git_metadata.min_commits_for_author_concentration_signal, 1, 100000, 20);
  if (lines.length >= minCommits && authors.size <= 1) {
    findings.push({
      id: 'git_author_concentration_signal',
      category: 'metadata_surface',
      title: 'Single-author concentration signal',
      severity: 'medium',
      summary: 'Recent git metadata appears strongly concentrated around one author identity.',
      evidence: [`commit_count=${lines.length}`, `authors=${authors.size}`],
      safe_autofix: false
    });
  }
  const maxBurst = Array.from(buckets.values()).reduce((m, n) => Math.max(m, n), 0);
  if (maxBurst >= clampInt(policy.git_metadata.burst_threshold, 1, 100000, 12)) {
    findings.push({
      id: 'git_burst_pattern_signal',
      category: 'metadata_surface',
      title: 'High burst commit pattern',
      severity: 'low',
      summary: 'Short-window burst commit pattern may indicate unreviewed rapid iteration phases.',
      evidence: [`max_window_commits=${maxBurst}`, `window_minutes=${clampInt(policy.git_metadata.burst_window_minutes, 1, 120, 10)}`],
      safe_autofix: false
    });
  }
  return findings;
}

function buildTsFindings(rootPath: string, policy: any, trigger: string, rustResult: any) {
  const findings: any[] = [];

  if (!rustResult.ok && (policy.engine.mode === 'rust_only' || policy.engine.allow_ts_fallback !== true)) {
    findings.push(normalizeFinding({
      id: 'rust_engine_required_but_failed',
      category: 'engine_health',
      title: 'Rust engine failed and fallback is disabled',
      severity: 'high',
      summary: 'Illusion auditor rust core failed and policy disallows fallback.',
      evidence: [cleanText(rustResult.reason || 'rust_engine_failed', 200)],
      safe_autofix: false
    }, 'ts'));
  }

  const rootEntries = listRootEntries(rootPath);
  const suspicious = new Set((policy.checks.suspicious_root_names || []).map((item: string) => String(item || '').toLowerCase()));
  for (const entry of rootEntries) {
    const lower = String(entry || '').toLowerCase();
    if (!lower || lower === '.git') continue;
    if (suspicious.has(lower) || /^\d+$/.test(lower)) {
      findings.push(normalizeFinding({
        id: `root_name_signal_${lower.replace(/[^a-z0-9_.-]+/g, '_')}`,
        category: 'root_hygiene',
        title: 'Suspicious root entry name',
        severity: 'medium',
        summary: 'Root-level naming pattern may weaken professional repository surface.',
        path: entry,
        evidence: [`entry=${entry}`],
        safe_autofix: false
      }, 'ts'));
    }
  }

  const readmePath = path.join(rootPath, 'README.md');
  if (fs.existsSync(readmePath)) {
    const text = String(fs.readFileSync(readmePath, 'utf8') || '');
    const openclawMentions = (text.match(/openclaw/gi) || []).length;
    if (openclawMentions > 0) {
      findings.push(normalizeFinding({
        id: 'readme_legacy_branding_openclaw',
        category: 'branding_consistency',
        title: 'Legacy branding string in README',
        severity: 'low',
        summary: 'README includes legacy naming pattern.',
        path: 'README.md',
        evidence: [`openclaw_mentions=${openclawMentions}`],
        safe_autofix: true,
        patch_preview: 'Replace legacy branding strings in README with canonical naming.'
      }, 'ts'));
    }
  }

  const ui = assessUiConsistency(rootPath, policy);
  if (ui.score < policy.thresholds.min_ui_score) {
    findings.push(normalizeFinding({
      id: 'ui_surface_consistency_below_threshold',
      category: 'ui_consistency',
      title: 'UI/documentation surface consistency below threshold',
      severity: 'high',
      summary: 'Expected UI and collaboration artifacts are missing or incomplete.',
      evidence: [
        `ui_score=${ui.score}`,
        `required=${ui.required_count}`,
        `present=${ui.present_count}`
      ],
      safe_autofix: false
    }, 'ts'));
  }

  const science = assessScientificCompleteness(rootPath, policy);
  if (science.score < policy.thresholds.min_scientific_score) {
    findings.push(normalizeFinding({
      id: 'scientific_completeness_below_threshold',
      category: 'scientific_integrity',
      title: 'Scientific reasoning surface completeness below threshold',
      severity: 'medium',
      summary: 'Scientific reasoning infrastructure appears incomplete for expected maturity level.',
      evidence: [
        `scientific_score=${science.score}`,
        `required=${science.required_count}`,
        `present=${science.present_count}`
      ],
      safe_autofix: false
    }, 'ts'));
  }

  const backlog = checkBacklogDrift(policy);
  if (backlog.finding) findings.push(backlog.finding);

  for (const row of checkGitMetadata(rootPath, policy)) {
    findings.push(normalizeFinding(row, 'ts'));
  }

  return {
    findings,
    metrics: {
      ui,
      science,
      backlog_ok: backlog.ok,
      backlog_payload: backlog.payload || null
    }
  };
}

function summarizeFindings(findings: any[]) {
  const rows = Array.isArray(findings) ? findings : [];
  const highCount = rows.filter((row) => row.severity === 'high').length;
  const mediumCount = rows.filter((row) => row.severity === 'medium').length;
  const lowCount = rows.filter((row) => row.severity === 'low').length;
  const maxScore = rows.reduce((m, row) => Math.max(m, clampInt(row.score, 0, 100, 0)), 0);
  const avgScore = rows.length
    ? Number((rows.reduce((sum, row) => sum + clampInt(row.score, 0, 100, 0), 0) / rows.length).toFixed(2))
    : 0;
  return {
    finding_count: rows.length,
    high_count: highCount,
    medium_count: mediumCount,
    low_count: lowCount,
    max_score: maxScore,
    average_score: avgScore
  };
}

function safeFixTemplate(relPath: string) {
  const p = String(relPath || '');
  if (p === 'CHANGELOG.md') {
    return '# Changelog\n\n## [Unreleased]\n';
  }
  if (p.endsWith('bug_report.md')) {
    return '## Summary\n\nDescribe the bug.\n';
  }
  if (p.endsWith('feature_request.md')) {
    return '## Problem Statement\n\nDescribe the requested capability.\n';
  }
  if (p.endsWith('security_report.md')) {
    return 'Security reports must follow SECURITY.md private disclosure guidance.\n';
  }
  if (p.endsWith('.md')) {
    const title = path.basename(p).replace(/\.md$/i, '').replace(/[_-]+/g, ' ');
    return `# ${title}\n\nPlaceholder generated by illusion integrity auto-fix.\n`;
  }
  return '';
}

function applySafeFixes(rootPath: string, findings: any[]) {
  const applied: any[] = [];
  const skipped: any[] = [];
  for (const row of findings) {
    const id = String(row && row.id || '');
    if (!row || row.safe_autofix !== true) {
      skipped.push({ id, reason: 'not_safe_autofix' });
      continue;
    }
    if (/^missing_file_/.test(id)) {
      const rel = String(row.path || '').replace(/^\/+/, '');
      if (!rel) {
        skipped.push({ id, reason: 'missing_path' });
        continue;
      }
      const abs = path.join(rootPath, rel);
      if (fs.existsSync(abs)) {
        skipped.push({ id, reason: 'already_exists', path: rel });
        continue;
      }
      fs.mkdirSync(path.dirname(abs), { recursive: true });
      fs.writeFileSync(abs, safeFixTemplate(rel), 'utf8');
      applied.push({ id, action: 'create_file', path: rel });
      continue;
    }
    if (id === 'readme_legacy_branding_openclaw') {
      const readmePath = path.join(rootPath, 'README.md');
      if (!fs.existsSync(readmePath)) {
        skipped.push({ id, reason: 'readme_missing' });
        continue;
      }
      const before = String(fs.readFileSync(readmePath, 'utf8') || '');
      const after = before.replace(/OpenClaw/g, 'Protheus').replace(/openclaw/g, 'protheus');
      if (after === before) {
        skipped.push({ id, reason: 'no_replacements' });
        continue;
      }
      fs.writeFileSync(readmePath, after, 'utf8');
      applied.push({ id, action: 'replace_legacy_branding', path: 'README.md' });
      continue;
    }
    skipped.push({ id, reason: 'no_handler' });
  }
  return { applied, skipped };
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function signReceipt(core: any, policy: any) {
  const secretFromEnv = cleanText(process.env[policy.signing_secret_env] || '', 400);
  const secret = secretFromEnv || cleanText(policy.signing_secret || '', 400);
  const h = crypto.createHmac('sha256', secret || 'illusion_audit_default_secret');
  h.update(JSON.stringify(core));
  return h.digest('hex');
}

function runAudit(args: any, policy: any) {
  const triggerRaw = normalizeToken(args.trigger || 'manual', 40).toLowerCase() || 'manual';
  const trigger = ['manual', 'startup', 'promotion'].includes(triggerRaw) ? triggerRaw : 'manual';
  const triggerCfg = policy.triggers[trigger] || policy.triggers.manual;
  if (triggerCfg.enabled !== true) {
    return {
      ok: true,
      skipped: true,
      type: 'illusion_integrity_audit',
      trigger,
      reason: 'trigger_disabled'
    };
  }

  const rootPath = args.root ? resolvePath(args.root, '.') : ROOT;
  const strict = toBool(args.strict, triggerCfg.strict || policy.strict_default);
  const applyRequested = toBool(args.apply, false);
  const approvalNote = cleanText(args['approval-note'] || args.approval_note || '', 500);
  const consentToken = cleanText(args['consent-token'] || args.consent_token || '', 200);
  const consentRequired = policy.autofix.require_human_consent === true;
  const consentSatisfied = !consentRequired
    || (
      approvalNote.length >= policy.autofix.required_approval_min_len
      && consentToken.startsWith(policy.autofix.required_token_prefix)
    );

  const rustResult = runRustScan(policy, rootPath, trigger, args);
  const rustFindings = Array.isArray(rustResult.payload && rustResult.payload.findings)
    ? rustResult.payload.findings.map((row: any) => normalizeFinding(row, 'rust'))
    : [];

  const tsResult = buildTsFindings(rootPath, policy, trigger, rustResult);
  const findings = [...rustFindings, ...tsResult.findings];
  const summary = summarizeFindings(findings);
  const shouldFail = (
    summary.max_score >= policy.thresholds.fail_score
    || summary.high_count > policy.thresholds.max_high_findings_before_fail
  );

  let autofix = {
    requested: applyRequested,
    allowed_by_policy: policy.autofix.allow_apply === true,
    consent_required: consentRequired,
    consent_satisfied: consentSatisfied,
    applied: [] as any[],
    skipped: [] as any[],
    applied_count: 0
  };
  if (applyRequested) {
    if (policy.autofix.allow_apply !== true) {
      autofix.skipped.push({ reason: 'autofix_disabled_by_policy' });
    } else if (!consentSatisfied) {
      autofix.skipped.push({
        reason: 'human_consent_missing',
        required_approval_min_len: policy.autofix.required_approval_min_len,
        required_token_prefix: policy.autofix.required_token_prefix
      });
    } else {
      const applied = applySafeFixes(rootPath, findings);
      autofix = {
        ...autofix,
        applied: applied.applied,
        skipped: [...autofix.skipped, ...applied.skipped],
        applied_count: applied.applied.length
      };
    }
  }

  const runId = `ill_audit_${Date.now().toString(36)}_${process.pid}`;
  ensureDir(policy.paths.reports_dir);
  ensureDir(policy.paths.patches_dir);
  const reportPath = path.join(policy.paths.reports_dir, `${runId}.json`);
  const patchPath = path.join(policy.paths.patches_dir, `${runId}.md`);
  const patchLines = [
    `# Illusion Integrity Suggested Fixes`,
    '',
    `run_id: ${runId}`,
    `trigger: ${trigger}`,
    `generated_at: ${nowIso()}`,
    ''
  ];
  for (const row of findings) {
    if (!row.patch_preview) continue;
    patchLines.push(`- [${row.severity}] ${row.id}: ${row.patch_preview}`);
  }
  fs.writeFileSync(patchPath, `${patchLines.join('\n')}\n`, 'utf8');

  const receiptCore = {
    schema_id: 'illusion_integrity_audit_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ts: nowIso(),
    ok: shouldFail ? false : true,
    lane_id: policy.lane_id,
    type: 'illusion_integrity_audit',
    trigger,
    strict,
    run_id: runId,
    engine: {
      mode: policy.engine.mode,
      rust_ok: rustResult.ok === true,
      rust_transport: rustResult.transport || null,
      rust_reason: rustResult.reason || null
    },
    summary,
    thresholds: policy.thresholds,
    findings,
    metrics: {
      ui: tsResult.metrics.ui,
      scientific: tsResult.metrics.science,
      backlog_ok: tsResult.metrics.backlog_ok,
      backlog_payload: tsResult.metrics.backlog_payload,
      rust_metrics: rustResult.payload && rustResult.payload.metrics ? rustResult.payload.metrics : null
    },
    scientific_reasoning: {
      model: 'scientific_reasoning_v1',
      hypothesis: 'professional_surface_leaks_exist',
      evidence_count: findings.length,
      posterior_risk: Number((summary.max_score / 100).toFixed(4)),
      fail_condition: shouldFail
    },
    mind_sovereignty: {
      policy_anchor: 'docs/MIND_SOVEREIGNTY.md',
      checked: fs.existsSync(path.join(rootPath, 'docs', 'MIND_SOVEREIGNTY.md')),
      fail_closed_on_high_risk: true
    },
    autofix,
    report_path: path.relative(ROOT, reportPath).replace(/\\/g, '/'),
    patch_path: path.relative(ROOT, patchPath).replace(/\\/g, '/')
  };
  const signature = signReceipt(receiptCore, policy);
  const receipt = {
    ...receiptCore,
    signature
  };
  writeJsonAtomic(reportPath, receipt);
  writeArtifactSet(
    {
      latestPath: policy.paths.latest_path,
      receiptsPath: policy.paths.receipts_path,
      historyPath: policy.paths.history_path
    },
    receipt,
    {
      schemaId: 'illusion_integrity_audit_receipt',
      schemaVersion: '1.0',
      artifactType: 'receipt',
      writeLatest: true,
      appendReceipt: true
    }
  );
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'illusion_integrity_audit_state',
    schema_version: '1.0',
    lane_id: policy.lane_id,
    run_count: Number(readJson(policy.paths.state_path, { run_count: 0 }).run_count || 0) + 1,
    last_run_id: runId,
    last_ok: receipt.ok === true,
    last_trigger: trigger,
    last_ts: receipt.ts
  });
  if (!receipt.ok && strict) {
    return {
      ...receipt,
      strict_block: true
    };
  }
  return receipt;
}

function status(policy: any) {
  const latest = readJson(policy.paths.latest_path, null);
  const state = readJson(policy.paths.state_path, null);
  if (!latest) {
    return {
      ok: false,
      available: false,
      type: 'illusion_integrity_audit_status',
      lane_id: policy.lane_id,
      latest_path: path.relative(ROOT, policy.paths.latest_path).replace(/\\/g, '/')
    };
  }
  return {
    ok: true,
    available: true,
    type: 'illusion_integrity_audit_status',
    lane_id: policy.lane_id,
    latest,
    state
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'run', 80) || 'run';
  if (args.help || cmd === 'help') {
    usage();
    emit({ ok: true, type: 'illusion_integrity_audit', action: 'help', ts: nowIso() }, 0);
  }
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = normalizePolicy(policyPath);
  if (policy.enabled !== true) {
    emit({
      ok: false,
      type: 'illusion_integrity_audit',
      action: cmd,
      error: 'lane_disabled',
      policy_path: path.relative(ROOT, policy.policy_path).replace(/\\/g, '/')
    }, 2);
  }
  if (cmd === 'status') {
    const out = status(policy);
    emit(out, out.ok ? 0 : 2);
  }
  if (cmd === 'run' || cmd === 'audit') {
    const out = runAudit(args, policy);
    const strict = toBool(args.strict, policy.strict_default);
    const fail = out && out.ok !== true && (strict || out.strict_block === true);
    emit(out, fail ? 2 : 0);
  }
  usage();
  process.exit(1);
}

main();

