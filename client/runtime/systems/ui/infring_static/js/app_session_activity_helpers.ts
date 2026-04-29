// Canonical Shell helper source: dashboard session-activity notification projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

async function infringPollSessionActivity(page, force) {
  var target = page && typeof page === 'object' ? page : {};
  var now = Date.now();
  if (!force && target._lastSessionActivityPollAt && (now - Number(target._lastSessionActivityPollAt || 0)) < 8000) {
    return;
  }
  target._lastSessionActivityPollAt = now;
  try {
    var payload = await InfringAPI.get('/api/sessions');
    var rows = Array.isArray(payload && payload.sessions)
      ? payload.sessions
      : (Array.isArray(payload && payload.rows) ? payload.rows : []);
    var priorMap = target._sessionActivityByAgent && typeof target._sessionActivityByAgent === 'object'
      ? target._sessionActivityByAgent
      : {};
    var nextMap = {};
    var activeId = String(target.activeAgentId || '').trim();
    var noticesEmitted = 0;
    for (var i = 0; i < rows.length; i++) {
      var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
      if (!row) continue;
      var agentId = String(row.agent_id || '').trim();
      if (!agentId) continue;
      var messageCount = Number(row.message_count || 0);
      if (!Number.isFinite(messageCount) || messageCount < 0) messageCount = 0;
      var updatedAt = String(row.updated_at || '').trim();
      nextMap[agentId] = {
        message_count: messageCount,
        updated_at: updatedAt
      };
      if (!target._sessionActivityBootstrapped) continue;
      if (noticesEmitted >= 8) continue;
      var prior = priorMap[agentId];
      if (!prior || typeof prior !== 'object') continue;
      var priorCount = Number(prior.message_count || 0);
      if (!Number.isFinite(priorCount) || priorCount < 0) priorCount = 0;
      var priorUpdated = String(prior.updated_at || '').trim();
      var countIncreased = messageCount > priorCount;
      var updatedChanged = !!updatedAt && updatedAt !== priorUpdated;
      if (!countIncreased && !updatedChanged) continue;
      if (agentId === activeId) continue;

      if (typeof target.addNotification !== 'function') continue;

      var label = agentId === 'system' ? 'System' : ('Agent ' + agentId);
      var agent = null;
      if (Array.isArray(target.agents)) {
        agent = target.agents.find(function(entry) {
          return entry && String(entry.id || '').trim() === agentId;
        });
        if (agent) {
          var agentName = String(agent.name || '').trim();
          if (agentName) label = agentName;
        }
      }
      var serverPreview = agent && agent.sidebar_preview && typeof agent.sidebar_preview === 'object'
        ? agent.sidebar_preview
        : null;
      var preview = target.agentChatPreviews && target.agentChatPreviews[agentId]
        ? target.agentChatPreviews[agentId]
        : null;
      var previewText = '';
      if (serverPreview && typeof serverPreview.text === 'string') {
        previewText = serverPreview.text.replace(/\s+/g, ' ').trim();
      }
      if (!previewText && preview && typeof preview.text === 'string') {
        previewText = preview.text.replace(/\s+/g, ' ').trim();
      }
      if (previewText.length > 120) previewText = previewText.slice(0, 117) + '...';
      var summary = previewText || 'posted a new update.';
      var message = previewText ? (label + ': ' + previewText) : (label + ' posted a new update.');

      target.addNotification({
        type: agentId === 'system' ? 'warn' : 'info',
        message: message,
        ts: now + noticesEmitted,
        source: 'session_activity',
        page: 'chat',
        agent_id: agentId,
        summary: summary
      });
      noticesEmitted += 1;
    }
    target._sessionActivityByAgent = nextMap;
    target._sessionActivityBootstrapped = true;
  } catch(_) {}
}
