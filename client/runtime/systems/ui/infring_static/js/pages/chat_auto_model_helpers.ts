'use strict';

function infringChatAutoModelMethods() {
  return {
    isAutoModelSelected() {
      return !!(
        this.currentAgent &&
        String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto'
      );
    },

    formatAutoRouteMeta(route) {
      if (!route || typeof route !== 'object') return '';
      var provider = String(route.provider || route.selected_provider || '').trim();
      var model = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      if (!model) return '';
      var shortModel = model;
      if (shortModel.indexOf('/') >= 0) {
        shortModel = shortModel.split('/').slice(-1)[0];
      }
      var reason = String(route.reason || '').trim();
      if (reason.length > 80) reason = reason.slice(0, 77) + '...';
      var prefix = provider ? ('Auto -> ' + provider + '/' + shortModel) : ('Auto -> ' + shortModel);
      return reason ? (prefix + ' (' + reason + ')') : prefix;
    },

    normalizeAutoModelNoticeName(modelId) {
      var value = String(modelId || '').trim();
      if (!value) return '';
      if (value.indexOf('/') >= 0) {
        value = value.split('/').slice(-1)[0];
      }
      return value.replace(/-\d{8}$/, '');
    },

    formatAutoModelSwitchLabel(modelId) {
      var normalized = this.normalizeAutoModelNoticeName(modelId);
      if (!normalized && this.currentAgent) {
        normalized = this.normalizeAutoModelNoticeName(
          this.currentAgent.runtime_model || this.currentAgent.model_name || ''
        );
      }
      return 'Auto:[' + (normalized || 'unknown') + ']';
    },

    captureAutoModelSwitchBaseline() {
      if (!this.currentAgent || !this.isAutoModelSelected()) return '';
      var current = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      return this.formatAutoModelSwitchLabel(current);
    },

    maybeAddAutoModelSwitchNotice(previousLabel, route) {
      if (!this.currentAgent || !this.isAutoModelSelected()) return;
      var previous = String(previousLabel || '').trim();
      if (!previous) {
        previous = this.formatAutoModelSwitchLabel(this.currentAgent.runtime_model || this.currentAgent.model_name || '');
      }
      var nextModel = '';
      if (route && typeof route === 'object') {
        nextModel = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      }
      if (!nextModel) {
        nextModel = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      }
      var next = this.formatAutoModelSwitchLabel(nextModel);
      if (!next || previous === next) return;
      this.addNoticeEvent({
        notice_label: 'Model switched from ' + previous + ' to ' + next,
        notice_type: 'model',
        ts: Date.now()
      });
    },

    applyAutoRouteTelemetry(data) {
      if (!data || typeof data !== 'object') return null;
      var route = null;
      if (data.auto_route && typeof data.auto_route === 'object') {
        route = data.auto_route;
      } else if (data.route && typeof data.route === 'object') {
        route = data.route;
      }
      if (!route) return null;
      if (!this.currentAgent) return route;
      if (!this.isAutoModelSelected()) return route;
      var provider = String(route.provider || route.selected_provider || this.currentAgent.model_provider || '').trim();
      var model = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      if (provider) this.currentAgent.model_provider = provider;
      if (model) {
        this.currentAgent.runtime_model = model.indexOf('/') >= 0 ? model.split('/').slice(-1)[0] : model;
        this.touchModelUsage(this.currentAgent.runtime_model);
      }
      this.setContextWindowFromCurrentAgent();
      return route;
    },

    collectContextWindowCandidatesFromAgent(agent) {
      var row = agent && typeof agent === 'object' ? agent : {};
      var provider = String(row.model_provider || row.provider || '').trim().toLowerCase();
      var out = [];
      var seen = {};
      var push = function(value) {
        var key = String(value || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      };
      var modelName = String(row.model_name || '').trim();
      var runtimeModel = String(row.runtime_model || '').trim();
      push(modelName);
      push(runtimeModel);
      if (provider && modelName && modelName.indexOf('/') < 0) push(provider + '/' + modelName);
      if (provider && runtimeModel && runtimeModel.indexOf('/') < 0) push(provider + '/' + runtimeModel);
      if (modelName.indexOf('/') >= 0) push(modelName.split('/').slice(-1)[0]);
      if (runtimeModel.indexOf('/') >= 0) push(runtimeModel.split('/').slice(-1)[0]);
      return out;
    },

    resolveBestContextWindowFromMap(candidates) {
      var keys = Array.isArray(candidates) ? candidates : [];
      var map = this._contextWindowByModel || {};
      var best = 0;
      for (var i = 0; i < keys.length; i += 1) {
        var fromMap = Number(map[keys[i]] || 0);
        if (!Number.isFinite(fromMap) || fromMap <= 0) continue;
        if (fromMap > best) best = fromMap;
      }
      return best;
    },

    refreshContextWindowMap(models) {
      var next = {};
      var rows = Array.isArray(models) ? models : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var id = String(row.id || '').trim();
        if (!id) continue;
        var provider = String(row.provider || row.model_provider || '').trim().toLowerCase();
        var windowSize = Number(row.context_window || row.context_window_tokens || 0);
        if (Number.isFinite(windowSize) && windowSize > 0) {
          var normalized = Math.round(windowSize);
          var keys = [id];
          if (id.indexOf('/') >= 0) {
            keys.push(id.split('/').slice(-1)[0]);
          } else if (provider) {
            keys.push(provider + '/' + id);
          }
          for (var k = 0; k < keys.length; k += 1) {
            var key = String(keys[k] || '').trim();
            if (!key) continue;
            var prior = Number(next[key] || 0);
            if (!Number.isFinite(prior) || normalized > prior) next[key] = normalized;
          }
        }
      }
      this._contextWindowByModel = next;
    },

    setContextWindowFromCurrentAgent() {
      var agent = this.currentAgent || {};
      var direct = Number(agent.context_window || agent.context_window_tokens || 0);
      var candidates = this.collectContextWindowCandidatesFromAgent(agent);
      var fromMap = this.resolveBestContextWindowFromMap(candidates);
      var best = 0;
      if (Number.isFinite(direct) && direct > 0) best = direct;
      if (Number.isFinite(fromMap) && fromMap > best) best = fromMap;
      if (!Number.isFinite(best) || best <= 0) best = 128000;
      this.contextWindow = Math.round(best);
      this.refreshContextPressure();
    },

    refreshContextPressure() {
      var windowSize = Number(this.contextWindow || 0);
      var used = Number(this.contextApproxTokens || 0);
      if (!Number.isFinite(windowSize) || windowSize <= 0 || !Number.isFinite(used) || used < 0) {
        this.contextPressure = 'low';
        return;
      }
      var ratio = used / windowSize;
      if (ratio >= 0.96) this.contextPressure = 'critical';
      else if (ratio >= 0.82) this.contextPressure = 'high';
      else if (ratio >= 0.55) this.contextPressure = 'medium';
      else this.contextPressure = 'low';
    },
  };
}
