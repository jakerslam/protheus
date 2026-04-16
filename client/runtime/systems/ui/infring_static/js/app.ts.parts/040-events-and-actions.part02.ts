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
      if (agent && agent.revive_recommended === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return false;
      var thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var durationMs = Number(agent.contract_total_ms);
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      return null;
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent.revive_recommended === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return [true];
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) return [false, false, false, false, false];
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return [true];
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
      if (agent.revive_recommended === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent.revive_recommended === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
    },

    closeTaskbarHeroMenu() {
      this.taskbarHeroMenuOpen = false;
    },

    closeTaskbarTextMenu() {
      this.taskbarTextMenuOpen = '';
    },

    taskbarTextMenuIsOpen(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) return false;
      return String(this.taskbarTextMenuOpen || '').trim().toLowerCase() === key;
    },

    toggleTaskbarTextMenu(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) {
        this.closeTaskbarTextMenu();
        return;
      }
      this.closeTaskbarHeroMenu();
      this.taskbarTextMenuOpen = this.taskbarTextMenuIsOpen(key) ? '' : key;
    },

    handleTaskbarHelpManual() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('manual');
    },
    handleTaskbarHelpReportIssue() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('report');
    },
    async submitReportIssueDraft() {
      var draft = String(this.reportIssueDraft || '').trim();
      if (!draft) {
        InfringToast.error('Please add issue details before submitting.');
        return;
      }
      var entry = {
        id: 'issue-' + String(Date.now()),
        ts: Date.now(),
        text: draft,
        page: String(this.page || '').trim(),
        agent_id: String((this.currentAgent && this.currentAgent.id) || '').trim()
      };
      try {
        var raw = localStorage.getItem('infring-issue-report-drafts');
        var list = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(list)) list = [];
        list.unshift(entry);
        localStorage.setItem('infring-issue-report-drafts', JSON.stringify(list.slice(0, 25)));
      } catch(_) {}
      var title = ((draft.split(/\r?\n/).find(function(line) { return String(line || '').trim(); }) || draft).replace(/\s+/g, ' ').trim().slice(0, 120) || 'Dashboard issue report');
      var issueBody = '## User Report\n\n' + draft + '\n\n## Runtime Context\n- page: ' + (entry.page || 'unknown') + '\n- agent_id: ' + (entry.agent_id || 'none') + '\n- reported_at: ' + new Date(entry.ts || Date.now()).toISOString() + '\n- client_version: ' + String(this.version || 'unknown');
      try {
        var result = await InfringAPI.post('/api/dashboard/action', {
          action: 'dashboard.github.issue.create',
          payload: { title: title, body: issueBody, source: 'dashboard_report_popup' }
        });
        var lane = result && typeof result === 'object' ? (result.lane || result.payload || result) : {};
        if ((result && result.ok === false) || (lane && lane.ok === false)) {
          throw new Error(String((lane && (lane.error || lane.message)) || (result && (result.error || result.message)) || 'issue_submit_failed'));
        }
        var issueUrl = String((lane && (lane.html_url || lane.issue_url)) || '').trim();
        this.reportIssueDraft = ''; this.closePopupWindow('report');
        InfringToast.success(issueUrl ? ('Issue submitted: ' + issueUrl) : 'Issue submitted.');
      } catch (e) {
        InfringToast.error('Issue submit failed (saved locally): ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },
    manualDocumentMarkdown() {
      return [
        '# Infring Manual',
        '## Table of Contents\n1. [What Infring Is](#what-infring-is)\n2. [Install + Start](#install--start)\n3. [CLI Guide](#cli-guide)\n4. [UI Guide](#ui-guide)\n5. [Tools + Evidence](#tools--evidence)\n6. [Memory + Sessions](#memory--sessions)\n7. [Safety Model](#safety-model)\n8. [Troubleshooting](#troubleshooting)\n9. [Reporting Issues](#reporting-issues)',
        '## What Infring Is\nInfring is a governed agent runtime with a CLI and dashboard UI. It is built for auditable execution: requests, tool outcomes, and runtime state should be observable and explainable.',
        '## Install + Start\nWindows: run installer with `-Repair -Full` when shims drift.\nGeneral flow: start runtime, open dashboard, select/create an agent, send prompts, review outputs.',
        '## CLI Guide\n- `infring gateway` launches gateway/runtime controls.\n- `infring gateway status` checks health and readiness.\n- Use `Get-Command infring` (PowerShell) or `which infring` (POSIX) to confirm PATH resolution.',
        '## UI Guide\n- Taskbar: system actions, help, notifications, utility menus.\n- Sidebar: agent conversations + live previews.\n- Chat Map: fast navigation across long threads.\n- Chat Surface: prompts, tools, receipts, and runtime feedback.',
        '## Tools + Evidence\nTool calls produce structured cards and outcomes. Prefer evidence-backed responses: check tool status, outputs, and receipts before concluding.',
        '## Memory + Sessions\nAgents maintain session context; branches and sessions can diverge by task. Keep work scoped per session to avoid cross-thread confusion.',
        '## Safety Model\nInfring aims for fail-closed behavior in risky paths: explicit checks, policy-aware actions, and governed mutation paths.',
        '## Troubleshooting\nIf UI appears stalled: verify runtime health, refresh taskbar/runtime, then retry. If installs fail: rerun installer repair/full and validate command resolution.',
        '## Reporting Issues\nUse Help -> Report an issue. Include expected behavior, actual behavior, reproduction steps, and any relevant screenshots/log lines.',
      ].join('\n\n');
    },

    manualDocumentHtml() {
      var markdown = this.manualDocumentMarkdown();
      if (typeof renderMarkdown === 'function') {
        return renderMarkdown(markdown);
      }
      return escapeHtml(markdown);
    },

    toggleTaskbarHeroMenu() {
      if (this.taskbarHeroActionPending) return;
      if (!this.taskbarHeroMenuOpen) this.closeTaskbarTextMenu();
      this.taskbarHeroMenuOpen = !this.taskbarHeroMenuOpen;
    },

    requestTaskbarRefresh() {
      this.closeTaskbarHeroMenu();
      var appStore = this.getAppStore ? this.getAppStore() : null;
      if (appStore && typeof appStore.bumpTaskbarRefreshTurn === 'function') {
        appStore.bumpTaskbarRefreshTurn();
      }
      if (this._taskbarRefreshOverlayTimer) {
        clearTimeout(this._taskbarRefreshOverlayTimer);
        this._taskbarRefreshOverlayTimer = 0;
      }
      if (this._taskbarRefreshReloadTimer) {
        clearTimeout(this._taskbarRefreshReloadTimer);
        this._taskbarRefreshReloadTimer = 0;
      }
      var self = this;
      this._taskbarRefreshOverlayTimer = window.setTimeout(function() {
        self.bootSplashVisible = true;
        self._bootSplashStartedAt = Date.now();
        if (typeof self.resetBootProgress === 'function') self.resetBootProgress();
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        self._taskbarRefreshOverlayTimer = 0;
      }, 1000);
      this._taskbarRefreshReloadTimer = window.setTimeout(function() {
        self._taskbarRefreshReloadTimer = 0;
        try {
          window.location.reload();
        } catch (_) {
          try {
            window.location.href = window.location.href;
          } catch (_) {}
        }
      }, 1100);
    },

    async postTaskbarHeroSystemRoute(route, body, options) {
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

    async runTaskbarHeroCommand(action) {
      var actionKey = String(action || '').trim().toLowerCase();
      if (!actionKey || this.taskbarHeroActionPending) return;
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
      this.taskbarHeroActionPending = actionKey;
      try {
        var result = null;
        try {
          result = await this.postTaskbarHeroSystemRoute(legacyRoute, body, {
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
        this.closeTaskbarHeroMenu();
        if (actionKey === 'restart') {
          InfringToast.success('Restart requested');
          this.requestTaskbarRefresh();
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
          this.requestTaskbarRefresh();
        }
      } catch (e) {
        InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
      } finally {
        this.taskbarHeroActionPending = '';
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
