// Chat session scope and input-history reconstruction helpers.
'use strict';

function infringChatSessionScopeMethods() {
  return {
    extractTerminalCommandsFromHistoryText: function(rawText) {
      var text = String(rawText || '');
      if (!text.trim()) return [];
      var lines = text.split('\n');
      var out = [];
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        var marker = line.indexOf(' % ');
        if (marker <= 0) continue;
        var cmd = line.slice(marker + 3).trim();
        if (cmd) out.push(cmd);
      }
      return out;
    },

    normalizeSessionKeyToken: function(value, fallback) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9:_-]+/g, '-').replace(/^-+|-+$/g, '');
      if (raw) return raw;
      var fallbackValue = String(fallback == null ? '' : fallback).trim().toLowerCase();
      return fallbackValue || 'main';
    },

    normalizeSessionAgentId: function(value) {
      var raw = String(value == null ? '' : value).trim().toLowerCase();
      raw = raw.replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '');
      return raw || 'main';
    },

    parseAgentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return null;
      var parts = raw.split(':').filter(Boolean);
      if (parts.length < 3 || parts[0] !== 'agent') return null;
      var agentId = this.normalizeSessionAgentId(parts[1]);
      var rest = parts.slice(2).join(':');
      if (!rest) return null;
      return {
        agentId: agentId,
        rest: this.normalizeSessionKeyToken(rest, 'main')
      };
    },

    resolveSessionAgentIdFromKey: function(sessionKey, fallbackAgentId) {
      var parsed = this.parseAgentSessionKey(sessionKey);
      if (parsed && parsed.agentId) return parsed.agentId;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    isSubagentSessionKey: function(sessionKey) {
      var raw = String(sessionKey == null ? '' : sessionKey).trim().toLowerCase();
      if (!raw) return false;
      if (raw.indexOf('subagent:') === 0) return true;
      var parsed = this.parseAgentSessionKey(raw);
      return !!(parsed && parsed.rest.indexOf('subagent:') === 0);
    },

    resolveSessionRowScopeToken: function(row) {
      var rawKey = String(
        (row && (row.session_key || row.key || row.session_id || row.id || row.main_key)) || ''
      ).trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      if (parsed && parsed.rest) return parsed.rest;
      return this.normalizeSessionKeyToken(rawKey, 'main');
    },

    resolveSessionRowLabel: function(row, fallbackAgentId) {
      var explicitLabel = String((row && (row.label || row.name || row.session_label)) || '').trim();
      if (explicitLabel) return explicitLabel;
      var rawKey = String((row && (row.session_key || row.key || row.session_id || row.id)) || '').trim();
      var parsed = this.parseAgentSessionKey(rawKey);
      var scopeToken = parsed && parsed.rest ? parsed.rest : this.resolveSessionRowScopeToken(row);
      if (scopeToken === 'main') return 'Main';
      if (scopeToken.indexOf('subagent:') === 0) {
        var subagentTail = scopeToken.slice('subagent:'.length).replace(/[:_-]+/g, ' ').trim();
        return subagentTail ? ('Subagent ' + subagentTail) : 'Subagent';
      }
      var normalized = String(scopeToken || '').replace(/[:_-]+/g, ' ').trim();
      if (normalized) return normalized;
      return this.normalizeSessionAgentId(fallbackAgentId);
    },

    normalizeSessionsList: function(rows, fallbackAgentId) {
      var source = Array.isArray(rows) ? rows : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i++) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var rawKey = String((row.session_key || row.key || row.session_id || row.id) || '').trim();
        var agentId = this.resolveSessionAgentIdFromKey(rawKey, row.agent_id || row.agentId || fallbackAgentId);
        var scopeToken = this.resolveSessionRowScopeToken(row);
        var scopeKey = this.normalizeSessionAgentId(agentId) + '|' + scopeToken;
        if (seen[scopeKey]) continue;
        seen[scopeKey] = true;
        out.push(Object.assign({}, row, {
          _agent_id: this.normalizeSessionAgentId(agentId),
          _scope_token: scopeToken,
          _scope_key: scopeKey,
          _label: this.resolveSessionRowLabel(row, agentId),
          _is_subagent: this.isSubagentSessionKey(rawKey),
        }));
      }
      return out;
    },

    resolveCurrentSessionRow: function(agentId) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = this.normalizeSessionsList(this.sessions || [], normalizedAgentId);
      var fallback = null;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        if (!fallback && row._agent_id === normalizedAgentId) fallback = row;
        if (row._agent_id === normalizedAgentId && row.active === true) return row;
      }
      if (fallback) return fallback;
      for (var j = 0; j < rows.length; j++) {
        if (rows[j] && rows[j].active === true) return rows[j];
      }
      return rows.length ? rows[0] : null;
    },

    resolveConversationCacheScopeKey: function(agentId, explicitSessionRow) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var row = explicitSessionRow && typeof explicitSessionRow === 'object'
        ? explicitSessionRow
        : this.resolveCurrentSessionRow(normalizedAgentId);
      var scopeToken = row && row._scope_token
        ? row._scope_token
        : this.resolveSessionRowScopeToken(row || {});
      return normalizedAgentId + '|' + this.normalizeSessionKeyToken(scopeToken, 'main');
    },

    applySessionsPayloadSnapshot: function(agentId, payload) {
      var normalizedAgentId = this.normalizeSessionAgentId(agentId);
      var rows = [];
      if (payload && payload.session && Array.isArray(payload.session.sessions)) {
        rows = payload.session.sessions;
      } else if (payload && Array.isArray(payload.sessions)) {
        rows = payload.sessions;
      }
      var normalizedRows = this.normalizeSessionsList(rows, normalizedAgentId);
      if (!normalizedRows.length) return;
      this.sessions = normalizedRows;
      var chatStore = window.InfringChatStore;
      if (chatStore && chatStore.sessions) chatStore.sessions.set(normalizedRows);
      if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
        this._sessionsLastLoadedAtByAgent = {};
      }
      this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
    },

    rebuildInputHistoryFromSessionPayload: function(data) {
      var payload = data && typeof data === 'object' ? data : {};
      var fallbackAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      this.applySessionsPayloadSnapshot(fallbackAgentId, payload);
      var state = payload && payload.session && typeof payload.session === 'object' ? payload.session : {};
      var sessions = this.normalizeSessionsList(Array.isArray(state.sessions) ? state.sessions : [], fallbackAgentId);
      var sourceRows = [];
      var seenSessionScopes = {};
      if (payload && payload.message_window && Array.isArray(payload.message_window.rows)) {
        for (var w = 0; w < payload.message_window.rows.length; w++) sourceRows.push(payload.message_window.rows[w]);
      } else {
        for (var i = 0; i < sessions.length; i++) {
          var session = sessions[i] || {};
          var scopeKey = String(session._scope_key || '').trim();
          if (scopeKey && seenSessionScopes[scopeKey]) continue;
          if (scopeKey) seenSessionScopes[scopeKey] = true;
          var messages = Array.isArray(session.messages) ? session.messages : [];
          for (var j = 0; j < messages.length; j++) sourceRows.push(messages[j]);
        }
      }
      if (!sourceRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }

      var normalized = this.normalizeSessionMessages({ messages: sourceRows });
      var maxEntries = Number(this.inputHistoryMaxEntries || 0);
      if (!Number.isFinite(maxEntries) || maxEntries < 20) maxEntries = 120;
      var chatRows = [];
      var terminalRows = [];
      for (var k = 0; k < normalized.length; k++) {
        var row = normalized[k] || {};
        var role = String(row.role || '').toLowerCase();
        var text = String(row.text || '').trim();
        if (!text) continue;
        if (role === 'user') {
          chatRows.push(text);
          continue;
        }
        var isTerminal = !!row.terminal || role === 'terminal';
        if (!isTerminal) continue;
        var source = String(row.terminal_source || '').toLowerCase();
        if (source && source !== 'user') continue;
        var commands = this.extractTerminalCommandsFromHistoryText(text);
        for (var c = 0; c < commands.length; c++) {
          var command = String(commands[c] || '').trim();
          if (command) terminalRows.push(command);
        }
      }
      if (!chatRows.length && !terminalRows.length) {
        this.chatInputHistory = [];
        this.terminalInputHistory = [];
        this.hydrateInputHistoryFromCache('chat');
        this.hydrateInputHistoryFromCache('terminal');
        this.resetInputHistoryNavigation('chat');
        this.resetInputHistoryNavigation('terminal');
        return;
      }
      chatRows = chatRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      terminalRows = terminalRows.filter(function(text, idx, arr) { return idx === 0 || text !== arr[idx - 1]; });
      if (chatRows.length > maxEntries) chatRows = chatRows.slice(chatRows.length - maxEntries);
      if (terminalRows.length > maxEntries) terminalRows = terminalRows.slice(terminalRows.length - maxEntries);


      this.chatInputHistory = chatRows;
      this.terminalInputHistory = terminalRows;
      this.hydrateInputHistoryFromCache('chat', fallbackAgentId);
      this.hydrateInputHistoryFromCache('terminal', fallbackAgentId);
      this.syncInputHistoryToCache('chat', fallbackAgentId);
      this.syncInputHistoryToCache('terminal', fallbackAgentId);
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
    },
  };
}
