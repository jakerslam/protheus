#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-027
 * Project Jigsaw AttackCinema Replay Theater
 */

const path = require('path');
const { ROOT } = require('../../../lib/queued_backlog_runtime');
const { runLaneCli } = require('../../../lib/backlog_lane_cli');

const POLICY_PATH = process.env.PROJECT_JIGSAW_ATTACKCINEMA_REPLAY_POLICY_PATH
  ? path.resolve(process.env.PROJECT_JIGSAW_ATTACKCINEMA_REPLAY_POLICY_PATH)
  : path.join(ROOT, 'config/project_jigsaw_attackcinema_policy.json');

runLaneCli({
  lane_id: 'V3-RACE-DEF-027',
  title: 'Project Jigsaw AttackCinema Replay Theater',
  type: 'project_jigsaw_attackcinema_replay',
  default_action: 'capture',
  script_label: 'systems/security/jigsaw/attackcinema_replay_theater.js',
  policy_path: POLICY_PATH,
  default_policy: {
    version: '1.0',
    enabled: true,
    strict_default: true,
    checks: [
    {
        "id": "recording_engine_live",
        "description": "Recorder lane captures security timelines",
        "file_must_exist": "systems/security/jigsaw/README.md"
    },
    {
        "id": "highlight_editor_lane",
        "description": "Deterministic highlight generation configured"
    },
    {
        "id": "clearance4_playback_gate",
        "description": "Clearance-4 playback policy enforced"
    },
    {
        "id": "encrypted_capture_storage",
        "description": "Encrypted capture-at-rest contract active"
    }
],
    paths: {
      state_path: 'state/security/project_jigsaw_attackcinema_replay/state.json',
      latest_path: 'state/security/project_jigsaw_attackcinema_replay/latest.json',
      receipts_path: 'state/security/project_jigsaw_attackcinema_replay/receipts.jsonl',
      history_path: 'state/security/project_jigsaw_attackcinema_replay/history.jsonl'
    }
  }
});
