      this.mobileMenuOpen = false;
    },
    setTheme(mode) {
      this.beginInstantThemeFlip();
      this.themeMode = mode;
      localStorage.setItem('infring-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },
    beginInstantThemeFlip() {
      var self = this;
      var body = document && document.body ? document.body : null;
      if (!body) return;
      body.classList.add('theme-switching');
      // Force style flush so no-transition styles are applied before theme variables swap.
      void body.offsetHeight;
      if (this._themeSwitchReset) {
        clearTimeout(this._themeSwitchReset);
      }
      this._themeSwitchReset = window.setTimeout(function() {
        body.classList.remove('theme-switching');
        self._themeSwitchReset = 0;
      }, 260);
    },
    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },
    requestTopbarRefresh() {
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
    toggleSidebar() {
      var nextCollapsed = !this.sidebarCollapsed;
      var resolveMessagesHost = function() {
        var nodes = document.querySelectorAll('#messages');
        for (var ni = 0; ni < nodes.length; ni++) if (nodes[ni] && nodes[ni].offsetParent !== null) return nodes[ni];
        return nodes && nodes.length ? nodes[0] : null;
      };
      var captureMessageBottomAnchor = function() {
        var host = resolveMessagesHost();
        if (!host || host.offsetParent === null) return null;
        var hostRect = host.getBoundingClientRect();
        var input = document.getElementById('msg-input');
        var alignY = hostRect.bottom;
        if (input && input.offsetParent !== null) {
          var inputRect = input.getBoundingClientRect();
          if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
        }
        var rows = host.querySelectorAll('.chat-message-block .message[id]');
        var best = null;
        var bestDiff = Number.POSITIVE_INFINITY;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i];
          if (!row || row.offsetParent === null) continue;
          var rect = row.getBoundingClientRect();
          if (rect.bottom < (hostRect.top - 40) || rect.top > (hostRect.bottom + 40)) continue;
          var diff = Math.abs(rect.bottom - alignY);
          if (diff < bestDiff) { bestDiff = diff; best = row; }
        }
        return best && best.id ? { id: String(best.id) } : null;
      };
      if (nextCollapsed) this._sidebarChatAnchorForExpand = captureMessageBottomAnchor();
      this.sidebarCollapsed = nextCollapsed;
      localStorage.setItem('infring-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
      // Always clear stale hover preview when toggling sidebar state.
      this.hideCollapsedAgentHover();
      this._collapsedHoverNeedsPointerMove = !!nextCollapsed;
      // Prevent synthetic hover events during collapse animation from showing preview immediately.
      this._collapsedHoverSuppressedUntil = this.sidebarCollapsed ? (Date.now() + 700) : 0;
      if (!nextCollapsed) {
        var anchor = (this._sidebarChatAnchorForExpand && this._sidebarChatAnchorForExpand.id)
          ? this._sidebarChatAnchorForExpand
          : captureMessageBottomAnchor();
        this._sidebarChatAnchorForExpand = null;
        var passes = 4;
        var restoreAnchor = function() {
          var host = resolveMessagesHost();
          if (!host || host.offsetParent === null || !anchor || !anchor.id) return;
          var row = document.getElementById(anchor.id);
          if (!row || !host.contains(row) || row.offsetParent === null) return;
          var hostRect = host.getBoundingClientRect();
          var input = document.getElementById('msg-input');
          var alignY = hostRect.bottom;
          if (input && input.offsetParent !== null) {
            var inputRect = input.getBoundingClientRect();
            if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
          }
          var alignOffset = Math.max(0, Math.min(Math.max(0, Number(host.clientHeight || 0)), Math.round(alignY - hostRect.top)));
          var rowBottom = Number(row.offsetTop || 0) + Math.max(0, Number(row.offsetHeight || 0));
          var maxTop = Math.max(0, Number(host.scrollHeight || 0) - Math.max(0, Number(host.clientHeight || 0)));
          var nextTop = Math.max(0, Math.min(maxTop, Math.round(rowBottom - alignOffset)));
          host.scrollTop = nextTop;
          if (passes-- > 1 && typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
          try { host.dispatchEvent(new Event('scroll')); } catch (_) {}
        };
        if (typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
        else setTimeout(restoreAnchor, 0);
      }
      this.scheduleSidebarScrollIndicators();
    },
    clearCollapsedAgentHoverState() {
      this.collapsedAgentHover = {
        id: '',
        kind: 'agent',
        active: false,
        name: '',
        text: '',
        unread: false,
        top: 0
      };
    },
    updateCollapsedAgentHoverPosition(ev) {
      var top = 0;
      var pointerTop = ev && typeof ev.clientY === 'number' ? Number(ev.clientY) : NaN;
      if (Number.isFinite(pointerTop) && pointerTop > 0) {
        top = Math.round(pointerTop);
      } else if (ev && ev.currentTarget && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        top = Math.round(rect.top + (rect.height / 2));
      }
      if (!Number.isFinite(top) || top <= 0) return;
      var viewportHeight = Number(
        typeof window !== 'undefined' && window && window.innerHeight ? window.innerHeight : 0
      );
      top = Math.max(16, top);
      if (viewportHeight > 0) {
        top = Math.min(Math.max(16, viewportHeight - 16), top);
      }
      this.collapsedAgentHover = Object.assign({}, this.collapsedAgentHover || {}, { top: top });
    },
    handleCollapsedAgentHoverMove(agent, ev) {
      this.updateCollapsedAgentHoverPosition(ev);
      if (!this.sidebarCollapsed || !agent) return this.hideCollapsedAgentHover();
      var now = Date.now();
      if (this._collapsedHoverNeedsPointerMove) return;
      this._collapsedHoverPointerMovedAt = now;
      if (Number(this._collapsedHoverSuppressedUntil || 0) > now) return;
      var hover = this.collapsedAgentHover || {};
      if (!hover.active || String(hover.id || '') !== String(agent.id || '')) {
        this.showCollapsedAgentHover(agent, ev);
      }
    },
    handleCollapsedNavHoverMove(label, ev) {
      this.updateCollapsedAgentHoverPosition(ev);
      if (!this.sidebarCollapsed) return this.hideCollapsedAgentHover();
      var now = Date.now();
      if (this._collapsedHoverNeedsPointerMove) return;
      this._collapsedHoverPointerMovedAt = now;
      if (Number(this._collapsedHoverSuppressedUntil || 0) > now) return;
      var navLabel = String(label || '').trim();
      if (navLabel && navLabel.toLowerCase() === 'system') {
        this.hideCollapsedAgentHover();
        return;
      }
      if (!navLabel) {
        this.hideCollapsedAgentHover();
        return;
      }
      var hover = this.collapsedAgentHover || {};
      if (!hover.active || String(hover.kind || '') !== 'nav' || String(hover.name || '') !== navLabel) {
        this.showCollapsedNavHover(navLabel, ev);
      }
    },
    isCollapsedAgentHoverVisible(rawHover) {
      var hover = rawHover || {};
      if (!hover.active) return false;
      var movedAt = Number(this._collapsedHoverPointerMovedAt || 0);
      if (!Number.isFinite(movedAt) || movedAt <= 0) return false;
      if ((Date.now() - movedAt) > 1500) return false;
      if (!hover.top || Number(hover.top) <= 0) return false;
      if (String(hover.kind || '').toLowerCase() === 'nav') {
        return String(hover.name || '').trim().length > 0;
      }
      var rawId = String(hover.id || '').trim();
      if (!rawId) return false;
      if (!this._collapsedAgentIdHasSidebarRow(rawId)) return false;
      if (String(hover.name || '').trim().toLowerCase() === 'agent') return false;
      var hoverText = this._normalizeCollapsedAgentHoverText(String(hover.text || ''));
      if (!hoverText) return false;
      return true;
    },
    _normalizeCollapsedAgentHoverText(rawText) {
      var text = String(rawText || '').trim();
      if (!text) return '';
      if (this._isCollapsedHoverStatePlaceholderText(text)) return '';
      return text;
    },
    _isCollapsedHoverStatePlaceholderText(text) {
      var normalized = String(text || '').trim().toLowerCase();
      return normalized === 'no messages yet'
        || normalized === 'system events and terminal output'
        || normalized === 'no matching text'
        || normalized === 'agent';
    },
    _collapsedAgentIdHasSidebarRow(rawId) {
      var id = String(rawId || '').trim();
      if (!id) return false;
      if (String(id).toLowerCase() === 'system') return true;
      if (!Array.isArray(this.chatSidebarRows)) return false;
      for (var i = 0; i < this.chatSidebarRows.length; i++) {
        var row = this.chatSidebarRows[i];
        if (!row) continue;
        if (String(row.id || '').trim() === id) return true;
      }
      return false;
    },
    sanitizeCollapsedAgentHoverState() {
      var hover = this.collapsedAgentHover || {};
      var rawId = String(hover.id || '').trim();
      var kind = String(hover.kind || 'agent').toLowerCase();
      if (!hover.active || !rawId) {
        this.hideCollapsedAgentHover();
        return;
      }
      var currentTop = Number(hover.top || 0);
      if (!Number.isFinite(currentTop) || currentTop <= 0) {
        this.hideCollapsedAgentHover();
        return;
      }
      if (kind === 'nav') {
        var navName = String(hover.name || '').trim();
        if (navName && navName.toLowerCase() === 'system') {
          this.hideCollapsedAgentHover();
          return;
        }
        if (!navName) {
          this.hideCollapsedAgentHover();
          return;
        }
        this.collapsedAgentHover = Object.assign({}, this.collapsedAgentHover || {}, {
          kind: 'nav',
          id: rawId,
          name: navName,
          text: '',
          unread: false,
          top: currentTop
        });
        return;
      }
      if (!this._collapsedAgentIdHasSidebarRow(rawId)) {
        this.hideCollapsedAgentHover();
        return;
      }
      var isSystemThread = rawId.toLowerCase() === 'system';
      var previewAgent = isSystemThread
        ? { id: 'system', is_system_thread: true, name: 'System' }
        : this.chatSidebarRows.find(function(agent) {
            return String(agent && agent.id || '').trim() === rawId;
          }) || { id: rawId };
      var preview = this.chatSidebarPreview(previewAgent) || {};
      var previewText = this._normalizeCollapsedAgentHoverText(preview.text || '');
      if (!previewText) {
        this.hideCollapsedAgentHover();
        return;
      }
      var previewName = String(hover.name || previewAgent.name || (isSystemThread ? 'System' : rawId)).trim();
      if (!previewName || previewName.toLowerCase() === 'agent') {
        this.hideCollapsedAgentHover();
        return;
      }
      this.collapsedAgentHover = Object.assign({}, this.collapsedAgentHover || {}, {
        id: rawId,
        kind: 'agent',
        name: previewName,
        text: previewText,
        unread: !!preview.unread_response,
        top: currentTop
      });
    },
    showCollapsedAgentHover(agent, ev) {
      if (!this.sidebarCollapsed || !agent) return;
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (eventType !== 'mousemove' && eventType !== 'pointermove') return;
      if (ev && ev.isTrusted === false) return;
      this._collapsedHoverPointerMovedAt = Date.now();
      if (this._collapsedHoverNeedsPointerMove) return;
      if (Number(this._collapsedHoverSuppressedUntil || 0) > Date.now()) return;
      var rawId = String((agent && agent.id) || '').trim();
      var isSystemThread = agent.is_system_thread === true || rawId.toLowerCase() === 'system';
      if (!rawId && !isSystemThread) {
        this.hideCollapsedAgentHover();
        return;
      }
      var hoverId = isSystemThread ? 'system' : rawId;
      var rowEl = ev && ev.currentTarget ? ev.currentTarget : null;
      if (rowEl && typeof rowEl.getAttribute === 'function') {
        var rowAgentId = String(rowEl.getAttribute('data-agent-id') || '').trim();
        if (!rowAgentId || rowAgentId !== hoverId) {
          this.hideCollapsedAgentHover();
          return;
        }
      }
      this.updateCollapsedAgentHoverPosition(ev);
      var currentTop = Number((this.collapsedAgentHover && this.collapsedAgentHover.top) || 0);
      if (!Number.isFinite(currentTop) || currentTop <= 0) {
        this.hideCollapsedAgentHover();
        return;
      }
      var preview = this.chatSidebarPreview(Object.assign({}, agent, { id: hoverId, is_system_thread: isSystemThread })) || {};
      var previewText = this._normalizeCollapsedAgentHoverText(preview.text || '');
      if (!previewText) {
        this.hideCollapsedAgentHover();
        return;
      }
      if (!this._collapsedAgentIdHasSidebarRow(hoverId)) {
        this.hideCollapsedAgentHover();
        return;
      }
      var hoverName = String(agent.name || (isSystemThread ? 'System' : hoverId)).trim();
      if (!hoverName || hoverName.toLowerCase() === 'agent') {
        this.hideCollapsedAgentHover();
        return;
      }
      this.collapsedAgentHover = Object.assign({}, this.collapsedAgentHover || {}, {
        id: hoverId,
        kind: 'agent',
        active: true,
        name: hoverName,
        text: previewText,
        unread: !!preview.unread_response
      });
    },
    showCollapsedNavHover(label, ev) {
      if (!this.sidebarCollapsed) return;
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (eventType !== 'mousemove' && eventType !== 'pointermove') return;
      if (ev && ev.isTrusted === false) return;
      this._collapsedHoverPointerMovedAt = Date.now();
      if (this._collapsedHoverNeedsPointerMove) return;
      if (Number(this._collapsedHoverSuppressedUntil || 0) > Date.now()) return;
      var navLabel = String(label || '').trim();
      if (navLabel && navLabel.toLowerCase() === 'system') {
        this.hideCollapsedAgentHover();
        return;
      }
      if (!navLabel) {
        this.hideCollapsedAgentHover();
        return;
      }
      this.updateCollapsedAgentHoverPosition(ev);
      var currentTop = Number((this.collapsedAgentHover && this.collapsedAgentHover.top) || 0);
      if (!Number.isFinite(currentTop) || currentTop <= 0) {
        this.hideCollapsedAgentHover();
        return;
      }
      this.collapsedAgentHover = Object.assign({}, this.collapsedAgentHover || {}, {
        id: 'nav:' + navLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-'),
        kind: 'nav',
        active: true,
        name: navLabel,
        text: '',
        unread: false,
        top: currentTop
      });
    },
    hideCollapsedAgentHover() {
      this._collapsedHoverPointerMovedAt = 0;
      this.clearCollapsedAgentHoverState();
    },
    runtimeFacadeState() {
      var store = this.getAppStore();
      var conn = this.normalizeConnectionIndicatorState(
        this.connectionIndicatorState ||
        ((store && store.connectionState) || this.connectionState || '')
      );
      if (conn === 'connecting') return 'connecting';
      if (conn === 'disconnected') return 'down';
      return 'connected';
    },
    runtimeFacadeClass() {
      var state = this.runtimeFacadeState();
      if (state === 'connected') return 'health-ok';
      if (state === 'connecting') return 'health-connecting';
      return 'health-down';
    },
    runtimeFacadeLabel() {
      var state = this.runtimeFacadeState();
      if (state === 'connected') {
        var store = this.getAppStore();
        var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || this.agentCount || 0);
        return String(agents) + ' agents';
      }
      if (state === 'connecting') return 'Connecting...';
      return 'Disconnected';
    },
    runtimeFacadeDisplayLabel() {
      var label = String(this.runtimeFacadeLabel() || '').trim();
      if (!label) return '';
      return label.replace(/\s+agents?$/i, '');
    },
    runtimeResponseP95Ms() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return null;
      var facadeP95 = Number(runtime.facade_response_p95_ms);
      if (Number.isFinite(facadeP95) && facadeP95 > 0) return Math.round(facadeP95);
      var p95 = Number(runtime.receipt_latency_p95_ms);
      if (Number.isFinite(p95) && p95 > 0) return Math.round(p95);
      var p99 = Number(runtime.receipt_latency_p99_ms);
      if (Number.isFinite(p99) && p99 > 0) return Math.round(p99);
      return null;
    },
    runtimeConfidencePercent() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return 80;
      var facadeConfidence = Number(runtime.facade_confidence_percent);
      if (Number.isFinite(facadeConfidence) && facadeConfidence > 0) {
        return Math.max(10, Math.min(100, Math.round(facadeConfidence)));
      }

      var score = 100;
      var queueDepth = Number(runtime.queue_depth || 0);
      var stale = Number(runtime.cockpit_stale_blocks || 0);
      var gaps = Number(runtime.health_coverage_gap_count || 0);
      var conduitSignals = Number(runtime.conduit_signals || 0);
      var targetSignals = Math.max(1, Number(runtime.target_conduit_signals || 4));
      var benchmark = String(runtime.benchmark_sanity_cockpit_status || runtime.benchmark_sanity_status || 'unknown').toLowerCase();
      var spine = Number(runtime.spine_success_rate);

      if (queueDepth > 20) score -= Math.min(20, Math.floor((queueDepth - 20) / 2));
      if (stale > 0) score -= Math.min(20, stale * 2);
      if (gaps > 0) score -= Math.min(20, gaps * 6);
      if (conduitSignals < Math.max(3, Math.floor(targetSignals * 0.5))) score -= 12;
      if (benchmark === 'warn') score -= 8;
      if (benchmark === 'fail' || benchmark === 'error') score -= 20;
      if (Number.isFinite(spine)) {
        if (spine < 0.9) score -= 15;
        if (spine < 0.6) score -= 10;
      }

      score = Math.max(10, Math.min(100, Math.round(score)));
      return score;
    },
    runtimeEtaSeconds() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return 0;
      var facadeEta = Number(runtime.facade_eta_seconds);
      if (Number.isFinite(facadeEta) && facadeEta >= 0) {
        return Math.max(0, Math.min(300, Math.round(facadeEta)));
      }
      var queueDepth = Math.max(0, Number(runtime.queue_depth || 0));
      if (queueDepth <= 0) return 0;
      // Conservative client-side estimate for "Active" mode only.
      return Math.max(1, Math.min(300, Math.ceil(queueDepth / 8)));
    },
    runtimeFacadeDetail() {
      var state = this.runtimeFacadeState();
      var store = this.getAppStore();
      var bootStage = String((store && store.bootStage) || '').trim();
      var stageSuffix = bootStage ? (' · ' + bootStage.replace(/_/g, ' ')) : '';
      if (state === 'connecting') return 'Establishing runtime link' + stageSuffix;
      if (state === 'down') return 'Runtime unavailable' + stageSuffix;
      var response = this.runtimeResponseP95Ms();
      var confidence = this.runtimeConfidencePercent();
      var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || 0);
      var base = 'Response ' + (response != null ? (response + 'ms') : '—') + ' · Confidence ' + confidence + '%';
      if (store && store.statusDegraded) {
        return base + ' · Status degraded' + stageSuffix;
      }
      if (state === 'active') {
        var eta = this.runtimeEtaSeconds();
        return (eta > 0 ? ('ETA ~' + eta + 's · ') : '') + base;
      }
      return base + ' · ' + agents + ' agent(s)';
    },
    runtimeFacadeTitle() {
      return this.runtimeFacadeLabel();
    },
    topbarClockParts() {
      var tick = Number(this.clockTick || Date.now());
      var dt = new Date(tick);
      if (!Number.isFinite(dt.getTime())) dt = new Date();
      var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      var monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
      var dayName = dayNames[dt.getDay()] || '';
      var monthName = monthNames[dt.getMonth()] || '';
      var day = dt.getDate();
      var hours24 = dt.getHours();
      var minutes = dt.getMinutes();
      var suffix = hours24 >= 12 ? 'PM' : 'AM';
      var hours12 = hours24 % 12;
      if (hours12 === 0) hours12 = 12;
      var minuteText = minutes < 10 ? ('0' + minutes) : String(minutes);
      return {
        main: dayName + ' ' + monthName + ' ' + day + ' ' + hours12 + ':' + minuteText,
        meridiem: suffix
      };
    },
    topbarClockMainLabel() {
      return this.topbarClockParts().main;
    },
    topbarClockMeridiemLabel() {
      return this.topbarClockParts().meridiem;
    },
    topbarClockLabel() {
      var parts = this.topbarClockParts();
      return parts.main + ' ' + parts.meridiem;
    },
    toggleAgentChatsSidebar() {
      if (this.sidebarCollapsed) {
        this.sidebarCollapsed = false;
        localStorage.setItem('infring-sidebar', 'expanded');
      }
      this.hideCollapsedAgentHover();
      this._collapsedHoverNeedsPointerMove = false;
      this._collapsedHoverSuppressedUntil = 0;
      this.scheduleSidebarScrollIndicators();
    },
    closeAgentChatsSidebar() {
      if (this.chatSidebarMode !== 'default') {
        this.chatSidebarMode = 'default';
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
      }
      this.confirmArchiveAgentId = '';
      this.scheduleSidebarScrollIndicators();
    },
    async applyBootChatSelection() {
      if (this.bootSelectionApplied) return;
      var store = this.getAppStore();
      if (!store || store.agentsLoading || !store.agentsHydrated) {
        return;
      }
      var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
      if (!rows.length) {
        this.bootSelectionApplied = true;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
        else store.activeAgentId = null;
        this.navigate('chat');
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
        return;
      }
      var target = null;
      if (store.activeAgentId) {
        var saved = String(store.activeAgentId);
        target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
      }
      if (!target) {
        rows.sort(function(a, b) {
