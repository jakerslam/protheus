// Canonical Shell helper source: agent preview/status projection helpers.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringMarkAgentPreviewUnread(page, agentId, unread) {
  var target = page && typeof page === 'object' ? page : {};
  var id = String(agentId || '').trim();
  if (!id) return;
  if (!target.agentChatPreviews) target.agentChatPreviews = {};
  if (!target.agentChatPreviews[id]) target.agentChatPreviews[id] = { text: '', ts: Date.now(), role: 'agent' };
  target.agentChatPreviews[id].unread_response = unread !== false;
}

function infringClassifyPreviewTool(tool) {
  if (!tool) return '';
  if (tool.running) return 'warning';
  var status = String(tool.status || '').toLowerCase();
  var result = String(tool.result || '').toLowerCase();
  var blocked = tool.blocked === true || status === 'blocked' ||
    result.indexOf('blocked') >= 0 ||
    result.indexOf('policy') >= 0 ||
    result.indexOf('denied') >= 0 ||
    result.indexOf('not allowed') >= 0 ||
    result.indexOf('forbidden') >= 0 ||
    result.indexOf('approval') >= 0 ||
    result.indexOf('permission') >= 0 ||
    result.indexOf('fail-closed') >= 0;
  if (blocked) return 'warning';
  if (tool.is_error) return 'error';
  return 'success';
}

function infringSummarizePreviewTools(tools) {
  if (!Array.isArray(tools) || !tools.length) return { has_tools: false, tool_state: '', tool_label: '' };
  var rank = { success: 1, warning: 2, error: 3 };
  var state = 'success';
  for (var i = 0; i < tools.length; i += 1) {
    var next = infringClassifyPreviewTool(tools[i]) || 'success';
    if ((rank[next] || 0) > (rank[state] || 0)) state = next;
  }
  var label = state === 'error' ? 'Tool error' : (state === 'warning' ? 'Tool warning' : 'Tool success');
  return { has_tools: true, tool_state: state, tool_label: label };
}

function infringSaveAgentChatPreview(page, agentId, messages) {
  var target = page && typeof page === 'object' ? page : {};
  if (!agentId) return;
  if (!target.agentChatPreviews) target.agentChatPreviews = {};
  var list = Array.isArray(messages) ? messages : [];
  var previewKey = String(agentId);
  var existingPreview = target.agentChatPreviews && target.agentChatPreviews[previewKey] ? target.agentChatPreviews[previewKey] : null;
  var preview = {
    text: '',
    ts: Date.now(),
    role: 'agent',
    has_tools: false,
    tool_state: '',
    tool_label: '',
    unread_response: !!(existingPreview && existingPreview.unread_response)
  };
  for (var i = list.length - 1; i >= 0; i -= 1) {
    var msg = list[i] || {};
    var text = '';
    var toolInfo = infringSummarizePreviewTools(msg.tools);
    if (typeof msg.text === 'string' && msg.text.trim()) {
      text = msg.text.replace(/\s+/g, ' ').trim();
    } else if (Array.isArray(msg.tools) && msg.tools.length) {
      text = '[Processes] ' + msg.tools.map(function(tool) { return tool && tool.name ? tool.name : 'tool'; }).join(', ');
    }
    if (text) {
      preview.text = text;
      preview.ts = Number(msg.ts || Date.now());
      preview.role = String(msg.role || 'agent');
      preview.has_tools = !!toolInfo.has_tools;
      preview.tool_state = toolInfo.tool_state || '';
      preview.tool_label = toolInfo.tool_label || '';
      break;
    }
  }
  if (preview.role === 'agent') preview.unread_response = String(target.activeAgentId || '') !== previewKey;
  else if (String(target.activeAgentId || '') === previewKey) preview.unread_response = false;

  var previewChanged = !!existingPreview && (
    Number(preview.ts || 0) > Number(existingPreview.ts || 0) ||
    String(preview.text || '') !== String(existingPreview.text || '') ||
    String(preview.role || '') !== String(existingPreview.role || '') ||
    String(preview.tool_state || '') !== String(existingPreview.tool_state || '')
  );
  var inactiveAgent = String(target.activeAgentId || '') !== previewKey;
  if (previewChanged && inactiveAgent && preview.role === 'agent' && String(preview.text || '').trim()) {
    var label = 'Agent';
    if (Array.isArray(target.agents)) {
      var found = target.agents.find(function(row) { return row && String(row.id || '') === previewKey; });
      if (found) {
        var foundName = String(found.name || '').trim();
        if (foundName) label = foundName;
      }
    }
    var compact = String(preview.text || '').replace(/\s+/g, ' ').trim();
    if (compact.length > 120) compact = compact.slice(0, 117) + '...';
    if (typeof target.addNotification === 'function') {
      target.addNotification({ type: 'info', message: label + ': ' + compact, agent_id: previewKey, page: 'chat', source: 'agent_preview', ts: Number(preview.ts || Date.now()) });
    }
  }
  target.agentChatPreviews[previewKey] = preview;
}

