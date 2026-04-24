    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    chatSidebarVisibleBase: 7,
    chatSidebarVisibleStep: 5,
    chatSidebarVisibleCount: 7,
    dashboardPopup: {
      id: '',
      active: false,
      source: '',
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: 0,
      top: 0,
      side: 'bottom',
      inline_away: 'right',
      block_away: 'bottom',
      compact: false
    },
    confirmArchiveAgentId: '',
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    connectionState: 'connecting',
    connectionIndicatorState: 'connecting',
    healthSummary: null,
    healthSummaryError: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    agentCount: 0,
    bootSelectionApplied: false,
    clockTick: Date.now(),
    _dashboardClockTimer: 0,
    _dashboardStatusTimer: 0,
    _dashboardVisibilityHandler: null,
    _themeSwitchReset: 0,
    _lastConnectionIndicatorAt: 0,
    _connectionIndicatorTimer: null,
    _pendingConnectionIndicatorState: '',
    _healthSummaryLoadedAt: 0,
    _healthSummaryLoading: null,
    _healthSummaryLoadSeq: 0,
    _pollStatusInFlight: null,
    _pollStatusQueued: false,
    sidebarHasOverflowAbove: false,
    sidebarHasOverflowBelow: false,
    chatSidebarHasOverflowAbove: false,
    chatSidebarHasOverflowBelow: false,
    _sidebarScrollIndicatorRaf: 0,
    _chatSidebarFlipDurationMs: 240,
    _chatSidebarFlipRaf: 0,
    _chatSidebarLastSnapshot: null,
    _dragSurfaceLockTransformMs: 500,
    _dragSurfaceVisualStates: {},
    chatSidebarDragActive: false,
    chatSidebarDragLeft: 0,
    chatSidebarDragTop: 0,
    _chatSidebarDragRowsCache: null,
    _chatSidebarDragRenderMaxRows: 10,
    _chatSidebarDragRenderRowHeight: 56,
    chatSidebarPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0;
    })(),
    chatSidebarPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.5;
    })(),
    chatSidebarPlacementTopPx: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
        if (Number.isFinite(raw)) return raw;
      } catch(_) {}
      return Number.NaN;
    })(),
    chatSidebarWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-sidebar-wall-lock')
          || localStorage.getItem('infring-chat-sidebar-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatSidebarMoveDurationMs: 280,
    _chatSidebarPointerActive: false,
    _chatSidebarPointerMoved: false,
    _chatSidebarPointerStartX: 0,
    _chatSidebarPointerStartY: 0,
    _chatSidebarPointerOriginLeft: 0,
    _chatSidebarPointerOriginTop: 0,
    _chatSidebarPointerLastX: 0,
    _chatSidebarPointerLastY: 0,
    _chatSidebarPointerLastAt: 0,
    _chatSidebarPointerVelocity: 0,
    _chatSidebarPointerMoveHandler: null,
    _chatSidebarPointerUpHandler: null,
    _sidebarToggleSuppressUntil: 0,
    chatMapDragActive: false,
    chatMapDragLeft: 0,
    chatMapDragTop: 0,
    chatMapPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 1;
    })(),
    chatMapPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.38;
    })(),
    chatMapWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-map-wall-lock')
          || localStorage.getItem('infring-chat-map-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatMapMoveDurationMs: 280,
    _chatMapPointerActive: false,
    _chatMapPointerMoved: false,
    _chatMapPointerStartX: 0,
    _chatMapPointerStartY: 0,
    _chatMapPointerOriginLeft: 0,
    _chatMapPointerOriginTop: 0,
    _chatMapPointerLastX: 0,
    _chatMapPointerLastY: 0,
    _chatMapPointerLastAt: 0,
    _chatMapPointerVelocity: 0,
    _chatMapPointerMoveHandler: null,
    _chatMapPointerUpHandler: null,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,
    bootProgressPercent: 6,
    bootProgressEvent: 'splash_visible',
    _bootProgressUpdatedAt: Date.now(),
    _taskbarRefreshOverlayTimer: 0,
    _taskbarRefreshReloadTimer: 0,
    taskbarHeroMenuOpen: false,
    taskbarTextMenuOpen: '',
    helpManualWindowOpen: false,
    reportIssueWindowOpen: false,
    reportIssueDraft: '',
    popupWindowPlacements: {
      manual: { left: null, top: null },
      report: { left: null, top: null }
    },
    popupWindowWallLocks: {
      manual: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-manual-wall-lock')
            || localStorage.getItem('infring-popup-window-manual-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })(),
      report: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-report-wall-lock')
            || localStorage.getItem('infring-popup-window-report-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })()
    },
    popupWindowDragActive: false,
    popupWindowDragKind: '',
    popupWindowDragLeft: 0,
    popupWindowDragTop: 0,
    popupWindowDragWallLock: '',
    _popupWindowMoveDurationMs: 260,
    _popupWindowPointerActive: false,
    _popupWindowPointerMoved: false,
    _popupWindowPointerStartX: 0,
    _popupWindowPointerStartY: 0,
    _popupWindowPointerOriginLeft: 0,
    _popupWindowPointerOriginTop: 0,
    _popupWindowPointerLastX: 0,
    _popupWindowPointerLastY: 0,
    _popupWindowPointerLastAt: 0,
    _popupWindowPointerVelocity: 0,
    _popupWindowPointerMoveHandler: null,
    _popupWindowPointerUpHandler: null,
    taskbarHeroActionPending: '',
    taskbarDockEdge: (() => {
      try {
        var raw = String(localStorage.getItem('infring-taskbar-dock-edge') || '').trim().toLowerCase();
        if (raw === 'bottom') return 'bottom';
      } catch(_) {}
      return 'top';
    })(),
    taskbarDockDragActive: false,
    taskbarDockDragY: 0,
    _taskbarDockPointerActive: false,
    _taskbarDockPointerMoved: false,
    _taskbarDockPointerStartX: 0,
    _taskbarDockPointerStartY: 0,
    _taskbarDockOriginY: 0,
    _taskbarDockPointerMoveHandler: null,
    _taskbarDockPointerUpHandler: null,
    taskbarReorderLeft: (() => {
      var defaults = ['nav_cluster'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-left');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarReorderRight: (() => {
      var defaults = ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-right');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarDragGroup: '',
    taskbarDragItem: '',
    taskbarDragStartOrder: [],
    _taskbarDragHoldTimer: 0,
    _taskbarDragHoldGroup: '',
    _taskbarDragHoldItem: '',
    _taskbarDragArmedGroup: '',
    _taskbarDragArmedItem: '',
    navBackStack: [],
    navForwardStack: [],
    _navCurrentPage: '',
    _navHistoryAction: '',
    _navHistoryCap: 48,

    appsIconBottomRowFill(index) {
      var idx = Number(index);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      idx = Math.floor(idx);
      var colors = Array.isArray(this.appsIconBottomRowColors) ? this.appsIconBottomRowColors : [];
      return String(colors[idx] || '#22c55e');
    },

    chatSidebarFlipDurationMs() {
      var raw = Number(this._chatSidebarFlipDurationMs || 240);
      if (!Number.isFinite(raw)) raw = 240;
      return Math.max(120, Math.min(420, Math.round(raw)));
    },

    readChatSidebarSnapshot() {
      var refs = this.$refs || {};
      var nav = refs.sidebarNav;
      if (!nav || typeof nav.querySelectorAll !== 'function') return null;
      var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
      var rects = {};
      var ids = [];
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-agent-id') || '').trim();
        if (!id || Object.prototype.hasOwnProperty.call(rects, id)) continue;
        var rect = node.getBoundingClientRect();
        rects[id] = {
          left: Number(rect.left || 0),
          top: Number(rect.top || 0)
        };
        ids.push(id);
      }
      return {
        order: ids.join('|'),
        scrollTop: Number(nav.scrollTop || 0),
        rects: rects
      };
    },

    animateChatSidebarFromSnapshot(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var refs = this.$refs || {};
      var nav = refs.sidebarNav;
      if (!nav || typeof nav.querySelectorAll !== 'function') return;
      var durationMs = this.chatSidebarFlipDurationMs();
      requestAnimationFrame(function() {
        var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || (node.classList && node.classList.contains('dragging'))) continue;
          var id = String(node.getAttribute('data-agent-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(snapshot.rects || {}, id)) continue;
          var from = snapshot.rects[id] || {};
          var rect = node.getBoundingClientRect();
          var dx = Number(from.left || 0) - Number(rect.left || 0);
          var dy = Number(from.top || 0) - Number(rect.top || 0);
          if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
          node.style.transition = 'none';
          node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
          void node.offsetHeight;
          node.style.transition = 'transform ' + durationMs + 'ms var(--ease-smooth)';
          node.style.transform = 'translate(0px, 0px)';
          (function(el) {
            window.setTimeout(function() {
              if (!el.classList.contains('dragging')) {
                el.style.transform = '';
              }
              el.style.transition = '';
            }, durationMs + 24);
          })(node);
        }
      });
    },

    maybeAnimateChatSidebarRows() {
      if (String(this.chatSidebarDragAgentId || '').trim()) {
        this._chatSidebarLastSnapshot = this.readChatSidebarSnapshot();
        return;
      }
      if (this._chatSidebarFlipRaf) return;
      var self = this;
      this._chatSidebarFlipRaf = requestAnimationFrame(function() {
        self._chatSidebarFlipRaf = 0;
        var current = self.readChatSidebarSnapshot();
        if (!current) {
          self._chatSidebarLastSnapshot = null;
          return;
        }
        var previous = self._chatSidebarLastSnapshot;
        self._chatSidebarLastSnapshot = current;
        if (!previous) return;
        if (Math.abs(Number(current.scrollTop || 0) - Number(previous.scrollTop || 0)) > 1) return;
        if (String(current.order || '') === String(previous.order || '')) return;
        self.animateChatSidebarFromSnapshot(previous);
      });
    },

    cleanupBottomDockDragGhost() {
      if (this._bottomDockGhostRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockGhostRaf); } catch(_) {}
      }
      if (this._bottomDockGhostCleanupTimer) {
        try { clearTimeout(this._bottomDockGhostCleanupTimer); } catch(_) {}
      }
      this._bottomDockGhostRaf = 0;
      this._bottomDockGhostCleanupTimer = 0;
      this._bottomDockGhostTargetX = 0;
      this._bottomDockGhostTargetY = 0;
      this._bottomDockGhostCurrentX = 0;
      this._bottomDockGhostCurrentY = 0;
      this._bottomDockDragBoundaries = [];
      this._bottomDockLastInsertionIndex = -1;
      this._bottomDockReorderLockUntil = 0;
      var node = this._bottomDockDragGhostEl;
      if (node && node.parentNode) {
        try { node.parentNode.removeChild(node); } catch(_) {}
      }
      this._bottomDockDragGhostEl = null;
      this._bottomDockRevealTargetDuringSettle = false;
    },

    setBottomDockGhostTarget(x, y) {
      var nextX = Number(x || 0);
      var nextY = Number(y || 0);
      var targetX = Number.isFinite(nextX) ? nextX : 0;
      var targetY = Number.isFinite(nextY) ? nextY : 0;
      this._bottomDockGhostTargetX = targetX;
      this._bottomDockGhostTargetY = targetY;
      this._bottomDockGhostCurrentX = targetX;
      this._bottomDockGhostCurrentY = targetY;
      var ghost = this._bottomDockDragGhostEl;
      if (!ghost) return;
      ghost.style.left = Math.round(targetX) + 'px';
      ghost.style.top = Math.round(targetY) + 'px';
    },

    dragSurfaceMoveDurationMs(rawValue, fallbackMs) {
      var fallback = Number(fallbackMs || 280);
      if (!Number.isFinite(fallback)) fallback = 280;
      fallback = Math.max(80, Math.round(fallback));
      var raw = Number(rawValue);
      if (!Number.isFinite(raw)) raw = fallback;
      return Math.max(80, Math.round(raw));
    },

    readBottomDockScale(el) {
      if (!el || typeof window === 'undefined' || typeof window.getComputedStyle !== 'function') {
        return 0.95;
      }
      try {
        var transform = String(window.getComputedStyle(el).transform || '').trim();
        if (!transform || transform === 'none') return 0.95;
        var matrix2d = transform.match(/^matrix\(([^)]+)\)$/);
        if (matrix2d && matrix2d[1]) {
          var parts2d = matrix2d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
          if (parts2d.length >= 2 && Number.isFinite(parts2d[0]) && Number.isFinite(parts2d[1])) {
            var scale2d = Math.sqrt((parts2d[0] * parts2d[0]) + (parts2d[1] * parts2d[1]));
            if (Number.isFinite(scale2d) && scale2d > 0.01) return scale2d;
          }
        }
        var matrix3d = transform.match(/^matrix3d\(([^)]+)\)$/);
        if (matrix3d && matrix3d[1]) {
          var parts3d = matrix3d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
          if (parts3d.length >= 1 && Number.isFinite(parts3d[0]) && parts3d[0] > 0.01) return parts3d[0];
        }
      } catch(_) {}
      return 0.95;
    },

    bootProgressClamped(rawPercent) {
      var next = Number(rawPercent);
      if (!Number.isFinite(next)) next = 0;
      return Math.max(0, Math.min(100, Math.round(next)));
    },

    resetBootProgress() {
      this.bootProgressPercent = 6;
      this.bootProgressEvent = 'splash_visible';
      this._bootProgressUpdatedAt = Date.now();
    },

    bootProgressFromBootStage(rawStage) {
      var stage = String(rawStage || '').trim().toLowerCase();
      if (!stage) return 38;
      if (
        stage === 'ready' ||
        stage === 'connected' ||
        stage === 'boot_complete' ||
        stage === 'runtime_ready'
      ) {
        return 70;
      }
      if (stage.indexOf('agent') >= 0) return 66;
      if (stage.indexOf('connect') >= 0) return 28;
      var isRecoveringStage = stage.indexOf('retry') >= 0; if (isRecoveringStage) return 24;
      if (stage.indexOf('unreachable') >= 0 || stage.indexOf('disconnected') >= 0) return 20;
      if (stage.indexOf('start') >= 0 || stage.indexOf('init') >= 0 || stage.indexOf('boot') >= 0) return 16;
      return 42;
    },

    setBootProgressPercent(rawPercent, opts) {
      var options = opts && typeof opts === 'object' ? opts : {};
      var next = this.bootProgressClamped(rawPercent);
      var current = this.bootProgressClamped(this.bootProgressPercent);
      var allowDecrease = options.allowDecrease === true;
      if (!allowDecrease && next < current) next = current;
      if (next === current) return;
      this.bootProgressPercent = next;
      this._bootProgressUpdatedAt = Date.now();
    },

    setBootProgressEvent(eventName, meta) {
      var event = String(eventName || '').trim().toLowerCase();
      if (!event) return;
      var target = 0;
      if (event === 'splash_visible') target = 6;
      else if (event === 'status_requesting') target = 18;
      else if (event === 'status_connected') target = 42;
      else if (event === 'status_retrying') target = 24;
      else if (event === 'agents_refresh_started') target = 56;
      else if (event === 'agents_hydrated') target = 76;
      else if (event === 'selection_applied') target = 90;
      else if (event === 'releasing') target = 97;
      else if (event === 'complete') target = 100;
      else target = 12;

      var stageTarget = this.bootProgressFromBootStage(meta && meta.bootStage);
      if (event === 'status_connected' || event === 'status_retrying') {
        target = Math.max(target, stageTarget);
      }
      if (event === 'complete') {
        this.setBootProgressPercent(100, { allowDecrease: true });
      } else {
        this.setBootProgressPercent(target);
      }
      this.bootProgressEvent = event;
    },
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
      var maxScroll = Math.max(0, scrollHeight - clientHeight);
      if (maxScroll <= 2) return { above: false, below: false };
      return {
        above: scrollTop > 2,
        below: (maxScroll - scrollTop) > 2
      };
    },
