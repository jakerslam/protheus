// Canonical Shell helper source: dashboard notification projection helpers.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringNormalizeNotificationType(rawType, message) {
  var value = String(rawType || '').trim().toLowerCase();
  if (!value) {
    var text = String(message || '').toLowerCase();
    if (/(completed|complete|done|success|succeeded|finished|resolved)/.test(text)) {
      value = 'completed';
    } else if (/(error|failed|failure|aborted|abort|exception|crash|denied|timeout)/.test(text)) {
      value = 'error';
    } else {
      value = 'info';
    }
  }
  if (['completed', 'complete', 'done', 'success', 'ok', 'resolved', 'action_completed', 'task_completed'].indexOf(value) >= 0) {
    return 'completed';
  }
  if (['error', 'failed', 'failure', 'fatal', 'critical', 'danger', 'exception', 'aborted', 'abort', 'timeout'].indexOf(value) >= 0) {
    return 'error';
  }
  return 'info';
}

function infringRingNotificationBell(page) {
  var target = page && typeof page === 'object' ? page : {};
  var seq = Number(target._notificationBellPulseSeq || 0) + 1;
  target._notificationBellPulseSeq = seq;
  target.notificationBellPulse = false;
  if (target._notificationBellPulseTimer) {
    clearTimeout(target._notificationBellPulseTimer);
    target._notificationBellPulseTimer = null;
  }
  var arm = function() {
    if (target._notificationBellPulseSeq !== seq) return;
    target.notificationBellPulse = true;
    target._notificationBellPulseTimer = setTimeout(function() {
      if (target._notificationBellPulseSeq !== seq) return;
      target.notificationBellPulse = false;
      target._notificationBellPulseTimer = null;
    }, 760);
  };
  if (typeof requestAnimationFrame === 'function') requestAnimationFrame(arm);
  else setTimeout(arm, 0);
}

function infringShowNotificationBubble(page, note) {
  var target = page && typeof page === 'object' ? page : {};
  var n = note || null;
  if (!n) return;
  target.notificationBubble = { id: n.id, message: n.message, type: n.type, ts: n.ts };
  if (target._notificationBubbleTimer) clearTimeout(target._notificationBubbleTimer);
  target._notificationBubbleTimer = setTimeout(function() {
    target.notificationBubble = null;
  }, 5200);
}

function infringAddNotification(page, payload) {
  var target = page && typeof page === 'object' ? page : {};
  if (!Array.isArray(target.notifications)) target.notifications = [];
  var p = payload || {};
  var noteTs = Number(p.ts || Date.now());
  if (!Number.isFinite(noteTs) || noteTs <= 0) noteTs = Date.now();
  var noteMessage = String(p.message || '');
  var noteType = infringNormalizeNotificationType(p.type, noteMessage);
  var noteAgentId = String(p.agent_id || p.agentId || '').trim();
  if (target.notifications.length) {
    var prior = target.notifications[0] || null;
    if (prior && String(prior.message || '') === noteMessage && String(prior.type || '') === noteType && String(prior.agent_id || '') === noteAgentId && Math.abs(noteTs - Number(prior.ts || 0)) <= 2200) return;
  }
  var note = {
    id: p.id || ('notif-' + (++target._notificationSeq) + '-' + Date.now()),
    message: noteMessage,
    type: noteType,
    ts: noteTs,
    read: !!target.notificationsOpen,
    page: String(p.page || '').trim(),
    agent_id: noteAgentId,
    source: String(p.source || '').trim()
  };
  target.notifications.unshift(note);
  if (target.notifications.length > 150) target.notifications = target.notifications.slice(0, 150);
  target.unreadNotifications = target.notifications.filter(function(n) { return !n.read; }).length;
  infringRingNotificationBell(target);
  infringShowNotificationBubble(target, note);
}

function infringToggleNotifications(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.notificationsOpen = !target.notificationsOpen;
  if (target.notificationsOpen) infringMarkAllNotificationsRead(target);
}

function infringMarkNotificationRead(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  target.notifications = (Array.isArray(target.notifications) ? target.notifications : []).map(function(n) {
    if (n.id === id) n.read = true;
    return n;
  });
  target.unreadNotifications = target.notifications.filter(function(n) { return !n.read; }).length;
}

function infringMarkAllNotificationsRead(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.notifications = (Array.isArray(target.notifications) ? target.notifications : []).map(function(n) {
    n.read = true;
    return n;
  });
  target.unreadNotifications = 0;
}

function infringDismissNotification(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var targetId = String(id || '').trim();
  if (!targetId) return;
  target.notifications = (Array.isArray(target.notifications) ? target.notifications : []).filter(function(n) {
    return String(n && n.id ? n.id : '') !== targetId;
  });
  target.unreadNotifications = target.notifications.filter(function(n) { return !n.read; }).length;
  if (target.notificationBubble && String(target.notificationBubble.id || '') === targetId) infringDismissNotificationBubble(target);
}

function infringClearNotifications(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.notifications = [];
  target.notificationsOpen = false;
  target.unreadNotifications = 0;
  target.notificationBubble = null;
  target.notificationBellPulse = false;
  target._notificationBellPulseSeq = 0;
  if (target._notificationBellPulseTimer) {
    clearTimeout(target._notificationBellPulseTimer);
    target._notificationBellPulseTimer = null;
  }
  if (target._notificationBubbleTimer) {
    clearTimeout(target._notificationBubbleTimer);
    target._notificationBubbleTimer = null;
  }
}

function infringReopenNotification(page, note) {
  var target = page && typeof page === 'object' ? page : {};
  if (!note) return;
  infringMarkNotificationRead(target, note.id);
  infringShowNotificationBubble(target, note);
  target.notificationsOpen = false;
  var targetAgentId = String(note.agent_id || '').trim();
  var targetPage = String(note.page || '').trim();
  if (targetAgentId) {
    if (typeof target.setActiveAgentId === 'function') target.setActiveAgentId(targetAgentId);
    else target.activeAgentId = targetAgentId;
  }
  if (targetPage) window.location.hash = targetPage;
  else if (targetAgentId) window.location.hash = 'chat';
}

function infringDismissNotificationBubble(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.notificationBubble = null;
  if (target._notificationBubbleTimer) {
    clearTimeout(target._notificationBubbleTimer);
    target._notificationBubbleTimer = null;
  }
}
