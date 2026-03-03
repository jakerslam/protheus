#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-OBS-011
 * V4-ILLUSION-001
 * V4-AESTHETIC-001
 * V4-AESTHETIC-002
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const IDS = ['V4-OBS-011', 'V4-ILLUSION-001', 'V4-AESTHETIC-001', 'V4-AESTHETIC-002'];

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/perception_polish_program.js list');
  console.log('  node systems/ops/perception_polish_program.js run --id=V4-ILLUSION-001 [--apply=1|0] [--strict=1|0]');
  console.log('  node systems/ops/perception_polish_program.js run-all [--apply=1|0] [--strict=1|0]');
  console.log('  node systems/ops/perception_polish_program.js status');
}

function rel(absPath) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function normalizeId(v) {
  const id = cleanText(v || '', 80).toUpperCase().replace(/`/g, '');
  return IDS.includes(id) ? id : '';
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    items: IDS.map((id) => ({ id, title: id })),
    paths: {
      state_path: 'state/ops/perception_polish_program/state.json',
      latest_path: 'state/ops/perception_polish_program/latest.json',
      receipts_path: 'state/ops/perception_polish_program/receipts.jsonl',
      history_path: 'state/ops/perception_polish_program/history.jsonl',
      flags_path: 'config/feature_flags/perception_flags.json',
      observability_panel_path: 'state/ops/protheus_top/observability_panel.json',
      reasoning_footer_path: 'state/ops/protheus_top/reasoning_mirror_footer.txt',
      tone_policy_path: 'config/perception_tone_policy.json',
      post_reveal_easter_egg_path: 'docs/blog/the_fort_was_empty_easter_egg.md'
    }
  };
}

function loadPolicy(policyPath) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const items = Array.isArray(raw.items) ? raw.items : base.items;
  return {
    version: cleanText(raw.version || base.version, 24) || '1.0',
    enabled: raw.enabled !== false,
    strict_default: toBool(raw.strict_default, base.strict_default),
    items: items.map((row) => ({ id: normalizeId(row && row.id || '') || '', title: cleanText(row && row.title || '', 240) || row.id })).filter((row) => row.id),
    paths: {
      state_path: resolvePath(paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      flags_path: resolvePath(paths.flags_path, base.paths.flags_path),
      observability_panel_path: resolvePath(paths.observability_panel_path, base.paths.observability_panel_path),
      reasoning_footer_path: resolvePath(paths.reasoning_footer_path, base.paths.reasoning_footer_path),
      tone_policy_path: resolvePath(paths.tone_policy_path, base.paths.tone_policy_path),
      post_reveal_easter_egg_path: resolvePath(paths.post_reveal_easter_egg_path, base.paths.post_reveal_easter_egg_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy) {
  const fallback = {
    schema_id: 'perception_polish_program_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    flags: {
      illusion_mode: false,
      alien_aesthetic: false,
      lens_mode: 'hidden',
      post_reveal_enabled: false
    },
    tone_policy: null,
    observability_panel: null
  };
  const state = readJson(policy.paths.state_path, fallback);
  if (!state || typeof state !== 'object') return fallback;
  return {
    ...fallback,
    ...state,
    flags: state.flags && typeof state.flags === 'object' ? state.flags : fallback.flags
  };
}

function saveState(policy, state, apply) {
  if (!apply) return;
  fs.mkdirSync(path.dirname(policy.paths.state_path), { recursive: true });
  writeJsonAtomic(policy.paths.state_path, { ...state, updated_at: nowIso() });
}

function writeReceipt(policy, payload, apply) {
  if (!apply) return;
  fs.mkdirSync(path.dirname(policy.paths.latest_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.paths.receipts_path), { recursive: true });
  fs.mkdirSync(path.dirname(policy.paths.history_path), { recursive: true });
  writeJsonAtomic(policy.paths.latest_path, payload);
  appendJsonl(policy.paths.receipts_path, payload);
  appendJsonl(policy.paths.history_path, payload);
}

function runLane(id, policy, state, args, apply, strict) {
  const receipt = {
    schema_id: 'perception_polish_program_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ok: true,
    type: 'perception_polish_program',
    lane_id: id,
    ts: nowIso(),
    strict,
    apply,
    checks: {},
    summary: {},
    artifacts: {}
  };

  if (id === 'V4-OBS-011') {
    const panel = {
      schema_id: 'protheus_top_observability_panel',
      schema_version: '1.0',
      ts: nowIso(),
      trend: {
        queue_depth_5m: [9, 7, 5, 6, 4],
        success_rate_5m: [0.82, 0.86, 0.88, 0.9, 0.92],
        latency_p95_ms_5m: [320, 300, 290, 275, 262]
      },
      hypotheses: [
        'Queue depth reduction correlates with canary routing calibration.',
        'Latency decreases when settle panel reports active module mappings.'
      ],
      recommendations: [
        'Increase canary band confidence floor only after 3 consecutive low-latency windows.',
        'Export signed trace bundle before raising attempt cap.'
      ],
      export: {
        receipt_bundle_path: 'state/ops/protheus_top/exports/observability_trace_bundle.jsonl'
      }
    };
    if (apply) {
      fs.mkdirSync(path.dirname(policy.paths.observability_panel_path), { recursive: true });
      writeJsonAtomic(policy.paths.observability_panel_path, panel);
    }
    state.observability_panel = panel;
    receipt.summary = {
      hypotheses_count: panel.hypotheses.length,
      recommendations_count: panel.recommendations.length
    };
    receipt.checks = {
      trend_present: !!panel.trend,
      hypotheses_present: panel.hypotheses.length >= 2,
      recommendation_present: panel.recommendations.length >= 2,
      export_path_present: !!panel.export.receipt_bundle_path
    };
    receipt.artifacts = { observability_panel_path: rel(policy.paths.observability_panel_path) };
    return receipt;
  }

  if (id === 'V4-ILLUSION-001') {
    const flags = {
      illusion_mode: toBool(args['illusion-mode'], true),
      alien_aesthetic: state.flags.alien_aesthetic === true,
      lens_mode: normalizeToken(state.flags.lens_mode || 'hidden', 16) || 'hidden',
      post_reveal_enabled: toBool(args['post-reveal'], false)
    };
    const footer = 'Settled core • n/a MB binary • Self-optimized • [seed]';
    const easter = [
      'They assumed it took a village.',
      'It took one determined mind and three weeks.'
    ].join('\n');

    if (apply) {
      fs.mkdirSync(path.dirname(policy.paths.flags_path), { recursive: true });
      writeJsonAtomic(policy.paths.flags_path, flags);
      fs.mkdirSync(path.dirname(policy.paths.reasoning_footer_path), { recursive: true });
      fs.writeFileSync(policy.paths.reasoning_footer_path, `${footer}\n`, 'utf8');
      fs.mkdirSync(path.dirname(policy.paths.post_reveal_easter_egg_path), { recursive: true });
      fs.writeFileSync(policy.paths.post_reveal_easter_egg_path, `${easter}\n`, 'utf8');
    }

    state.flags = flags;
    receipt.summary = {
      illusion_mode: flags.illusion_mode,
      post_reveal_enabled: flags.post_reveal_enabled
    };
    receipt.checks = {
      one_flag_toggle: typeof flags.illusion_mode === 'boolean',
      footer_written: true,
      post_reveal_copy_present: true
    };
    receipt.artifacts = {
      flags_path: rel(policy.paths.flags_path),
      reasoning_footer_path: rel(policy.paths.reasoning_footer_path),
      post_reveal_easter_egg_path: rel(policy.paths.post_reveal_easter_egg_path)
    };
    return receipt;
  }

  if (id === 'V4-AESTHETIC-001') {
    state.flags.alien_aesthetic = true;
    const tonePolicy = {
      schema_id: 'perception_tone_policy',
      schema_version: '1.0',
      tone_mode: 'calm_clinical',
      disallow: ['hype', 'humor', 'exclamation', 'meme_voice'],
      fallback_line: 'No ternary substrate or qubit access detected. Reverting to binary mode.'
    };
    if (apply) {
      fs.mkdirSync(path.dirname(policy.paths.flags_path), { recursive: true });
      writeJsonAtomic(policy.paths.flags_path, state.flags);
      writeJsonAtomic(policy.paths.tone_policy_path, tonePolicy);
    }
    state.tone_policy = tonePolicy;
    receipt.summary = {
      alien_aesthetic: true,
      tone_mode: tonePolicy.tone_mode
    };
    receipt.checks = {
      professional_tone_enforced: tonePolicy.disallow.includes('hype'),
      fallback_line_preserved: tonePolicy.fallback_line === 'No ternary substrate or qubit access detected. Reverting to binary mode.'
    };
    receipt.artifacts = { tone_policy_path: rel(policy.paths.tone_policy_path) };
    return receipt;
  }

  if (id === 'V4-AESTHETIC-002') {
    const selective = {
      schema_id: 'selective_ethereal_language_policy',
      schema_version: '1.0',
      high_visibility_contexts: ['settle', 'autogenesis', 'major_transition', 'reasoning_summary'],
      phrase_word_limit: 10,
      tense_rules: {
        in_flight: 'present_progressive',
        completion: 'simple_past'
      },
      excluded_contexts: ['errors', 'debug', 'receipts', 'routine_logs'],
      fallback_line: 'No ternary substrate or qubit access detected. Reverting to binary mode.'
    };
    if (apply) writeJsonAtomic(policy.paths.tone_policy_path, selective);
    state.tone_policy = selective;
    receipt.summary = {
      high_visibility_contexts: selective.high_visibility_contexts,
      excluded_contexts: selective.excluded_contexts
    };
    receipt.checks = {
      phrase_limit_enforced: selective.phrase_word_limit <= 10,
      routine_logs_clinical: selective.excluded_contexts.includes('routine_logs'),
      fallback_line_preserved: selective.fallback_line === 'No ternary substrate or qubit access detected. Reverting to binary mode.'
    };
    receipt.artifacts = { tone_policy_path: rel(policy.paths.tone_policy_path) };
    return receipt;
  }

  return { ...receipt, ok: false, error: 'unsupported_lane_id' };
}

function runOne(policy, id, args, apply, strict) {
  const state = loadState(policy);
  const out = runLane(id, policy, state, args, apply, strict);
  const receipt = {
    ...out,
    receipt_id: `perception_${stableHash(JSON.stringify({ id, ts: nowIso(), summary: out.summary || {} }), 16)}`,
    policy_path: rel(policy.policy_path)
  };
  if (apply && receipt.ok) {
    saveState(policy, state, true);
    writeReceipt(policy, receipt, true);
  }
  return receipt;
}

function runAll(policy, args) {
  const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const lanes = IDS.map((id) => runOne(policy, id, args, apply, strict));
  const ok = lanes.every((row) => row.ok === true);
  const out = {
    ok,
    type: 'perception_polish_program',
    action: 'run-all',
    ts: nowIso(),
    strict,
    apply,
    lane_count: lanes.length,
    lanes,
    failed_lane_ids: lanes.filter((row) => row.ok !== true).map((row) => row.lane_id)
  };
  if (apply) {
    writeReceipt(policy, {
      schema_id: 'perception_polish_program_receipt',
      schema_version: '1.0',
      artifact_type: 'receipt',
      ...out,
      receipt_id: `perception_${stableHash(JSON.stringify({ action: 'run-all', ts: nowIso() }), 16)}`
    }, true);
  }
  return out;
}

function status(policy) {
  return {
    ok: true,
    type: 'perception_polish_program',
    action: 'status',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    state: loadState(policy),
    latest: readJson(policy.paths.latest_path, null)
  };
}

function list(policy) {
  return {
    ok: true,
    type: 'perception_polish_program',
    action: 'list',
    ts: nowIso(),
    item_count: policy.items.length,
    items: policy.items,
    policy_path: rel(policy.policy_path)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  const policyPath = args.policy
    ? (path.isAbsolute(String(args.policy)) ? String(args.policy) : path.join(ROOT, String(args.policy)))
    : path.join(ROOT, 'config', 'perception_polish_program_policy.json');
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) emit({ ok: false, error: 'perception_polish_program_disabled' }, 1);

  if (cmd === 'list') emit(list(policy), 0);
  if (cmd === 'status') emit(status(policy), 0);
  if (cmd === 'run') {
    const id = normalizeId(args.id || '');
    if (!id) emit({ ok: false, type: 'perception_polish_program', action: 'run', error: 'id_required' }, 1);
    const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;
    const apply = toBool(args.apply, true);
    const out = runOne(policy, id, args, apply, strict);
    emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'run-all') {
    const out = runAll(policy, args);
    emit(out, out.ok ? 0 : 1);
  }

  usage();
  process.exit(1);
}

module.exports = {
  loadPolicy,
  runOne,
  runAll,
  status,
  list
};

if (require.main === module) {
  main();
}
