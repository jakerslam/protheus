    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    collapsedAgentHover: {
      id: '',
      active: false,
      name: '',
      text: '',
      unread: false,
      top: 0
    },
    confirmArchiveAgentId: '',
    archivedAgentIds: (() => {
      try {
        var raw = localStorage.getItem('infring-archived-agent-ids');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id); });
      } catch(_) {
        return [];
      }
    })(),
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    connectionState: 'connecting',
    connectionIndicatorState: 'connecting',
    version: '0.1.0',
    agentCount: 0,
    bootSelectionApplied: false,
    clockTick: Date.now(),
    _themeSwitchReset: 0,
    _lastConnectionIndicatorAt: 0,
    _connectionIndicatorTimer: null,
    _pendingConnectionIndicatorState: '',
    sidebarHasOverflowAbove: false,
    sidebarHasOverflowBelow: false,
    chatSidebarHasOverflowAbove: false,
    chatSidebarHasOverflowBelow: false,
    _sidebarScrollIndicatorRaf: 0,
    _collapsedHoverSuppressedUntil: 0,
    _collapsedHoverNeedsPointerMove: false,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,

    normalizeConnectionIndicatorState(state) {
      var raw = String(state || '').trim().toLowerCase();
      if (raw === 'connected') return 'connected';
      if (raw === 'disconnected') return 'disconnected';
      return 'connecting';
    },

    queueConnectionIndicatorState(state) {
      var next = this.normalizeConnectionIndicatorState(state);
      var now = Date.now();
      var minIntervalMs = next === 'connecting' ? 1200 : 250;
      if (next !== 'connecting') {
        this.connectionIndicatorState = next;
        this._lastConnectionIndicatorAt = now;
        this._pendingConnectionIndicatorState = '';
        if (this._connectionIndicatorTimer) {
          clearTimeout(this._connectionIndicatorTimer);
          this._connectionIndicatorTimer = null;
        }
        return;
      }
      if (!this._lastConnectionIndicatorAt || (now - this._lastConnectionIndicatorAt) >= minIntervalMs) {
        this.connectionIndicatorState = next;
        this._lastConnectionIndicatorAt = now;
        this._pendingConnectionIndicatorState = '';
        if (this._connectionIndicatorTimer) {
          clearTimeout(this._connectionIndicatorTimer);
          this._connectionIndicatorTimer = null;
        }
        return;
      }
      this._pendingConnectionIndicatorState = next;
      if (this._connectionIndicatorTimer) return;
      var delay = Math.max(0, minIntervalMs - (now - this._lastConnectionIndicatorAt));
      var self = this;
      this._connectionIndicatorTimer = setTimeout(function() {
        self._connectionIndicatorTimer = null;
        var pending = self._pendingConnectionIndicatorState || next;
        self._pendingConnectionIndicatorState = '';
        self.connectionIndicatorState = self.normalizeConnectionIndicatorState(pending);
        self._lastConnectionIndicatorAt = Date.now();
      }, delay);
    },

    _computeScrollHintState(el) {
      if (!el) return { above: false, below: false };
      var scrollHeight = Number(el.scrollHeight || 0);
      var clientHeight = Number(el.clientHeight || 0);
      var scrollTop = Math.max(0, Number(el.scrollTop || 0));
