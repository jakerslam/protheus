// Canonical Shell helper source: dashboard boot progress and connection indicator projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringBootProgressClamped(rawPercent) {
  var next = Number(rawPercent);
  if (!Number.isFinite(next)) next = 0;
  return Math.max(0, Math.min(100, Math.round(next)));
}

function infringResetBootProgress(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.bootProgressPercent = 6;
  target.bootProgressEvent = 'splash_visible';
  target._bootProgressUpdatedAt = Date.now();
}

function infringBootProgressFromBootStage(rawStage) {
  var stage = String(rawStage || '').trim().toLowerCase();
  if (!stage) return 38;
  if (
    stage === 'ready' ||
    stage === 'connected' ||
    stage === 'boot_complete' ||
    stage === 'runtime_ready'
  ) {
    return 70;
  }
  if (stage.indexOf('agent') >= 0) return 66;
  if (stage.indexOf('connect') >= 0) return 28;
  var isRecoveringStage = stage.indexOf('retry') >= 0;
  if (isRecoveringStage) return 24;
  if (stage.indexOf('unreachable') >= 0 || stage.indexOf('disconnected') >= 0) return 20;
  if (stage.indexOf('start') >= 0 || stage.indexOf('init') >= 0 || stage.indexOf('boot') >= 0) return 16;
  return 42;
}

function infringSetBootProgressPercent(page, rawPercent, opts) {
  var target = page && typeof page === 'object' ? page : {};
  var options = opts && typeof opts === 'object' ? opts : {};
  var next = infringBootProgressClamped(rawPercent);
  var current = infringBootProgressClamped(target.bootProgressPercent);
  var allowDecrease = options.allowDecrease === true;
  if (!allowDecrease && next < current) next = current;
  if (next === current) return;
  target.bootProgressPercent = next;
  target._bootProgressUpdatedAt = Date.now();
}

function infringSetBootProgressEvent(page, eventName, meta) {
  var target = page && typeof page === 'object' ? page : {};
  var event = String(eventName || '').trim().toLowerCase();
  if (!event) return;
  var targetPercent = 0;
  if (event === 'splash_visible') targetPercent = 6;
  else if (event === 'status_requesting') targetPercent = 18;
  else if (event === 'status_connected') targetPercent = 42;
  else if (event === 'status_retrying') targetPercent = 24;
  else if (event === 'agents_refresh_started') targetPercent = 56;
  else if (event === 'agents_hydrated') targetPercent = 76;
  else if (event === 'selection_applied') targetPercent = 90;
  else if (event === 'releasing') targetPercent = 97;
  else if (event === 'complete') targetPercent = 100;
  else targetPercent = 12;

  var stageTarget = infringBootProgressFromBootStage(meta && meta.bootStage);
  if (event === 'status_connected' || event === 'status_retrying') {
    targetPercent = Math.max(targetPercent, stageTarget);
  }
  if (event === 'complete') {
    infringSetBootProgressPercent(target, 100, { allowDecrease: true });
  } else {
    infringSetBootProgressPercent(target, targetPercent);
  }
  target.bootProgressEvent = event;
}

function infringNormalizeConnectionIndicatorState(state) {
  var raw = String(state || '').trim().toLowerCase();
  if (raw === 'connected') return 'connected';
  if (raw === 'disconnected') return 'disconnected';
  return 'connecting';
}

function infringQueueConnectionIndicatorState(page, state) {
  var target = page && typeof page === 'object' ? page : {};
  var next = infringNormalizeConnectionIndicatorState(state);
  var now = Date.now();
  var minIntervalMs = next === 'connecting' ? 1200 : 250;
  if (next !== 'connecting') {
    target.connectionIndicatorState = next;
    target._lastConnectionIndicatorAt = now;
    target._pendingConnectionIndicatorState = '';
    if (target._connectionIndicatorTimer) {
      clearTimeout(target._connectionIndicatorTimer);
      target._connectionIndicatorTimer = null;
    }
    return;
  }
  if (!target._lastConnectionIndicatorAt || (now - target._lastConnectionIndicatorAt) >= minIntervalMs) {
    target.connectionIndicatorState = next;
    target._lastConnectionIndicatorAt = now;
    target._pendingConnectionIndicatorState = '';
    if (target._connectionIndicatorTimer) {
      clearTimeout(target._connectionIndicatorTimer);
      target._connectionIndicatorTimer = null;
    }
    return;
  }
  target._pendingConnectionIndicatorState = next;
  if (target._connectionIndicatorTimer) return;
  var delay = Math.max(0, minIntervalMs - (now - target._lastConnectionIndicatorAt));
  target._connectionIndicatorTimer = setTimeout(function() {
    target._connectionIndicatorTimer = null;
    var pending = target._pendingConnectionIndicatorState || next;
    target._pendingConnectionIndicatorState = '';
    target.connectionIndicatorState = infringNormalizeConnectionIndicatorState(pending);
    target._lastConnectionIndicatorAt = Date.now();
  }, delay);
}
