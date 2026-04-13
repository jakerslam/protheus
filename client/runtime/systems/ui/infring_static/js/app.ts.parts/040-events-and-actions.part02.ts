      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var monthMin = 30 * 24 * 60;
      if (totalMin >= monthMin) {
        return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
      }
      if (totalMin >= 1440) {
        return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
      }
      if (totalMin >= 60) {
        return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
      }
      return totalMin + 'm';
    },

    expiryCountdownCritical(agent) {
      if (agent && agent._timed_out_local === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      var thresholdMs = 3600000;
      if (Number.isFinite(totalMs) && totalMs > 0) {
        thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      }
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      var durationMs = Number(agent.contract_duration_ms != null ? agent.contract_duration_ms : (contract && contract.duration_ms));
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      var durationSeconds = Number(agent.contract_duration_seconds != null ? agent.contract_duration_seconds : (contract && contract.duration_seconds));
      if (Number.isFinite(durationSeconds) && durationSeconds > 0) return Math.floor(durationSeconds * 1000);
      var expiryMs = this.agentContractExpiryMs(agent);
      if (expiryMs <= 0) return null;
      var startedAt = String(
        agent.contract_started_at ||
        (contract && contract.started_at ? contract.started_at : '') ||
        agent.created_at ||
        (contract && contract.created_at ? contract.created_at : '') ||
        ''
      ).trim();
      var startedTs = Number(new Date(startedAt).getTime());
      if (Number.isFinite(startedTs) && startedTs > 0 && expiryMs > startedTs) {
        return Math.max(1000, Math.floor(expiryMs - startedTs));
      }
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs <= 0) return null;
      return Math.max(remainingMs, 3600000);
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent._timed_out_local === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return hearts;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) totalMs = Math.max(1, remainingMs);
      var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
      var filled = Math.ceil(ratio * totalHearts);
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) filled = 0;
      if (filled < 0) filled = 0;
      if (filled > totalHearts) filled = totalHearts;
      for (var i = 0; i < totalHearts; i++) {
        hearts[i] = i < filled;
      }
      return hearts;
    },

    agentHeartShowsInfinity(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread) return false;
      if (agent._timed_out_local === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent._timed_out_local === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
    },

    closeTopbarHeroMenu() {
      this.topbarHeroMenuOpen = false;
    },

    toggleTopbarHeroMenu() {
      if (this.topbarHeroActionPending) return;
      this.topbarHeroMenuOpen = !this.topbarHeroMenuOpen;
    },

    requestTopbarRefresh() {
      this.closeTopbarHeroMenu();
      var appStore = this.getAppStore ? this.getAppStore() : null;
      if (appStore && typeof appStore.bumpTopbarRefreshTurn === 'function') {
        appStore.bumpTopbarRefreshTurn();
      }
      if (this._topbarRefreshOverlayTimer) {
        clearTimeout(this._topbarRefreshOverlayTimer);
        this._topbarRefreshOverlayTimer = 0;
      }
      if (this._topbarRefreshReloadTimer) {
        clearTimeout(this._topbarRefreshReloadTimer);
        this._topbarRefreshReloadTimer = 0;
      }
      var self = this;
      this._topbarRefreshOverlayTimer = window.setTimeout(function() {
        self.bootSplashVisible = true;
        self._bootSplashStartedAt = Date.now();
        if (typeof self.resetBootProgress === 'function') self.resetBootProgress();
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        self._topbarRefreshOverlayTimer = 0;
      }, 1000);
      this._topbarRefreshReloadTimer = window.setTimeout(function() {
        self._topbarRefreshReloadTimer = 0;
        try {
          window.location.reload();
        } catch (_) {
          try {
            window.location.href = window.location.href;
          } catch (_) {}
        }
      }, 1100);
    },

    async postTopbarHeroSystemRoute(route, body, options) {
      var opts = (options && typeof options === 'object') ? options : {};
      var timeoutMs = Number(opts.timeoutMs);
      if (!Number.isFinite(timeoutMs) || timeoutMs < 250) timeoutMs = 1800;
      var allowTransientSuccess = opts.allowTransientSuccess === true;
      var controller = null;
      try {
        if (typeof AbortController !== 'undefined') controller = new AbortController();
      } catch (_) {
        controller = null;
      }
      var timer = 0;
      if (controller && typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        timer = window.setTimeout(function() {
          try {
            controller.abort();
          } catch (_) {}
        }, timeoutMs);
      }
      try {
        var headers = { 'Content-Type': 'application/json' };
        try {
          var token = String(localStorage.getItem('infring-api-key') || '').trim();
          if (token) headers.Authorization = 'Bearer ' + token;
        } catch (_) {}
        var response = await fetch(route, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify(body || {}),
          signal: controller ? controller.signal : undefined
        });
        var text = '';
        try {
          text = await response.text();
        } catch (_) {
          text = '';
        }
        var parsed = {};
        try {
          parsed = text ? JSON.parse(text) : {};
        } catch (_) {
          parsed = {};
        }
        if (!response.ok) {
          var error = new Error(String((parsed && (parsed.error || parsed.message)) || ('system_route_http_' + response.status)));
          error.status = response.status;
          error.payload = parsed;
          throw error;
        }
        return parsed && typeof parsed === 'object' ? parsed : {};
      } catch (error) {
        var message = String(error && error.message ? error.message : '');
        var aborted = !!(controller && controller.signal && controller.signal.aborted) || (error && error.name === 'AbortError');
        var disconnected =
          error &&
          error.name === 'TypeError' &&
          (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
        if (allowTransientSuccess && (aborted || disconnected)) {
          return {
            ok: true,
            type: 'dashboard_system_action_assumed',
            accepted_transient_disconnect: true
          };
        }
        throw error;
      } finally {
        if (timer) {
          try {
            clearTimeout(timer);
          } catch (_) {}
        }
      }
    },

    async runTopbarHeroCommand(action) {
      var actionKey = String(action || '').trim().toLowerCase();
      if (!actionKey || this.topbarHeroActionPending) return;
      var dashboardAction = '';
      var legacyRoute = '';
      var body = {};
      if (actionKey === 'restart') {
        dashboardAction = 'dashboard.system.restart';
        legacyRoute = '/api/system/restart';
      }
      else if (actionKey === 'shutdown') {
        dashboardAction = 'dashboard.system.shutdown';
        legacyRoute = '/api/system/shutdown';
      }
      else if (actionKey === 'update') {
        dashboardAction = 'dashboard.update.apply';
        legacyRoute = '/api/system/update';
        body = { apply: true };
      } else {
        return;
      }
      this.topbarHeroActionPending = actionKey;
      try {
        var result = null;
        try {
          result = await this.postTopbarHeroSystemRoute(legacyRoute, body, {
            timeoutMs: actionKey === 'update' ? 12000 : 1400,
            allowTransientSuccess: actionKey === 'restart' || actionKey === 'shutdown'
          });
        } catch (routeError) {
          var routeStatus = Number(routeError && routeError.status || 0);
          var routeMessage = String(routeError && routeError.message ? routeError.message : '').toLowerCase();
          var canFallbackToActionBus =
            !!dashboardAction &&
            (
              routeStatus === 404 ||
              routeStatus === 400 ||
              routeMessage.indexOf('unknown_action') >= 0 ||
              routeMessage.indexOf('resource not found') >= 0
            );
          if (!canFallbackToActionBus) throw routeError;
          result = await InfringAPI.post('/api/dashboard/action', {
            action: dashboardAction,
            payload: body
          });
        }
        var payload =
          result && result.lane && typeof result.lane === 'object'
            ? result.lane
            : (
              result && result.payload && typeof result.payload === 'object'
                ? result.payload
                : result
            );
        if (result && result.ok === false) {
          throw new Error(String(result.error || payload.error || (actionKey + '_failed')));
        }
        this.closeTopbarHeroMenu();
        if (actionKey === 'restart') {
          InfringToast.success('Restart requested');
          this.requestTopbarRefresh();
        } else if (actionKey === 'shutdown') {
          InfringToast.success('Shut down requested');
          this.connected = false;
          this.connectionState = 'disconnected';
          this.wsConnected = false;
        } else {
          var updateAvailable = payload.update_available;
          if (updateAvailable == null && payload.post_check && typeof payload.post_check === 'object') {
            updateAvailable = payload.post_check.has_update;
          }
          if (updateAvailable === false) {
            InfringToast.success('Already up to date');
          } else {
            InfringToast.success('Update requested');
          }
          this.requestTopbarRefresh();
        }
      } catch (e) {
        InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
      } finally {
        this.topbarHeroActionPending = '';
      }
    },

    normalizeDashboardHealthSummary(payload) {
      var summary = payload && typeof payload === 'object' ? payload : {};
      var agents = Array.isArray(summary.agents) ? summary.agents : [];
      return {
        ok: summary.ok === true,
        ts: Number(summary.ts || Date.now()),
        durationMs: Number(summary.durationMs != null ? summary.durationMs : summary.duration_ms || 0),
        heartbeatSeconds: Number(summary.heartbeatSeconds != null ? summary.heartbeatSeconds : summary.heartbeat_seconds || 0),
        defaultAgentId: String(summary.defaultAgentId || summary.default_agent_id || ''),
        agent_count: Number(summary.agent_count || agents.length || 0),
        agents: agents
      };
    },

    async loadDashboardHealthSummary(force) {
      var now = Date.now();
      if (!force && this._healthSummaryLoading) return this._healthSummaryLoading;
      if (!force && this._healthSummaryLoadedAt && (now - Number(this._healthSummaryLoadedAt || 0)) < 15000) {
        return this.healthSummary;
      }
      var seq = Number(this._healthSummaryLoadSeq || 0) + 1;
      this._healthSummaryLoadSeq = seq;
      var self = this;
      this._healthSummaryLoading = (async function() {
        try {
          var payload = await InfringAPI.get('/api/health');
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(payload);
          self.healthSummaryError = '';
        } catch (e) {
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(null);
          self.healthSummaryError = String(e && e.message ? e.message : 'health_unavailable');
        } finally {
          if (seq === Number(self._healthSummaryLoadSeq || 0)) {
            self._healthSummaryLoadedAt = Date.now();
            self._healthSummaryLoading = null;
          }
        }
        return self.healthSummary;
      })();
      return this._healthSummaryLoading;
    },

    async pollStatus(opts) {
      var force = !!(opts && opts.force);
      if (this._pollStatusInFlight) {
        this._pollStatusQueued = true;
        return this._pollStatusInFlight;
      }
      var self = this;
      this._pollStatusInFlight = (async function() {
        var store = self.getAppStore();
        if (!store) {
          self.connected = false;
          self.connectionState = 'connecting';
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_retrying');
          return;
        }
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        if (typeof store.checkStatus === 'function') await store.checkStatus();
        if (typeof self.setBootProgressEvent === 'function') {
          self.setBootProgressEvent(
            store && store.connectionState === 'connected' ? 'status_connected' : 'status_retrying',
            { bootStage: store && store.bootStage }
          );
        }
        var shouldHydrateHealth = force || store.connectionState !== 'connected' || !store.runtimeSync;
        if (shouldHydrateHealth) await self.loadDashboardHealthSummary(store.connectionState !== 'connected');
        var now = Date.now();
        var shouldRefreshAgents =
          force ||
          !store.agentsHydrated ||
          (store.connectionState !== 'connected') ||
          (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
        if (shouldRefreshAgents) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_refresh_started');
          if (typeof store.refreshAgents === 'function') await store.refreshAgents();
        }
        if (store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_hydrated');
        }
        self.reconcileArchivedAgentIdsWithLiveAgents();
        if (typeof self.syncChatSidebarTopologyOrderFromAgents === 'function') {
          self.syncChatSidebarTopologyOrderFromAgents();
        }
        self.connected = store.connected;
        self.version = store.version;
        self.agentCount = store.agentCount;
        self.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
        self.queueConnectionIndicatorState(self.connectionState);
        self.wsConnected = InfringAPI.isWsConnected();
        if (!self.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
          await self.applyBootChatSelection();
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('selection_applied');
        }
        self.scheduleSidebarScrollIndicators();
        if (store.booting === false && store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('releasing', { bootStage: store.bootStage });
        }
        self.releaseBootSplash(false);
      })();
      try {
        await this._pollStatusInFlight;
      } finally {
        this._pollStatusInFlight = null;
        if (this._pollStatusQueued) {
          this._pollStatusQueued = false;
          window.setTimeout(function() { self.pollStatus({ force: true }); }, 0);
        }
      }
    }
  };
}
