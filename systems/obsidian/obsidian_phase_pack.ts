#!/usr/bin/env node
'use strict';
export {};

/**
 * OBS-005..012 implementation pack.
 * Extends Obsidian integration with phase 1/2/3 features under policy-root control.
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
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const DEFAULT_POLICY_PATH = process.env.OBSIDIAN_PHASE_PACK_POLICY_PATH
  ? path.resolve(process.env.OBSIDIAN_PHASE_PACK_POLICY_PATH)
  : path.join(ROOT, 'config', 'obsidian_phase_pack_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/obsidian/obsidian_phase_pack.js wisdom-project --title=<title> --principle=<text> --holo-node-id=<id> [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js ops-card --kind=doctor|execution --status=ok|warn|fail [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js intent-compile --note-path=<abs-path> [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js canvas-map --canvas-id=<id> --nodes=a,b,c [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js identity-sync --entity-id=<id> --note=<path> --holo-node-id=<id> [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js plugin-control --action=status|queue|approve|veto [--pending-id=<id>] [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js phone-mode --segments=wisdom,cards,intents [--apply=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js resilience-check [--strict=0|1]');
  console.log('  node systems/obsidian/obsidian_phase_pack.js status');
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function writeTextAtomic(filePath, text) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, String(text), 'utf8');
  fs.renameSync(tmp, filePath);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    phone_seed_segment_limit: 128,
    paths: {
      vault_root: 'memory',
      wisdom_root: 'state/obsidian/projections/wisdom',
      cards_root: 'state/obsidian/projections/cards',
      canvas_root: 'state/obsidian/projections/canvas',
      intents_root: 'state/obsidian/intents',
      mobile_root: 'state/obsidian/mobile',
      identity_bus_path: 'state/obsidian/identity_bus.json',
      plugin_state_path: 'state/obsidian/plugin_state.json',
      latest_path: 'state/obsidian/phase_pack_latest.json',
      receipts_path: 'state/obsidian/phase_pack_receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    phone_seed_segment_limit: clampInt(raw.phone_seed_segment_limit, 16, 5000, base.phone_seed_segment_limit),
    paths: {
      vault_root: resolvePath(paths.vault_root, base.paths.vault_root),
      wisdom_root: resolvePath(paths.wisdom_root, base.paths.wisdom_root),
      cards_root: resolvePath(paths.cards_root, base.paths.cards_root),
      canvas_root: resolvePath(paths.canvas_root, base.paths.canvas_root),
      intents_root: resolvePath(paths.intents_root, base.paths.intents_root),
      mobile_root: resolvePath(paths.mobile_root, base.paths.mobile_root),
      identity_bus_path: resolvePath(paths.identity_bus_path, base.paths.identity_bus_path),
      plugin_state_path: resolvePath(paths.plugin_state_path, base.paths.plugin_state_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    }
  };
}

function receipt(policy, row) {
  const payload = {
    ts: nowIso(),
    ok: true,
    shadow_only: policy.shadow_only,
    ...row
  };
  writeJsonAtomic(policy.paths.latest_path, payload);
  appendJsonl(policy.paths.receipts_path, payload);
  return payload;
}

function wisdomProject(args, policy) {
  const apply = toBool(args.apply, false);
  const title = cleanText(args.title || 'Untitled Wisdom', 160);
  const principle = cleanText(args.principle || '', 3000);
  const holoNodeId = normalizeToken(args['holo-node-id'] || args.holo_node_id || '', 120) || 'unknown';
  const provenance = cleanText(args.provenance || 'holo_viz', 240);
  const slug = normalizeToken(title, 80) || `wisdom_${stableHash(title, 8)}`;
  const filePath = path.join(policy.paths.wisdom_root, `${slug}.md`);
  const text = [
    '---',
    `title: ${title}`,
    `holo_node_id: ${holoNodeId}`,
    `provenance: ${provenance}`,
    `generated_at: ${nowIso()}`,
    '---',
    '',
    `# ${title}`,
    '',
    principle,
    '',
    `- Source Node: [[${holoNodeId}]]`,
    `- Receipt Anchor: ${stableHash(`${title}|${holoNodeId}|${provenance}`, 24)}`,
    ''
  ].join('\n');
  if (apply) writeTextAtomic(filePath, text);
  return receipt(policy, {
    type: 'obs_wisdom_projector',
    apply,
    title,
    file_path: filePath,
    holo_node_id: holoNodeId
  });
}

function opsCard(args, policy) {
  const apply = toBool(args.apply, false);
  const kind = normalizeToken(args.kind || 'execution', 40) || 'execution';
  const status = normalizeToken(args.status || 'ok', 20) || 'ok';
  const summary = cleanText(args.summary || `${kind} status update`, 500);
  const filePath = path.join(policy.paths.cards_root, `${kind}_${Date.now()}.md`);
  const body = [
    '---',
    `kind: ${kind}`,
    `status: ${status}`,
    `ts: ${nowIso()}`,
    '---',
    '',
    `# ${kind.toUpperCase()} Card`,
    '',
    `Status: **${status}**`,
    '',
    summary,
    ''
  ].join('\n');
  if (apply) writeTextAtomic(filePath, body);
  return receipt(policy, {
    type: 'obs_ops_visibility_card',
    apply,
    kind,
    status,
    file_path: filePath
  });
}

function compileChecklist(content) {
  return String(content || '')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^-\s*\[\s\]/.test(line))
    .map((line, idx) => ({
      id: `step_${idx + 1}`,
      text: cleanText(line.replace(/^-\s*\[\s\]\s*/, ''), 240)
    }))
    .filter((row) => row.text);
}