function infringGetAgentChatPreview(page, agentId) {
  var target = page && typeof page === 'object' ? page : {};
  if (!agentId) return null;
  return target.agentChatPreviews ? target.agentChatPreviews[String(agentId)] || null : null;
}

function infringCoerceAgentTimestamp(value) {
  if (value === null || typeof value === 'undefined' || value === '') return 0;
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) return 0;
    return value < 1e12 ? Math.round(value * 1000) : Math.round(value);
  }
  var asNum = Number(value);
  if (Number.isFinite(asNum) && String(value).trim() !== '') return asNum < 1e12 ? Math.round(asNum * 1000) : Math.round(asNum);
  var asDate = Number(new Date(value).getTime());
  return Number.isFinite(asDate) ? asDate : 0;
}

function infringAgentLastActivityTs(page, agent) {
  if (!agent) return 0;
  var latest = 0;
  var keys = ['last_active_at', 'last_activity_at', 'last_message_at', 'last_seen_at', 'updated_at'];
  for (var i = 0; i < keys.length; i += 1) {
    var ts = infringCoerceAgentTimestamp(agent[keys[i]]);
    if (ts > latest) latest = ts;
  }
  if (agent.id) {
    var preview = infringGetAgentChatPreview(page, agent.id);
    var previewTs = infringCoerceAgentTimestamp(preview && preview.ts);
    if (previewTs > latest) latest = previewTs;
  }
  return latest;
}

function infringAgentStatusFreshness(agent) {
  var raw = agent && agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object' ? agent.sidebar_status_freshness : {};
  var source = String((raw.source || (agent && agent.sidebar_status_source) || '')).trim();
  var sourceSequence = String((raw.source_sequence || (agent && agent.sidebar_status_source_sequence) || '')).trim();
  var ageRaw = Number(typeof raw.age_seconds !== 'undefined' ? raw.age_seconds : (agent && agent.sidebar_status_age_seconds));
  var ageSeconds = Number.isFinite(ageRaw) && ageRaw >= 0 ? ageRaw : 0;
  var staleRaw = raw.stale;
  if (typeof staleRaw !== 'boolean' && agent && typeof agent.sidebar_status_stale === 'boolean') staleRaw = agent.sidebar_status_stale;
  return { source: source, source_sequence: sourceSequence, age_seconds: ageSeconds, stale: staleRaw === true };
}

function infringAgentStatusState(agent) {
  if (!agent) return 'offline';
  var serverState = String(typeof agent.sidebar_status_state === 'string' ? agent.sidebar_status_state : '').trim().toLowerCase();
  if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
  var freshness = infringAgentStatusFreshness(agent);
  if (freshness.stale) return 'offline';
  return 'offline';
}

function infringAgentStatusLabel(agent) {
  var serverLabel = String(agent && typeof agent.sidebar_status_label === 'string' ? agent.sidebar_status_label : '').trim().toLowerCase();
  if (serverLabel === 'active' || serverLabel === 'idle' || serverLabel === 'offline') return serverLabel;
  var serverState = String(agent && typeof agent.sidebar_status_state === 'string' ? agent.sidebar_status_state : '').trim().toLowerCase();
  if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
  var freshness = infringAgentStatusFreshness(agent);
  if (freshness.stale) return 'offline';
  return 'offline';
}

function infringSetAgentLiveActivity(page, agentId, state) {
  var target = page && typeof page === 'object' ? page : {};
  var id = String(agentId || '').trim();
  if (!id) return;
  var normalized = String(state || '').trim().toLowerCase();
  if (!normalized || normalized === 'idle' || normalized === 'done' || normalized === 'stop' || normalized === 'stopped') {
    if (target.agentLiveActivity && Object.prototype.hasOwnProperty.call(target.agentLiveActivity, id)) {
      delete target.agentLiveActivity[id];
      target.agentLiveActivity = Object.assign({}, target.agentLiveActivity);
    }
    return;
  }
  target.agentLiveActivity = Object.assign({}, target.agentLiveActivity || {}, { [id]: { state: normalized, ts: Date.now() } });
}

function infringClearAgentLiveActivity(page, agentId) {
  infringSetAgentLiveActivity(page, agentId, 'idle');
}

function infringIsAgentLiveBusy(page, agent) {
  var target = page && typeof page === 'object' ? page : {};
  if (!agent || !agent.id) return false;
  var id = String(agent.id);
  var entry = target.agentLiveActivity ? target.agentLiveActivity[id] : null;
  if (!entry) return false;
  var state = String(entry.state || '').toLowerCase();
  var ts = Number(entry.ts || 0);
  var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
  return !!(busyState && Number.isFinite(ts) && (Date.now() - ts) <= 180000);
}
