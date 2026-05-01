// Agents page identity/config/store bridge helpers.
'use strict';

function infringAgentsIdentityStateMethods() {
  return {
    profileInfo: function(name) {
      return this.profileDescriptions[name] || { label: name, desc: '' };
    },

    mostRecentModelFromUsageCache() {
      try {
        var raw = localStorage.getItem('of-chat-model-usage-v1');
        if (!raw) return '';
        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return '';
        var bestModel = '';
        var bestTs = 0;
        Object.keys(parsed).forEach(function(key) {
          var modelId = String(key || '').trim();
          if (!modelId) return;
          var ts = Number(parsed[key] || 0);
          if (!Number.isFinite(ts) || ts <= 0) return;
          if (ts > bestTs) {
            bestTs = ts;
            bestModel = modelId;
          }
        });
        return bestModel;
      } catch(_) {
        return '';
      }
    },

    isAgentMissingError(err) {
      var msg = String(err && err.message ? err.message : '').toLowerCase();
      return msg.indexOf('agent_not_found') >= 0 || msg.indexOf('agent_not_archived') >= 0;
    },
    rememberAgentIdentity(agent, extra) {
      var sourceAgent = agent && typeof agent === 'object' ? agent : {};
      var extraPayload = extra && typeof extra === 'object' ? extra : {};
      var agentId = String(extraPayload.id || extraPayload.agent_id || sourceAgent.id || sourceAgent.agent_id || '').trim();
      if (!agentId) return null;
      if (!this.agentIdentityById || typeof this.agentIdentityById !== 'object') this.agentIdentityById = {};
      var prior = this.agentIdentityById[agentId] && typeof this.agentIdentityById[agentId] === 'object'
        ? this.agentIdentityById[agentId]
        : {};
      var identitySource = Object.assign(
        {},
        sourceAgent.identity && typeof sourceAgent.identity === 'object' ? sourceAgent.identity : {},
        extraPayload.identity && typeof extraPayload.identity === 'object' ? extraPayload.identity : {},
        extraPayload
      );
      if (!identitySource.name) identitySource.name = extraPayload.agent_name || sourceAgent.agent_name || sourceAgent.name || '';
      var mergedSource = Object.assign({}, sourceAgent, extraPayload, {
        id: agentId,
        name: identitySource.name || sourceAgent.name || extraPayload.name || '',
        identity: identitySource
      });
      var next = Object.assign({}, prior, identitySource, { id: agentId });
      var label = normalizeDashboardOptionalString(mergedSource.agent_name) || normalizeDashboardAgentLabel(mergedSource, next);
      var avatarUrl = resolveDashboardAgentAvatar(mergedSource, next);
      var emoji = resolveDashboardAgentEmoji(mergedSource, next);
      if (label) next.name = label;
      if (avatarUrl) {
        next.avatar = avatarUrl;
        next.avatar_url = avatarUrl;
      }
      if (emoji) next.emoji = emoji;
      this.agentIdentityById[agentId] = next;
      return next;
    },

    captureDetailConfigForm(agent, full) {
      var baseAgent = agent && typeof agent === 'object' ? agent : {};
      var source = full && typeof full === 'object' ? full : baseAgent;
      var config = source.config && typeof source.config === 'object'
        ? cloneDashboardConfigObject(source.config)
        : {};
      var configIdentity = config.identity && typeof config.identity === 'object' ? config.identity : {};
      var identity = Object.assign(
        {},
        baseAgent.identity && typeof baseAgent.identity === 'object' ? baseAgent.identity : {},
        source.identity && typeof source.identity === 'object' ? source.identity : {},
        configIdentity
      );
      var nextForm = {
        name: normalizeDashboardOptionalString(source.name || baseAgent.name),
        system_prompt: normalizeDashboardOptionalString(source.system_prompt || config.system_prompt),
        emoji: normalizeDashboardOptionalString(identity.emoji || config.emoji),
        color: normalizeDashboardOptionalString(identity.color || config.color || '#2563EB') || '#2563EB',
        archetype: normalizeDashboardOptionalString(identity.archetype || config.archetype),
        vibe: normalizeDashboardOptionalString(identity.vibe || config.vibe)
      };
      this.configFormOriginal = cloneDashboardConfigObject(nextForm);
      this.configForm = cloneDashboardConfigObject(nextForm);
      return this.configForm;
    },
    resetConfigForm() {
      var original = this.configFormOriginal && typeof this.configFormOriginal === 'object'
        ? this.configFormOriginal
        : {};
      this.configForm = cloneDashboardConfigObject(original);
      return this.configForm;
    },

    normalizePendingAgent(agent) {
      var source = agent && typeof agent === 'object' ? agent : {};
      var agentId = String(source.id || source.agent_id || '').trim();
      if (!agentId) return null;
      var identity = this.rememberAgentIdentity(source, source) || {};
      var label = normalizeDashboardOptionalString(source.agent_name) || normalizeDashboardAgentLabel(source, identity);
      var avatarUrl = resolveDashboardAgentAvatar(source, identity);
      var emoji = resolveDashboardAgentEmoji(source, identity);
      var normalizedIdentity = Object.assign(
        {},
        source.identity && typeof source.identity === 'object' ? source.identity : {},
        identity
      );
      if (avatarUrl) normalizedIdentity.avatar_url = avatarUrl;
      if (emoji) normalizedIdentity.emoji = emoji;
      return Object.assign({}, source, {
        id: agentId,
        name: label || agentId,
        state: normalizeDashboardOptionalString(source.state) || (source.archived ? 'archived' : 'Running'),
        role: normalizeDashboardOptionalString(source.role) || 'analyst',
        avatar_url: avatarUrl || normalizeDashboardOptionalString(source.avatar_url),
        avatar: emoji || normalizeDashboardOptionalString(source.avatar),
        identity: normalizedIdentity
      });
    },

    shellAppStoreBridge() {
      return typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
    },

    shellAppStore() {
      var bridge = this.shellAppStoreBridge();
      return bridge && typeof bridge.current === 'function' ? bridge.current() : null;
    },

    shellAppStoreMethod(name) {
      var bridge = this.shellAppStoreBridge();
      return bridge && typeof bridge.method === 'function' ? bridge.method(name) : null;
    },

    async refreshAgentsViaShellStore(options) {
      var refreshAgents = this.shellAppStoreMethod('refreshAgents');
      if (typeof refreshAgents === 'function') await refreshAgents(options);
    },

    assignShellAppStore(values) {
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.assign === 'function') return bridge.assign(values);
      var store = this.shellAppStore();
      if (store && values && typeof values === 'object') Object.assign(store, values);
      return store;
    },

    setActiveAgentIdViaShellStore(agentId) {
      var setActiveAgentId = this.shellAppStoreMethod('setActiveAgentId');
      if (typeof setActiveAgentId === 'function') {
        setActiveAgentId(agentId || null);
        return;
      }
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.set === 'function') bridge.set('activeAgentId', agentId || null);
    },

  };
}