function intentCompile(args, policy) {
  const apply = toBool(args.apply, false);
  const notePath = args['note-path'] ? path.resolve(String(args['note-path'])) : (args.note_path ? path.resolve(String(args.note_path)) : '');
  if (!notePath) return { ok: false, error: 'note_path_required' };
  if (!fs.existsSync(notePath)) return { ok: false, error: 'note_not_found' };
  const content = fs.readFileSync(notePath, 'utf8');
  const steps = compileChecklist(content);
  const proposalId = `obs_intent_${stableHash(`${notePath}|${Date.now()}`, 18)}`;
  const proposal = {
    schema_id: 'obsidian_intent_proposal',
    schema_version: '1.0',
    proposal_id: proposalId,
    source_note: notePath,
    created_at: nowIso(),
    mode: 'shadow',
    steps,
    confidence: Number((steps.length ? Math.min(0.95, 0.4 + steps.length * 0.1) : 0.2).toFixed(4))
  };
  if (apply) writeJsonAtomic(path.join(policy.paths.intents_root, `${proposalId}.json`), proposal);
  return receipt(policy, {
    type: 'obs_intent_compiler',
    apply,
    proposal_id: proposalId,
    step_count: steps.length,
    source_note: notePath,
    mode: 'shadow'
  });
}

function canvasMap(args, policy) {
  const apply = toBool(args.apply, false);
  const canvasId = normalizeToken(args['canvas-id'] || args.canvas_id || '', 120);
  if (!canvasId) return { ok: false, error: 'canvas_id_required' };
  const nodes = String(args.nodes || '')
    .split(',')
    .map((v) => cleanText(v, 80))
    .filter(Boolean)
    .slice(0, 50);
  const canvas = {
    nodes: nodes.map((label, idx) => ({
      id: `n${idx + 1}`,
      type: 'text',
      text: label,
      x: idx * 260,
      y: 0,
      width: 220,
      height: 80
    })),
    edges: nodes.length > 1
      ? nodes.slice(1).map((_, idx) => ({
          id: `e${idx + 1}`,
          fromNode: `n${idx + 1}`,
          toNode: `n${idx + 2}`
        }))
      : []
  };
  const canvasPath = path.join(policy.paths.canvas_root, `${canvasId}.canvas`);
  if (apply) writeTextAtomic(canvasPath, `${JSON.stringify(canvas, null, 2)}\n`);
  return receipt(policy, {
    type: 'obs_canvas_intelligence',
    apply,
    canvas_id: canvasId,
    node_count: canvas.nodes.length,
    edge_count: canvas.edges.length,
    canvas_path: canvasPath
  });
}

function identitySync(args, policy) {
  const apply = toBool(args.apply, false);
  const entityId = normalizeToken(args['entity-id'] || args.entity_id || '', 120);
  const note = cleanText(args.note || '', 320);
  const holoNodeId = normalizeToken(args['holo-node-id'] || args.holo_node_id || '', 120);
  if (!entityId || !note || !holoNodeId) return { ok: false, error: 'entity_note_holo_required' };

  const bus = readJson(policy.paths.identity_bus_path, { links: {} });
  const links = bus.links && typeof bus.links === 'object' ? bus.links : {};
  links[entityId] = {
    entity_id: entityId,
    note,
    holo_node_id: holoNodeId,
    updated_at: nowIso()
  };
  if (apply) writeJsonAtomic(policy.paths.identity_bus_path, { links });

  return receipt(policy, {
    type: 'obs_cross_view_identity_bus',
    apply,
    entity_id: entityId,
    note,
    holo_node_id: holoNodeId
  });
}

function pluginControl(args, policy) {
  const apply = toBool(args.apply, false);
  const action = normalizeToken(args.action || 'status', 40) || 'status';
  const pendingId = normalizeToken(args['pending-id'] || args.pending_id || '', 120);
  const db = readJson(policy.paths.plugin_state_path, {
    pending: [],
    approved: [],
    vetoed: []
  });

  db.pending = Array.isArray(db.pending) ? db.pending : [];
  db.approved = Array.isArray(db.approved) ? db.approved : [];
  db.vetoed = Array.isArray(db.vetoed) ? db.vetoed : [];

  if (action === 'queue') {
    const id = pendingId || `pending_${stableHash(`${Date.now()}|queue`, 12)}`;
    if (!db.pending.includes(id)) db.pending.push(id);
  }
  if (action === 'approve' && pendingId) {
    db.pending = db.pending.filter((id) => id !== pendingId);
    if (!db.approved.includes(pendingId)) db.approved.push(pendingId);
  }
  if (action === 'veto' && pendingId) {
    db.pending = db.pending.filter((id) => id !== pendingId);
    if (!db.vetoed.includes(pendingId)) db.vetoed.push(pendingId);
  }

  if (apply) writeJsonAtomic(policy.paths.plugin_state_path, db);

  return receipt(policy, {
    type: 'obs_plugin_control_surface',
    apply,
    action,
    pending_count: db.pending.length,
    approved_count: db.approved.length,
    vetoed_count: db.vetoed.length
  });
}

function phoneMode(args, policy) {
  const apply = toBool(args.apply, false);
  const segments = String(args.segments || 'wisdom,cards,intents')
    .split(',')
    .map((v) => normalizeToken(v, 40))
    .filter(Boolean)
    .slice(0, policy.phone_seed_segment_limit);

  const bundle = {
    schema_id: 'obsidian_phone_seed_bundle',
    schema_version: '1.0',
    generated_at: nowIso(),
    segments,
    bounded: true,
    max_segments: policy.phone_seed_segment_limit
  };
  const bundlePath = path.join(policy.paths.mobile_root, 'phone_seed_bundle.json');
  if (apply) writeJsonAtomic(bundlePath, bundle);

  return receipt(policy, {
    type: 'obs_phone_seed_mode',
    apply,
    segments,
    bundle_path: bundlePath,
    bounded: true
  });
}

function resilienceCheck(args, policy) {
  const strict = toBool(args.strict, false);
  const checks = {
    identity_bus_exists: fs.existsSync(policy.paths.identity_bus_path),
    plugin_state_exists: fs.existsSync(policy.paths.plugin_state_path),
    receipts_exist: fs.existsSync(policy.paths.receipts_path)
  };

  const pass = Object.values(checks).every(Boolean);
  const out = receipt(policy, {
    type: 'obs_projection_resilience_check',
    strict,
    checks,
    pass
  });
  if (strict && !pass) return { ...out, exit_code: 1 };
  return { ...out, exit_code: 0 };
}

function status(policy) {
  return {
    ok: true,
    type: 'obsidian_phase_pack_status',
    shadow_only: policy.shadow_only,
    latest: readJson(policy.paths.latest_path, {}),
    identity_links: (() => {
      const db = readJson(policy.paths.identity_bus_path, { links: {} });
      return db.links && typeof db.links === 'object' ? Object.keys(db.links).length : 0;
    })()
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }

  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) emit({ ok: false, error: 'obsidian_phase_pack_disabled' }, 1);

  if (cmd === 'wisdom-project') emit(wisdomProject(args, policy));
  if (cmd === 'ops-card') emit(opsCard(args, policy));
  if (cmd === 'intent-compile') {
    const out = intentCompile(args, policy);
    emit(out, out.ok === false ? 1 : 0);
  }
  if (cmd === 'canvas-map') {
    const out = canvasMap(args, policy);
    emit(out, out.ok === false ? 1 : 0);
  }
  if (cmd === 'identity-sync') {
    const out = identitySync(args, policy);
    emit(out, out.ok === false ? 1 : 0);
  }
  if (cmd === 'plugin-control') emit(pluginControl(args, policy));
  if (cmd === 'phone-mode') emit(phoneMode(args, policy));
  if (cmd === 'resilience-check') {
    const out = resilienceCheck(args, policy);
    emit(out, out.exit_code || 0);
  }
  if (cmd === 'status') emit(status(policy));

  emit({ ok: false, error: 'unknown_command', cmd }, 2);
}

if (require.main === module) {
  main();
}
