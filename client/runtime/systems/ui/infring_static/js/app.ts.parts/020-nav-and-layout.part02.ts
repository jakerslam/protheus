    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    chatSidebarVisibleBase: 7,
    chatSidebarVisibleStep: 5,
    chatSidebarVisibleCount: 7,
    collapsedAgentHover: {
      id: '',
      kind: 'agent',
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
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
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
    _chatSidebarFlipDurationMs: 240,
    _chatSidebarFlipRaf: 0,
    _chatSidebarLastSnapshot: null,
    _collapsedHoverSuppressedUntil: 0,
    _collapsedHoverNeedsPointerMove: false,
    _collapsedHoverPointerMovedAt: 0,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,
    bootProgressPercent: 6,
    bootProgressEvent: 'splash_visible',
    _bootProgressUpdatedAt: Date.now(),
    _topbarRefreshOverlayTimer: 0,
    _topbarRefreshReloadTimer: 0,
    topbarDockEdge: (() => {
      try {
        var raw = String(localStorage.getItem('infring-topbar-dock-edge') || '').trim().toLowerCase();
        if (raw === 'bottom') return 'bottom';
      } catch(_) {}
      return 'top';
    })(),
    topbarDockDragActive: false,
    topbarDockDragY: 0,
    _topbarDockPointerActive: false,
    _topbarDockPointerMoved: false,
    _topbarDockPointerStartX: 0,
    _topbarDockPointerStartY: 0,
    _topbarDockOriginY: 0,
    _topbarDockPointerMoveHandler: null,
    _topbarDockPointerUpHandler: null,
    topbarReorderLeft: (() => {
      var defaults = ['nav_cluster', 'refresh'];
      try {
        var raw = localStorage.getItem('infring-topbar-order-left');
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
    topbarReorderRight: (() => {
      var defaults = ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      try {
        var raw = localStorage.getItem('infring-topbar-order-right');
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
    topbarDragGroup: '',
    topbarDragItem: '',
    topbarDragStartOrder: [],
    _topbarDragHoldTimer: 0,
    _topbarDragHoldGroup: '',
    _topbarDragHoldItem: '',
    _topbarDragArmedGroup: '',
    _topbarDragArmedItem: '',
    bottomDockOrder: (() => {
      var defaults = ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
      try {
        var raw = localStorage.getItem('infring-bottom-dock-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i++) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j++) {
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
    bottomDockTileConfig: {
      chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
      overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
      agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
      scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
      skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
      runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
      settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
    },
    appsIconBottomRowColors: (() => {
      var palette = ['#14b8a6', '#06b6d4', '#38bdf8', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#f43f5e', '#64748b'];
      var out = [];
      for (var i = 0; i < 3; i += 1) {
        out.push(palette[Math.floor(Math.random() * palette.length)]);
      }
      return out;
    })(),
    bottomDockDragId: '',
    bottomDockDragStartOrder: [],
    bottomDockDragCommitted: false,
    bottomDockHoverId: '',
    bottomDockHoverWeightById: {},
    bottomDockPointerX: 0,
    bottomDockPointerY: 0,
    bottomDockPreviewText: '',
    bottomDockPreviewMorphFromText: '',
    bottomDockPreviewHoverKey: '',
    bottomDockPreviewX: 0,
    bottomDockPreviewY: 0,
    bottomDockPreviewWidth: 0,
    bottomDockPreviewVisible: false,
    bottomDockPreviewLabelMorphing: false,
    bottomDockPreviewLabelFxReady: true,
    _bottomDockPreviewHideTimer: 0,
    _bottomDockPreviewReflowRaf: 0,
    _bottomDockPreviewReflowFrames: 0,
    _bottomDockPreviewWidthRaf: 0,
    _bottomDockPreviewLabelFxRaf: 0,
    _bottomDockPreviewLabelFxTimer: 0,
    _bottomDockPreviewLabelMorphTimer: 0,
    bottomDockClickAnimId: '',
    _bottomDockDragGhostEl: null,
    _bottomDockClickAnimTimer: 0,
    _bottomDockClickAnimDurationMs: 980,
    _bottomDockSuppressClickUntil: 0,
    _bottomDockPointerActive: false,
    _bottomDockPointerMoved: false,
    _bottomDockPointerCandidateId: '',
    _bottomDockPointerStartX: 0,
    _bottomDockPointerStartY: 0,
    _bottomDockPointerLastX: 0,
    _bottomDockPointerLastY: 0,
    _bottomDockPointerGrabOffsetX: 16,
    _bottomDockPointerGrabOffsetY: 16,
    _bottomDockDragGhostWidth: 32,
    _bottomDockDragGhostHeight: 32,
    _bottomDockPointerMoveHandler: null,
    _bottomDockPointerUpHandler: null,
    _bottomDockGhostTargetX: 0,
    _bottomDockGhostTargetY: 0,
    _bottomDockGhostCurrentX: 0,
    _bottomDockGhostCurrentY: 0,
    _bottomDockGhostRaf: 0,
    _bottomDockGhostCleanupTimer: 0,
    _bottomDockMoveDurationMs: 360,
    _bottomDockExpandedScale: 1.54,
    bottomDockRotationDeg: Number.NaN,
    _bottomDockRevealTargetDuringSettle: false,
    _bottomDockDragBoundaries: [],
    _bottomDockLastInsertionIndex: -1,
    _bottomDockReorderLockUntil: 0,
    bottomDockPlacementId: (() => {
      try {
        var raw = String(localStorage.getItem('infring-bottom-dock-placement') || '').trim().toLowerCase();
        var allowed = {
          left: true,
          center: true,
          right: true,
          'top-left': true,
          'top-center': true,
          'top-right': true,
          'left-top': true,
          'left-bottom': true,
          'right-top': true,
          'right-bottom': true
        };
        if (allowed[raw]) return raw;
        if (raw === 'left-center') return 'left-top';
        if (raw === 'right-center') return 'right-top';
      } catch(_) {}
      return 'center';
    })(),
    bottomDockSnapPoints: [
      { id: 'left', x: 0.16, y: 0.995, side: 'bottom' },
      { id: 'center', x: 0.50, y: 0.995, side: 'bottom' },
      { id: 'right', x: 0.84, y: 0.995, side: 'bottom' },
      { id: 'top-left', x: 0.16, y: 0.005, side: 'top' },
      { id: 'top-center', x: 0.50, y: 0.005, side: 'top' },
      { id: 'top-right', x: 0.84, y: 0.005, side: 'top' },
      { id: 'left-top', x: 0.005, y: (1 / 3), side: 'left' },
      { id: 'left-bottom', x: 0.005, y: (2 / 3), side: 'left' },
      { id: 'right-top', x: 0.995, y: (1 / 3), side: 'right' },
      { id: 'right-bottom', x: 0.995, y: (2 / 3), side: 'right' }
    ],
    bottomDockContainerDragActive: false,
    bottomDockContainerSettling: false,
    bottomDockContainerDragX: 0,
    bottomDockContainerDragY: 0,
    _bottomDockContainerPointerActive: false,
    _bottomDockContainerPointerMoved: false,
    _bottomDockContainerPointerStartX: 0,
    _bottomDockContainerPointerStartY: 0,
    _bottomDockContainerPointerLastX: 0,
    _bottomDockContainerPointerLastY: 0,
    _bottomDockContainerOriginX: 0,
    _bottomDockContainerOriginY: 0,
    _bottomDockContainerPointerMoveHandler: null,
    _bottomDockContainerPointerUpHandler: null,
    _bottomDockContainerSettleTimer: 0,
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

    bottomDockMoveDurationMs() {
      return this.dragSurfaceMoveDurationMs(this._bottomDockMoveDurationMs, 360);
    },

    bottomDockExpandedScale() {
      var raw = Number(this._bottomDockExpandedScale || 1.54);
      if (!Number.isFinite(raw) || raw <= 1) raw = 1.54;
      return raw;
    },

    bottomDockReadViewportSize() {
      var width = 0;
      var height = 0;
      try {
        width = Number(window && window.innerWidth || 0);
        height = Number(window && window.innerHeight || 0);
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) {
        width = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
      }
      if (!Number.isFinite(height) || height <= 0) {
        height = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(width) || width <= 0) width = 1440;
      if (!Number.isFinite(height) || height <= 0) height = 900;
      return { width: width, height: height };
    },

    bottomDockReadBaseSize() {
      var width = 0;
      var height = 0;
      try {
        var node = document && typeof document.querySelector === 'function'
          ? document.querySelector('.bottom-dock')
          : null;
        if (node) {
          width = Number(node.offsetWidth || 0);
          height = Number(node.offsetHeight || 0);
        }
      } catch(_) {
        width = 0;
        height = 0;
      }
      if (!Number.isFinite(width) || width <= 0) width = 420;
      if (!Number.isFinite(height) || height <= 0) height = 54;
      return { width: width, height: height };
    },

    bottomDockNormalizeSide(side) {
      var key = String(side || '').trim().toLowerCase();
      if (key === 'top' || key === 'left' || key === 'right') return key;
      return 'bottom';
    },

    bottomDockIsVerticalSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      return key === 'left' || key === 'right';
    },

    bottomDockRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return -90;
      if (key === 'right') return 90;
      return 0;
    },

    bottomDockIconRotationDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left') return 90;
      if (key === 'right') return -90;
      return 0;
    },

    bottomDockUpDegForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right' || key === 'top' || key === 'bottom') return 0;
      return 0;
    },

    bottomDockOrientation(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var horizontal = !this.bottomDockIsVerticalSide(side);
      var axis = horizontal ? 'x' : 'y';
      return {
        side: side,
        horizontal: horizontal,
        axis: axis,
        primarySign: 1,
        upDeg: Number(this.bottomDockUpDegForSide(side) || 0)
      };
    },

    bottomDockOppositeSide(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint);
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      if (side === 'top') return 'bottom';
      return 'top';
    },

    bottomDockWallSide() {
      return this.bottomDockNormalizeSide(this.bottomDockActiveSide());
    },

    bottomDockOpenSide() {
      return this.bottomDockOppositeSide(this.bottomDockWallSide());
    },

    bottomDockRotationDegResolved(sideHint) {
      var side = this.bottomDockNormalizeSide(sideHint || this.bottomDockActiveSide());
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = Number(this.bottomDockRotationDegForSide(side));
      }
      return Number(this.bottomDockNormalizeRotationDeg(rotationDeg) || 0);
    },

    bottomDockScreenDeltaToLocal(dx, dy, sideHint) {
      var screenDx = Number(dx || 0);
      var screenDy = Number(dy || 0);
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (rotationDeg * Math.PI) / 180;
      var cos = Math.cos(theta);
      var sin = Math.sin(theta);
      return {
        x: (screenDx * cos) + (screenDy * sin),
        y: (-screenDx * sin) + (screenDy * cos)
      };
    },

    bottomDockCanonicalRotationCandidatesForSide(side) {
      var key = this.bottomDockNormalizeSide(side);
      if (key === 'left' || key === 'right') return [90, -90];
      return [0];
    },

    bottomDockNormalizeRotationDeg(value) {
      var raw = Number(value);
      var canonical = [-90, 0, 90];
      if (!Number.isFinite(raw)) return 0;
      var best = canonical[0];
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < canonical.length; i += 1) {
        var candidate = canonical[i];
        var dist = Math.abs(raw - candidate);
        if (dist < bestDist) {
          bestDist = dist;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      var best = target;
      var bestDelta = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = Math.abs(candidate - current);
        if (delta < bestDelta) {
          bestDelta = delta;
          best = candidate;
        }
      }
      return best;
    },

    bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      var left = x < (Number(view.width || 0) * 0.5);
      var top = y < (Number(view.height || 0) * 0.5);
      // TL + BR => counterclockwise. TR + BL => clockwise.
      return (left === top) ? 'ccw' : 'cw';
    },

    bottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
      var current = Number(currentDeg);
      var target = Number(targetDeg);
      var dir = String(direction || '').trim().toLowerCase();
      if (!Number.isFinite(target)) target = 0;
      if (!Number.isFinite(current)) return target;
      if (dir !== 'cw' && dir !== 'ccw') {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      var best = null;
      var bestAbs = Number.POSITIVE_INFINITY;
      for (var k = -2; k <= 2; k += 1) {
        var candidate = target + (k * 360);
        var delta = candidate - current;
        if (dir === 'cw' && delta < 0) continue;
        if (dir === 'ccw' && delta > 0) continue;
        var absDelta = Math.abs(delta);
        if (absDelta < bestAbs) {
          bestAbs = absDelta;
          best = candidate;
        }
      }
      if (best === null) {
        return this.bottomDockResolveShortestRotationDeg(current, target);
      }
      return best;
    },

    bottomDockResolveRotationForSide(side, anchorX, anchorY) {
      var current = this.bottomDockNormalizeRotationDeg(this.bottomDockRotationDeg);
      var dir = this.bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
      var candidates = this.bottomDockCanonicalRotationCandidatesForSide(side);
      if (!Array.isArray(candidates) || !candidates.length) return current;
      var best = Number(candidates[0] || 0);
      var bestScore = Number.POSITIVE_INFINITY;
      var bestDeltaAbs = Number.POSITIVE_INFINITY;
      for (var i = 0; i < candidates.length; i += 1) {
        var target = Number(candidates[i] || 0);
        var delta = target - current;
        var deltaAbs = Math.abs(delta);
        var directionPenalty = 0;
        if (dir === 'cw' && delta < 0) directionPenalty = 0.35;
        if (dir === 'ccw' && delta > 0) directionPenalty = 0.35;
        var score = deltaAbs + directionPenalty;
        if (score < bestScore || (score === bestScore && deltaAbs < bestDeltaAbs)) {
          best = target;
          bestScore = score;
          bestDeltaAbs = deltaAbs;
        }
      }
      var chosenDelta = best - current;
      if (Math.abs(chosenDelta) > 90) {
        if (dir === 'cw') return current + 90;
        if (dir === 'ccw') return current - 90;
        return current + (chosenDelta > 0 ? 90 : -90);
      }
      return best;
    },

    bottomDockSnapDefinitions() {
      var source = Array.isArray(this.bottomDockSnapPoints) ? this.bottomDockSnapPoints : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < source.length; i += 1) {
        var row = source[i];
        if (!row || typeof row !== 'object') continue;
        var id = String(row.id || '').trim().toLowerCase();
        if (!id || seen[id]) continue;
        var nx = Number(row.x);
        var ny = Number(row.y);
        var side = this.bottomDockNormalizeSide(row.side);
        if (!Number.isFinite(nx)) nx = 0.5;
        if (!Number.isFinite(ny)) ny = 0.995;
        nx = Math.max(0, Math.min(1, nx));
        ny = Math.max(0, Math.min(1, ny));
        seen[id] = true;
        out.push({ id: id, x: nx, y: ny, side: side });
      }
      if (!out.length) {
        out.push({ id: 'center', x: 0.5, y: 0.995, side: 'bottom' });
      }
      return out;
    },

    bottomDockSnapDefinitionById(id) {
      var key = String(id || '').trim().toLowerCase();
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return null;
      for (var i = 0; i < defs.length; i += 1) {
        if (defs[i] && defs[i].id === key) return defs[i];
      }
      for (var j = 0; j < defs.length; j += 1) {
        if (defs[j] && defs[j].id === 'center') return defs[j];
      }
      return defs[0] || null;
    },

    bottomDockSideForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      return this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
    },

    bottomDockActiveSnapId() {
      if (this.bottomDockContainerDragActive) {
        var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
        return this.bottomDockNearestSnapId(anchor.x, anchor.y);
      }
      var snap = this.bottomDockSnapDefinitionById(this.bottomDockPlacementId);
      return String(snap && snap.id || 'center');
    },

    bottomDockActiveSide() {
      return this.bottomDockSideForSnapId(this.bottomDockActiveSnapId());
    },

    bottomDockClampDragAnchor(anchorX, anchorY) {
      var view = this.bottomDockReadViewportSize();
      var margin = 8;
      var minX = margin;
      var maxX = Number(view.width || 0) - margin;
      var minY = margin;
      var maxY = Number(view.height || 0) - margin;
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
      if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
      x = Math.max(minX, Math.min(maxX, x));
      y = Math.max(minY, Math.min(maxY, y));
      return { x: x, y: y };
    },

    bottomDockClampAnchor(anchorX, anchorY, sideOverride) {
      var view = this.bottomDockReadViewportSize();
      var dock = this.bottomDockReadBaseSize();
      var side = this.bottomDockNormalizeSide(sideOverride);
      var hoverScale = this.bottomDockExpandedScale();
      if (!Number.isFinite(hoverScale) || hoverScale < 1) hoverScale = 1;
      if (side === 'left' || side === 'right') hoverScale = 1;
      var margin = 8;
      var baseWidth = Math.max(20, Number(dock.width || 0) * hoverScale);
      var baseHeight = Math.max(20, Number(dock.height || 0) * hoverScale);
      var visualWidth = this.bottomDockIsVerticalSide(side) ? baseHeight : baseWidth;
      var visualHeight = this.bottomDockIsVerticalSide(side) ? baseWidth : baseHeight;
      var halfWidth = visualWidth / 2;
      var halfHeight = visualHeight / 2;
      var minX = margin;
      var maxX = Number(view.width || 0) - margin;
      var minY = margin;
      var maxY = Number(view.height || 0) - margin;
      if (side === 'bottom') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = visualHeight + margin;
        maxY = Number(view.height || 0) - margin;
      } else if (side === 'top') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = margin;
        maxY = Number(view.height || 0) - visualHeight - margin;
      } else if (side === 'left') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = halfHeight + margin;
        maxY = Number(view.height || 0) - halfHeight - margin;
      } else if (side === 'right') {
        minX = halfWidth + margin;
        maxX = Number(view.width || 0) - halfWidth - margin;
        minY = halfHeight + margin;
        maxY = Number(view.height || 0) - halfHeight - margin;
      }
      if (!Number.isFinite(minX) || !Number.isFinite(maxX) || maxX <= minX) {
        minX = margin;
        maxX = Number(view.width || 0) - margin;
      }
      if (!Number.isFinite(minY) || !Number.isFinite(maxY) || maxY <= minY) {
        minY = margin;
        maxY = Number(view.height || 0) - margin;
      }
      var x = Number(anchorX);
      var y = Number(anchorY);
      if (!Number.isFinite(x)) {
        if (side === 'left') x = minX;
        else if (side === 'right') x = maxX;
        else x = Number(view.width || 0) * 0.5;
      }
      if (!Number.isFinite(y)) {
        if (side === 'top') y = margin;
        else if (side === 'left' || side === 'right') y = Number(view.height || 0) * 0.5;
        else y = Number(view.height || 0) - margin;
      }
      x = Math.max(minX, Math.min(maxX, x));
      y = Math.max(minY, Math.min(maxY, y));
      return { x: x, y: y };
    },

    bottomDockAnchorForSnapId(id) {
      var snap = this.bottomDockSnapDefinitionById(id);
      var view = this.bottomDockReadViewportSize();
      var x = Number(view.width || 0) * Number(snap && snap.x || 0.5);
      var y = Number(view.height || 0) * Number(snap && snap.y || 0.995);
      var side = this.bottomDockNormalizeSide(snap && snap.side || 'bottom');
      return this.bottomDockClampAnchor(x, y, side);
    },

    bottomDockNearestSnapId(anchorX, anchorY) {
      var defs = this.bottomDockSnapDefinitions();
      if (!defs.length) return 'center';
      var anchor = this.bottomDockClampDragAnchor(anchorX, anchorY);
      var bestId = defs[0].id;
      var bestDist = Number.POSITIVE_INFINITY;
      for (var i = 0; i < defs.length; i += 1) {
        var row = defs[i];
        if (!row) continue;
        var snapAnchor = this.bottomDockAnchorForSnapId(row.id);
        var dx = Number(anchor.x || 0) - Number(snapAnchor.x || 0);
        var dy = Number(anchor.y || 0) - Number(snapAnchor.y || 0);
        var dist = (dx * dx) + (dy * dy);
        if (!Number.isFinite(dist)) continue;
        if (dist >= bestDist) continue;
        bestDist = dist;
        bestId = row.id;
      }
      return String(bestId || 'center');
    },

    persistBottomDockPlacement() {
      var key = String(this.bottomDockPlacementId || '').trim().toLowerCase();
      var snap = this.bottomDockSnapDefinitionById(key);
      this.bottomDockPlacementId = String(snap && snap.id || 'center');
      try {
        localStorage.setItem('infring-bottom-dock-placement', this.bottomDockPlacementId);
      } catch(_) {}
    },

    bottomDockContainerStyle() {
      var activeSnapId = this.bottomDockContainerDragActive
        ? this.bottomDockNearestSnapId(this.bottomDockContainerDragX, this.bottomDockContainerDragY)
        : this.bottomDockPlacementId;
      var side = this.bottomDockSideForSnapId(activeSnapId);
      var anchor = this.bottomDockContainerDragActive
        ? this.bottomDockClampAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY, side)
        : this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      var rotationDeg = Number(this.bottomDockRotationDeg);
      if (!Number.isFinite(rotationDeg)) {
        rotationDeg = this.bottomDockResolveRotationForSide(side, anchor.x, anchor.y);
        this.bottomDockRotationDeg = rotationDeg;
      }
      var upDeg = Number(this.bottomDockUpDegForSide(side) || 0);
      var tileRotationDeg = upDeg - Number(rotationDeg || 0);
      var iconRotationDeg = 0;
      var durationMs = this.bottomDockContainerDragActive ? 0 : this.bottomDockMoveDurationMs();
      return (
        '--bottom-dock-anchor-x:' + Math.round(Number(anchor.x || 0)) + 'px;' +
        '--bottom-dock-anchor-y:' + Math.round(Number(anchor.y || 0)) + 'px;' +
        '--bottom-dock-position-transition:' + Math.max(0, Math.round(Number(durationMs || 0))) + 'ms;' +
        '--bottom-dock-up-deg:' + Math.round(Number(upDeg || 0)) + 'deg;' +
        '--bottom-dock-rotation-deg:' + Math.round(Number(rotationDeg || 0)) + 'deg;' +
        '--bottom-dock-tile-rotation-deg:' + Math.round(Number(tileRotationDeg || 0)) + 'deg;' +
        '--bottom-dock-icon-rotation-deg:' + Math.round(Number(iconRotationDeg || 0)) + 'deg;'
      );
    },

    bindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler || this._bottomDockContainerPointerUpHandler) return;
      var self = this;
      this._bottomDockContainerPointerMoveHandler = function(ev) { self.handleBottomDockContainerPointerMove(ev); };
      this._bottomDockContainerPointerUpHandler = function(ev) { self.endBottomDockContainerPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true);
    },

    unbindBottomDockContainerPointerListeners() {
      if (this._bottomDockContainerPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockContainerPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockContainerPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockContainerPointerMoveHandler = null;
      this._bottomDockContainerPointerUpHandler = null;
    },

    startBottomDockContainerPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (target && typeof target.closest === 'function') {
        var tileNode = target.closest('.bottom-dock-btn[data-dock-id]');
        if (tileNode) return;
      }
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      this._bottomDockContainerSettleTimer = 0;
      this.bottomDockContainerSettling = false;
      var anchor = this.bottomDockAnchorForSnapId(this.bottomDockPlacementId);
      this._bottomDockContainerPointerActive = true;
      this._bottomDockContainerPointerMoved = false;
      this._bottomDockContainerPointerStartX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerStartY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = Number(ev.clientX || 0);
      this._bottomDockContainerPointerLastY = Number(ev.clientY || 0);
      this._bottomDockContainerOriginX = Number(anchor.x || 0);
      this._bottomDockContainerOriginY = Number(anchor.y || 0);
      this.bottomDockContainerDragX = Number(anchor.x || 0);
      this.bottomDockContainerDragY = Number(anchor.y || 0);
      this.bindBottomDockContainerPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerPointerMove(ev) {
      if (!this._bottomDockContainerPointerActive) return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      this._bottomDockContainerPointerLastX = nextX;
      this._bottomDockContainerPointerLastY = nextY;
      var movedX = Math.abs(nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._bottomDockContainerPointerStartY || 0));
      if (!this._bottomDockContainerPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._bottomDockContainerPointerMoved = true;
        this.bottomDockContainerDragActive = true;
        this.bottomDockHoverId = '';
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.cancelBottomDockPreviewReflow();
      }
      var candidateX = Number(this._bottomDockContainerOriginX || 0) + (nextX - Number(this._bottomDockContainerPointerStartX || 0));
      var candidateY = Number(this._bottomDockContainerOriginY || 0) + (nextY - Number(this._bottomDockContainerPointerStartY || 0));
      var anchor = this.bottomDockClampDragAnchor(candidateX, candidateY);
      this.bottomDockContainerDragX = Number(anchor.x || 0);
      this.bottomDockContainerDragY = Number(anchor.y || 0);
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      var side = this.bottomDockSideForSnapId(nearestId);
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(side, anchor.x, anchor.y);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endBottomDockContainerPointerDrag() {
      if (!this._bottomDockContainerPointerActive) return;
      this._bottomDockContainerPointerActive = false;
      this.unbindBottomDockContainerPointerListeners();
      if (!this._bottomDockContainerPointerMoved) {
        this.bottomDockContainerDragActive = false;
        this._bottomDockContainerPointerMoved = false;
        return;
      }
      var anchor = this.bottomDockClampDragAnchor(this.bottomDockContainerDragX, this.bottomDockContainerDragY);
      var nearestId = this.bottomDockNearestSnapId(anchor.x, anchor.y);
      this.bottomDockPlacementId = nearestId;
      this.bottomDockRotationDeg = this.bottomDockResolveRotationForSide(this.bottomDockSideForSnapId(nearestId), anchor.x, anchor.y);
      this.persistBottomDockPlacement();
      this.bottomDockContainerDragActive = false;
      this.bottomDockContainerSettling = true;
      this._bottomDockContainerPointerMoved = false;
      if (this._bottomDockContainerSettleTimer) {
        try { clearTimeout(this._bottomDockContainerSettleTimer); } catch(_) {}
      }
      var self = this;
      var settleMs = this.bottomDockMoveDurationMs() + 36;
      this._bottomDockContainerSettleTimer = window.setTimeout(function() {
        self._bottomDockContainerSettleTimer = 0;
        self.bottomDockContainerSettling = false;
      }, settleMs);
    },

    settleBottomDockDragGhost(dragId, done) {
      var finish = typeof done === 'function' ? done : function() {};
      var ghost = this._bottomDockDragGhostEl;
      if (!ghost || !document) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var key = String(dragId || '').trim();
      if (!key) {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var slot = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
      if (!slot || typeof slot.getBoundingClientRect !== 'function') {
        this.cleanupBottomDockDragGhost();
        finish();
        return;
      }
      var rect = slot.getBoundingClientRect();
      var durationMs = this.bottomDockMoveDurationMs();
      var targetWidth = Number(rect && rect.width ? rect.width : 0);
      var targetHeight = Number(rect && rect.height ? rect.height : 0);
      var slotStyle = null;
      if (!Number.isFinite(targetWidth) || targetWidth <= 0) {
        targetWidth = Number(ghost.offsetWidth || 32);
      }
      if (!Number.isFinite(targetHeight) || targetHeight <= 0) {
        targetHeight = Number(ghost.offsetHeight || 32);
      }
      var slotRadiusPx = 0;
      try {
        if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function') {
          slotStyle = window.getComputedStyle(slot);
        }
        var rawRadius = slotStyle ? String(slotStyle.borderTopLeftRadius || slotStyle.borderRadius || '') : '';
        var rawWidth = slotStyle ? String(slotStyle.width || '') : '';
        var parsedRadius = parseFloat(rawRadius);
        var parsedWidth = parseFloat(rawWidth);
        if (Number.isFinite(parsedRadius) && parsedRadius >= 0) {
          if (Number.isFinite(parsedWidth) && parsedWidth > 0) {
            slotRadiusPx = (parsedRadius / parsedWidth) * targetWidth;
          } else {
            slotRadiusPx = parsedRadius;
          }
        }
      } catch(_) {}
      if (!slotRadiusPx) {
        slotRadiusPx = Math.round((targetWidth / 32) * 11);
      }
      ghost.style.transition =
        'left ' + durationMs + 'ms var(--ease-smooth), ' +
        'top ' + durationMs + 'ms var(--ease-smooth), ' +
        'width ' + durationMs + 'ms var(--ease-smooth), ' +
        'height ' + durationMs + 'ms var(--ease-smooth), ' +
        'border-radius ' + durationMs + 'ms var(--ease-smooth), ' +
        'opacity ' + durationMs + 'ms var(--ease-smooth)';
      var targetX = Number(rect.left || 0) + ((Number(rect.width || 0) - targetWidth) / 2);
      var targetY = Number(rect.top || 0) + ((Number(rect.height || 0) - targetHeight) / 2);
      var self = this;
      var moveGhost = function() {
        if (slotStyle) {
          ghost.style.background = String(slotStyle.background || ghost.style.background || '');
          ghost.style.border = String(slotStyle.border || ghost.style.border || '');
          ghost.style.borderWidth = String(slotStyle.borderTopWidth || ghost.style.borderWidth || '');
          ghost.style.borderStyle = String(slotStyle.borderTopStyle || ghost.style.borderStyle || '');
          ghost.style.borderColor = String(slotStyle.borderColor || ghost.style.borderColor || '');
          ghost.style.boxShadow = String(slotStyle.boxShadow || ghost.style.boxShadow || '');
          ghost.style.color = String(slotStyle.color || ghost.style.color || '');
        }
        ghost.style.left = targetX + 'px';
        ghost.style.top = targetY + 'px';
        ghost.style.width = targetWidth + 'px';
        ghost.style.height = targetHeight + 'px';
        ghost.style.borderRadius = slotRadiusPx + 'px';
        ghost.style.setProperty(
          '--dock-ghost-scale',
          String(Math.max(0.8, Math.min(4, targetWidth / 32)))
        );
        ghost.style.opacity = '1';
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(moveGhost);
      else moveGhost();
      if (this._bottomDockGhostCleanupTimer) {
        try { clearTimeout(this._bottomDockGhostCleanupTimer); } catch(_) {}
      }
      this._bottomDockGhostCleanupTimer = window.setTimeout(function() {
        self._bottomDockRevealTargetDuringSettle = true;
        var settleHoldMs = 54;
        var completeSettle = function() {
          self._bottomDockGhostCleanupTimer = 0;
          finish();
          if (typeof requestAnimationFrame !== 'function') {
            self.cleanupBottomDockDragGhost();
            return;
          }
          requestAnimationFrame(function() {
            requestAnimationFrame(function() {
              self.cleanupBottomDockDragGhost();
            });
          });
        };
        self._bottomDockGhostCleanupTimer = window.setTimeout(completeSettle, settleHoldMs);
      }, durationMs + 40);
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
      if (stage.indexOf('retry') >= 0) return 24;
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

    topbarDockEdgeNormalized(raw) {
      var key = String(raw || '').trim().toLowerCase();
      return key === 'bottom' ? 'bottom' : 'top';
    },

    topbarPersistDockEdge() {
      this.topbarDockEdge = this.topbarDockEdgeNormalized(this.topbarDockEdge);
      try {
        localStorage.setItem('infring-topbar-dock-edge', this.topbarDockEdge);
      } catch(_) {}
    },

    topbarReadHeight() {
      if (typeof document === 'undefined') return 46;
      try {
        var node = document.querySelector('.global-topbar');
        var height = Number(node && node.offsetHeight || 0);
        if (Number.isFinite(height) && height > 0) return height;
      } catch(_) {}
      return 46;
    },

    topbarReadViewportHeight() {
      var h = 0;
      try { h = Number(window && window.innerHeight || 0); } catch(_) { h = 0; }
      if (!Number.isFinite(h) || h <= 0) {
        h = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
      }
      if (!Number.isFinite(h) || h <= 0) h = 900;
      return h;
    },

    topbarAnchorForDockEdge(edgeRaw) {
      var edge = this.topbarDockEdgeNormalized(edgeRaw);
      if (edge === 'bottom') {
        return Math.max(0, this.topbarReadViewportHeight() - this.topbarReadHeight());
      }
      return 0;
    },

    topbarClampDragY(yRaw) {
      var y = Number(yRaw);
      if (!Number.isFinite(y)) y = this.topbarAnchorForDockEdge(this.topbarDockEdge);
      var maxY = Math.max(0, this.topbarReadViewportHeight() - this.topbarReadHeight());
      return Math.max(0, Math.min(maxY, y));
    },

    topbarNearestDockEdge(yRaw) {
      var y = this.topbarClampDragY(yRaw);
      var topY = this.topbarAnchorForDockEdge('top');
      var bottomY = this.topbarAnchorForDockEdge('bottom');
      var topDist = Math.abs(y - topY);
      var bottomDist = Math.abs(y - bottomY);
      return bottomDist < topDist ? 'bottom' : 'top';
    },

    topbarContainerStyle() {
      var styles = [];
      if (this.page !== 'chat') {
        styles.push('background:transparent;border-bottom:none;box-shadow:none;-webkit-backdrop-filter:none;backdrop-filter:none;');
      }
      var transitionMs = this.topbarDockDragActive ? 0 : 220;
      styles.push('--topbar-dock-transition:' + Math.max(0, Math.round(Number(transitionMs || 0))) + 'ms;');
      if (this.topbarDockDragActive) {
        var y = this.topbarClampDragY(this.topbarDockDragY);
        styles.push('top:' + Math.round(Number(y || 0)) + 'px;bottom:auto;');
      } else if (this.topbarDockEdgeNormalized(this.topbarDockEdge) === 'bottom') {
        styles.push('top:auto;bottom:0;');
      } else {
        styles.push('top:0;bottom:auto;');
      }
      return styles.join('');
    },

    shouldIgnoreTopbarDockDragTarget(target) {
      if (!target || typeof target.closest !== 'function') return false;
      return Boolean(
        target.closest(
          'button, a, input, textarea, select, [role="button"], [draggable="true"], .topbar-reorder-item, .theme-switcher, .notif-wrap, .topbar-search-popup, .topbar-search-popup-anchor, .topbar-clock'
        )
      );
    },

    bindTopbarDockPointerListeners() {
      if (this._topbarDockPointerMoveHandler || this._topbarDockPointerUpHandler) return;
      var self = this;
      this._topbarDockPointerMoveHandler = function(ev) { self.handleTopbarDockPointerMove(ev); };
      this._topbarDockPointerUpHandler = function(ev) { self.endTopbarDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._topbarDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._topbarDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._topbarDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._topbarDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._topbarDockPointerUpHandler, true);
    },

    unbindTopbarDockPointerListeners() {
      if (this._topbarDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._topbarDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._topbarDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._topbarDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._topbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._topbarDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._topbarDockPointerUpHandler, true); } catch(_) {}
      }
      this._topbarDockPointerMoveHandler = null;
      this._topbarDockPointerUpHandler = null;
    },

    startTopbarDockPointerDrag(ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (String(this.topbarDragGroup || '').trim()) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreTopbarDockDragTarget(target)) return;
      this._topbarDockPointerActive = true;
      this._topbarDockPointerMoved = false;
      this._topbarDockPointerStartX = Number(ev.clientX || 0);
      this._topbarDockPointerStartY = Number(ev.clientY || 0);
      this._topbarDockOriginY = this.topbarAnchorForDockEdge(this.topbarDockEdge);
      this.topbarDockDragY = this._topbarDockOriginY;
      this.bindTopbarDockPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleTopbarDockPointerMove(ev) {
      if (!this._topbarDockPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      var movedX = Math.abs(x - Number(this._topbarDockPointerStartX || 0));
      var movedY = Math.abs(y - Number(this._topbarDockPointerStartY || 0));
      if (!this._topbarDockPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._topbarDockPointerMoved = true;
        this.topbarDockDragActive = true;
      }
      var candidateY = Number(this._topbarDockOriginY || 0) + (y - Number(this._topbarDockPointerStartY || 0));
      this.topbarDockDragY = this.topbarClampDragY(candidateY);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endTopbarDockPointerDrag() {
      if (!this._topbarDockPointerActive) return;
      this._topbarDockPointerActive = false;
      this.unbindTopbarDockPointerListeners();
      if (!this._topbarDockPointerMoved) {
        this.topbarDockDragActive = false;
        return;
      }
      this._topbarDockPointerMoved = false;
      this.topbarDockEdge = this.topbarNearestDockEdge(this.topbarDockDragY);
      this.topbarDockDragActive = false;
      this.topbarPersistDockEdge();
    },

    topbarReorderDefaults(group) {
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') return ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      return ['nav_cluster', 'refresh'];
    },

    topbarReorderStorageKey(group) {
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? 'infring-topbar-order-right' : 'infring-topbar-order-left';
    },

    topbarReorderOrderForGroup(group) {
      var key = String(group || '').trim().toLowerCase();
      return key === 'right' ? this.topbarReorderRight : this.topbarReorderLeft;
    },

    setTopbarReorderOrderForGroup(group, nextOrder) {
      var key = String(group || '').trim().toLowerCase();
      if (key === 'right') {
        this.topbarReorderRight = nextOrder;
        return;
      }
      this.topbarReorderLeft = nextOrder;
    },

    normalizeTopbarReorder(group, rawOrder) {
      var defaults = this.topbarReorderDefaults(group);
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i += 1) {
        var id = String(source[i] || '').trim();
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
    },

    persistTopbarReorder(group) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var normalized = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
      this.setTopbarReorderOrderForGroup(key, normalized);
      try {
        localStorage.setItem(this.topbarReorderStorageKey(key), JSON.stringify(normalized));
      } catch(_) {}
    },

    topbarReorderOrderIndex(group, item) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var itemId = String(item || '').trim();
      if (!itemId) return 999;
      var order = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
      var idx = order.indexOf(itemId);
      if (idx >= 0) return idx;
      var fallback = this.topbarReorderDefaults(key).indexOf(itemId);
      return fallback >= 0 ? fallback : 999;
    },

    topbarReorderItemStyle(group, item) {
      return 'order:' + this.topbarReorderOrderIndex(group, item);
    },

    topbarReorderItemRects(group) {
      if (typeof document === 'undefined') return {};
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var out = {};
      var box = null;
      try {
        box = document.querySelector('.topbar-reorder-box-' + key);
      } catch(_) {
        box = null;
      }
      if (!box || typeof box.querySelectorAll !== 'function') return out;
      var nodes = box.querySelectorAll('.topbar-reorder-item[data-topbar-item]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-topbar-item') || '').trim();
        if (!id || Object.prototype.hasOwnProperty.call(out, id)) continue;
        var rect = node.getBoundingClientRect();
        out[id] = { left: Number(rect.left || 0), top: Number(rect.top || 0) };
      }
      return out;
    },

    animateTopbarReorderFromRects(group, beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function' || typeof document === 'undefined') return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      requestAnimationFrame(function() {
        var box = null;
        try {
          box = document.querySelector('.topbar-reorder-box-' + key);
        } catch(_) {
          box = null;
        }
        if (!box || typeof box.querySelectorAll !== 'function') return;
        var nodes = box.querySelectorAll('.topbar-reorder-item[data-topbar-item]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-topbar-item') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var dx = Number(from.left || 0) - Number(rect.left || 0);
          var dy = Number(from.top || 0) - Number(rect.top || 0);
          if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
          node.style.transition = 'none';
          node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
          void node.offsetHeight;
          node.style.transition = 'transform 220ms var(--ease-smooth)';
          node.style.transform = 'translate(0px, 0px)';
          (function(el) {
            window.setTimeout(function() {
              if (!el.classList.contains('dragging')) el.style.transform = '';
              el.style.transition = '';
            }, 250);
          })(node);
        }
      });
    },

    applyTopbarReorder(group, dragItem, targetItem, preferAfter, animate) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var dragId = String(dragItem || '').trim();
      var targetId = String(targetItem || '').trim();
      if (!dragId || !targetId || dragId === targetId) return false;
      var current = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
      var fromIndex = current.indexOf(dragId);
      var toIndex = current.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (Boolean(preferAfter)) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var beforeRects = Boolean(animate) ? this.topbarReorderItemRects(key) : null;
      this.setTopbarReorderOrderForGroup(key, next);
      if (beforeRects) this.animateTopbarReorderFromRects(key, beforeRects);
      return true;
    },

    handleTopbarReorderPointerDown(group, ev) {
      if (String(this.topbarDragGroup || '').trim()) return;
      if (!ev || Number(ev.button) !== 0) return;
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!item) return;
      this.cancelTopbarDragHold();
      this._topbarDragHoldGroup = key;
      this._topbarDragHoldItem = item;
      var self = this;
      if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        this._topbarDragHoldTimer = window.setTimeout(function() {
          self._topbarDragHoldTimer = 0;
          self._topbarDragArmedGroup = key;
          self._topbarDragArmedItem = item;
        }, 180);
      }
    },

    cancelTopbarDragHold() {
      if (this._topbarDragHoldTimer) {
        try { clearTimeout(this._topbarDragHoldTimer); } catch(_) {}
      }
      this._topbarDragHoldTimer = 0;
      this._topbarDragHoldGroup = '';
      this._topbarDragHoldItem = '';
      if (!String(this.topbarDragGroup || '').trim()) {
        this._topbarDragArmedGroup = '';
        this._topbarDragArmedItem = '';
      }
    },

    forceTopbarMoveDragEffect(ev) {
      if (!ev || !ev.dataTransfer) return;
      try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
      try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
    },

    setTopbarDragBodyActive(active) {
      if (typeof document === 'undefined' || !document.body || !document.body.classList) return;
      if (active) {
        document.body.classList.add('topbar-drag-active');
      } else {
        document.body.classList.remove('topbar-drag-active');
      }
    },

    handleTopbarReorderDragStart(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var item = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!item || this._topbarDragArmedGroup !== key || this._topbarDragArmedItem !== item) {
        if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
        return;
      }
      this.topbarDragGroup = key;
      this.topbarDragItem = item;
      this.topbarDragStartOrder = this.normalizeTopbarReorder(key, this.topbarReorderOrderForGroup(key));
      this._topbarDragArmedGroup = '';
      this._topbarDragArmedItem = '';
      this.cancelTopbarDragHold();
      if (ev && ev.dataTransfer) {
        this.forceTopbarMoveDragEffect(ev);
        try { ev.dataTransfer.setData('application/x-infring-topbar', key + ':' + item); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key + ':' + item); } catch(_) {}
        try {
          if (
            typeof document !== 'undefined' &&
            document.body &&
            typeof ev.dataTransfer.setDragImage === 'function'
          ) {
            var ghost = target && typeof target.cloneNode === 'function'
              ? target.cloneNode(true)
              : document.createElement('span');
            ghost.style.position = 'fixed';
            ghost.style.left = '-9999px';
            ghost.style.top = '-9999px';
            ghost.style.margin = '0';
            ghost.style.pointerEvents = 'none';
            ghost.style.transform = 'none';
            ghost.style.opacity = '1';
            if (ghost.classList && ghost.classList.contains('dragging')) {
              ghost.classList.remove('dragging');
            }
            var rect = target && typeof target.getBoundingClientRect === 'function'
              ? target.getBoundingClientRect()
              : null;
            var offsetX = 0;
            var offsetY = 0;
            if (rect) {
              var width = Math.max(1, Math.round(Number(rect.width || 0)));
              var height = Math.max(1, Math.round(Number(rect.height || 0)));
              ghost.style.width = width + 'px';
              ghost.style.height = height + 'px';
              ghost.style.boxSizing = 'border-box';
              if (typeof ev.clientX === 'number') {
                offsetX = Math.round(Math.max(0, Math.min(width, ev.clientX - rect.left)));
              }
              if (typeof ev.clientY === 'number') {
                offsetY = Math.round(Math.max(0, Math.min(height, ev.clientY - rect.top)));
              }
            } else {
              ghost.style.width = '1px';
              ghost.style.height = '1px';
            }
            document.body.appendChild(ghost);
            ev.dataTransfer.setDragImage(ghost, offsetX, offsetY);
            window.setTimeout(function() {
              if (ghost.parentNode) ghost.parentNode.removeChild(ghost);
            }, 0);
          }
        } catch(_) {}
      }
      if (target && target.classList) target.classList.add('dragging');
      this.setTopbarDragBodyActive(true);
    },

    handleTopbarReorderDragMove(ev) {
      this.forceTopbarMoveDragEffect(ev);
    },

    handleTopbarReorderDragEnter(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTopbarMoveDragEffect(ev);
    },

    handleTopbarReorderDragOver(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.forceTopbarMoveDragEffect(ev);
      var dragItem = String(this.topbarDragItem || '').trim();
      if (!dragItem) return;
      var target = ev && ev.target && typeof ev.target.closest === 'function'
        ? ev.target.closest('.topbar-reorder-item[data-topbar-item]')
        : null;
      var targetItem = target ? String(target.getAttribute('data-topbar-item') || '').trim() : '';
      if (!targetItem || targetItem === dragItem) return;
      var preferAfter = false;
      if (target && typeof target.getBoundingClientRect === 'function') {
        var rect = target.getBoundingClientRect();
        var midX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        preferAfter = Number(ev && ev.clientX || 0) >= midX;
      }
      this.applyTopbarReorder(key, dragItem, targetItem, preferAfter, true);
    },

    handleTopbarReorderDrop(group, ev) {
      var key = String(group || '').trim().toLowerCase();
      if (key !== 'right') key = 'left';
      if (String(this.topbarDragGroup || '').trim() !== key) return;
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
      this.persistTopbarReorder(key);
      this.topbarDragGroup = '';
      this.topbarDragItem = '';
      this.topbarDragStartOrder = [];
      this.cancelTopbarDragHold();
      this.setTopbarDragBodyActive(false);
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.topbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

    handleTopbarDragEnd() {
      var key = String(this.topbarDragGroup || '').trim();
      if (key) this.persistTopbarReorder(key);
      this.topbarDragGroup = '';
      this.topbarDragItem = '';
      this.topbarDragStartOrder = [];
      this.cancelTopbarDragHold();
      this.setTopbarDragBodyActive(false);
      if (typeof document !== 'undefined') {
        try {
          var draggingNodes = document.querySelectorAll('.topbar-reorder-item.dragging');
          for (var i = 0; i < draggingNodes.length; i += 1) {
            draggingNodes[i].classList.remove('dragging');
          }
        } catch(_) {}
      }
    },

    bottomDockDefaultOrder() {
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      if (registry) {
        var ids = Object.keys(registry);
        if (ids.length) return ids;
      }
      return ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
    },

    bottomDockTileConfigById(id) {
      var key = String(id || '').trim();
      if (!key) return null;
      var registry = (this.bottomDockTileConfig && typeof this.bottomDockTileConfig === 'object')
        ? this.bottomDockTileConfig
        : null;
      var tile = registry && Object.prototype.hasOwnProperty.call(registry, key) ? registry[key] : null;
      return tile && typeof tile === 'object' ? tile : null;
    },

    bottomDockTileData(id, field, fallback) {
      var key = String(field || '').trim();
      var tile = this.bottomDockTileConfigById(id);
      var value = (key && tile && Object.prototype.hasOwnProperty.call(tile, key)) ? tile[key] : fallback;
      return (value === undefined || value === null) ? String(fallback || '') : String(value);
    },

    bottomDockTileAnimationName(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      var name = animation ? String(animation[0] || '').trim() : '';
      return name || 'none';
    },

    bottomDockTileAnimationDurationAttr(id) {
      var tile = this.bottomDockTileConfigById(id);
      var animation = tile && Array.isArray(tile.animation) ? tile.animation : null;
      if (!animation) return null;
      var durationMs = Number(animation[1]);
      if (!Number.isFinite(durationMs) || durationMs < 120) return null;
      return String(Math.round(durationMs));
    },

    bottomDockSlotStyle(id) {
      var key = String(id || '').trim();
      var order = key ? this.bottomDockOrderIndex(key) : 999;
      var weight = this.bottomDockHoverWeight(key);
      if (!Number.isFinite(weight) || weight < 0) weight = 0;
      if (weight > 1) weight = 1;
      return 'order:' + order + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
    },

    bottomDockTileStyle(id) {
      var key = String(id || '').trim();
      var tile = this.bottomDockTileConfigById(key);
      var style = tile && typeof tile.style === 'string' ? String(tile.style || '').trim() : '';
      return style || '';
    },

    normalizeBottomDockOrder(rawOrder) {
      var defaults = this.bottomDockDefaultOrder();
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i++) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j++) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistBottomDockOrder() {
      this.bottomDockOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      try {
        localStorage.setItem('infring-bottom-dock-order', JSON.stringify(this.bottomDockOrder));
      } catch(_) {}
    },

    bottomDockOrderIndex(id) {
      var key = String(id || '').trim();
      if (!key) return 999;
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var idx = order.indexOf(key);
      if (idx >= 0) return idx;
      var fallback = this.bottomDockDefaultOrder().indexOf(key);
      return fallback >= 0 ? fallback : 999;
    },

    bottomDockAxisBasis(sideHint) {
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (Number(rotationDeg || 0) * Math.PI) / 180;
      var ux = Math.cos(theta);
      var uy = Math.sin(theta);
      if (Math.abs(ux) < 0.0001) ux = 0;
      if (Math.abs(uy) < 0.0001) uy = 0;
      return { ux: ux, uy: uy, vx: -uy, vy: ux };
    },

    bottomDockProjectPointToAxis(x, y, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var ux = Number(axis.ux || 0);
      var uy = Number(axis.uy || 0);
      var vx = Number(axis.vx || (-uy));
      var vy = Number(axis.vy || ux);
      var px = Number(x || 0);
      var py = Number(y || 0);
      return {
        primary: (px * ux) + (py * uy),
        secondary: (px * vx) + (py * vy)
      };
    },

    bottomDockAxisHalfExtent(width, height, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var w = Number(width || 0);
      var h = Number(height || 0);
      if (!Number.isFinite(w) || w < 0) w = 0;
      if (!Number.isFinite(h) || h < 0) h = 0;
      var ux = Math.abs(Number(axis.ux || 0));
      var uy = Math.abs(Number(axis.uy || 0));
      var vx = Math.abs(Number(axis.vx || 0));
      var vy = Math.abs(Number(axis.vy || 0));
      return {
        primary: ((ux * w) + (uy * h)) / 2,
        secondary: ((vx * w) + (vy * h)) / 2
      };
    },

    bottomDockProjectedRectBounds(rect, basis) {
      if (!rect) return null;
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var left = Number(rect.left || 0);
      var top = Number(rect.top || 0);
      var right = Number(rect.right || left);
      var bottom = Number(rect.bottom || top);
      var p1 = this.bottomDockProjectPointToAxis(left, top, axis);
      var p2 = this.bottomDockProjectPointToAxis(right, top, axis);
      var p3 = this.bottomDockProjectPointToAxis(left, bottom, axis);
      var p4 = this.bottomDockProjectPointToAxis(right, bottom, axis);
      var primaryMin = Math.min(p1.primary, p2.primary, p3.primary, p4.primary);
      var primaryMax = Math.max(p1.primary, p2.primary, p3.primary, p4.primary);
      var secondaryMin = Math.min(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      var secondaryMax = Math.max(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      return {
        primaryMin: primaryMin,
        primaryMax: primaryMax,
        secondaryMin: secondaryMin,
        secondaryMax: secondaryMax
      };
    },

    bottomDockButtonRects() {
      var out = {};
      var root = document.querySelector('.bottom-dock');
      if (!root) return out;
      var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-dock-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var width = Number(rect.width || 0);
        var height = Number(rect.height || 0);
        var left = Number(rect.left || 0);
        var top = Number(rect.top || 0);
        out[id] = {
          left: left,
          top: top,
          width: width,
          height: height,
          cx: left + (width / 2),
          cy: top + (height / 2)
        };
      }
      return out;
    },

    animateBottomDockFromRects(beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var durationMs = this.bottomDockMoveDurationMs();
      var self = this;
      requestAnimationFrame(function() {
        var root = document.querySelector('.bottom-dock');
        if (!root) return;
        var rootScale = self.readBottomDockScale(root);
        if (!Number.isFinite(rootScale) || rootScale <= 0.01) rootScale = 1;
        var side = self.bottomDockActiveSide();
        var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i++) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var fromCx = Number(from.cx);
          var fromCy = Number(from.cy);
          if (!Number.isFinite(fromCx)) fromCx = Number(from.left || 0) + (Number(from.width || 0) / 2);
          if (!Number.isFinite(fromCy)) fromCy = Number(from.top || 0) + (Number(from.height || 0) / 2);
          var toCx = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
          var toCy = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
          var screenDx = Number(fromCx || 0) - Number(toCx || 0);
          var screenDy = Number(fromCy || 0) - Number(toCy || 0);
          if (Math.abs(screenDx) < 0.5 && Math.abs(screenDy) < 0.5) continue;
          var localDelta = self.bottomDockScreenDeltaToLocal(screenDx, screenDy, side);
          var tx = Number(localDelta.x || 0) / rootScale;
          var ty = Number(localDelta.y || 0) / rootScale;
          if (Math.abs(tx) < 0.25 && Math.abs(ty) < 0.25) continue;
          node.style.setProperty('--dock-reorder-transition', '0ms');
          node.style.setProperty('--dock-reorder-translate-x', Math.round(tx) + 'px');
          node.style.setProperty('--dock-reorder-translate-y', Math.round(ty) + 'px');
          void node.offsetHeight;
          node.style.setProperty('--dock-reorder-transition', Math.max(0, Math.round(durationMs)) + 'ms');
          node.style.setProperty('--dock-reorder-translate-x', '0px');
          node.style.setProperty('--dock-reorder-translate-y', '0px');
          (function(el) {
            window.setTimeout(function() {
              if (
                !el.classList.contains('dragging') &&
                !el.classList.contains('hovered') &&
                !el.classList.contains('neighbor-hover') &&
                !el.classList.contains('second-neighbor-hover')
              ) {
                el.style.removeProperty('--dock-reorder-translate-x');
                el.style.removeProperty('--dock-reorder-translate-y');
              }
              el.style.removeProperty('--dock-reorder-transition');
            }, durationMs + 30);
          })(node);
        }
      });
    },

    setBottomDockHover(id) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      this.bottomDockHoverId = key;
      if (this._bottomDockPreviewHideTimer) {
        try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        this._bottomDockPreviewHideTimer = 0;
      }
      if (!Number.isFinite(this.bottomDockPointerX) || this.bottomDockPointerX <= 0) {
        try {
          var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
          if (slot && typeof slot.getBoundingClientRect === 'function') {
            var slotRect = slot.getBoundingClientRect();
            this.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
            this.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
          }
        } catch(_) {}
      }
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    clearBottomDockHover(id) {
      if (id) return;
      this.bottomDockHoverId = '';
      if (!this.bottomDockHoverId) {
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.cancelBottomDockPreviewReflow();
        var self = this;
        if (this._bottomDockPreviewHideTimer) {
          try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        }
        this._bottomDockPreviewHideTimer = window.setTimeout(function() {
          self._bottomDockPreviewHideTimer = 0;
          if (!String(self.bottomDockHoverId || '').trim()) {
            self.bottomDockPreviewVisible = false;
            self.bottomDockPreviewText = '';
            self.bottomDockPreviewMorphFromText = '';
            self.bottomDockPreviewLabelMorphing = false;
            self.bottomDockPreviewWidth = 0;
          }
        }, 40);
        return;
      }
      this.syncBottomDockPreview();
    },

    readBottomDockSlotCenters() {
      var out = [];
      if (typeof document === 'undefined') return out;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.querySelectorAll !== 'function') return out;
      var nodes = root.querySelectorAll('.dock-tile-slot[data-dock-slot-id]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getAttribute !== 'function' || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-dock-slot-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var centerX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        var centerY = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
        if (!Number.isFinite(centerX) || !Number.isFinite(centerY)) continue;
        out.push({ id: id, centerX: centerX, centerY: centerY });
      }
      return out;
    },

    bottomDockWeightForDistance(distancePx) {
      var d = Math.abs(Number(distancePx || 0));
      if (!Number.isFinite(d)) return 0;
      var sigma = 52;
      var exponent = -((d * d) / (2 * sigma * sigma));
      var weight = Math.exp(exponent);
      if (!Number.isFinite(weight) || weight < 0.008) return 0;
      if (weight > 1) return 1;
      return weight;
    },

    refreshBottomDockHoverWeights() {
      var side = this.bottomDockActiveSide();
      var vertical = this.bottomDockIsVerticalSide(side);
      var primaryPointer = vertical
        ? Number(this.bottomDockPointerY || 0)
        : Number(this.bottomDockPointerX || 0);
      if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var centers = this.readBottomDockSlotCenters();
      if (!centers.length) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var nearestId = '';
      var nearestDistance = Number.POSITIVE_INFINITY;
      var weights = {};
      for (var i = 0; i < centers.length; i += 1) {
        var item = centers[i];
        if (!item || !item.id) continue;
        var anchor = vertical ? Number(item.centerY || 0) : Number(item.centerX || 0);
        var dist = Math.abs(primaryPointer - anchor);
        if (!Number.isFinite(dist)) continue;
        if (dist < nearestDistance) {
          nearestDistance = dist;
          nearestId = item.id;
        }
        weights[item.id] = this.bottomDockWeightForDistance(dist);
      }
      this.bottomDockHoverWeightById = weights;
      if (nearestId) this.bottomDockHoverId = nearestId;
    },

    updateBottomDockPointer(ev) {
      if (!ev) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      if (!Number.isFinite(x) || x <= 0) return;
      this.bottomDockPointerX = x;
      if (Number.isFinite(y) && y > 0) this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
    },

    reviveBottomDockHoverFromPoint(clientX, clientY) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.getBoundingClientRect !== 'function') return;
      var rect = root.getBoundingClientRect();
      var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
      var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
      if (!withinX || !withinY) return;
      this.bottomDockPointerX = x;
      this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    scheduleBottomDockPreviewReflow() {
      this.cancelBottomDockPreviewReflow();
      var self = this;
      this._bottomDockPreviewReflowFrames = 10;
      var step = function() {
        if (!String(self.bottomDockHoverId || '').trim()) {
          self._bottomDockPreviewReflowRaf = 0;
          self._bottomDockPreviewReflowFrames = 0;
          return;
        }
        self.syncBottomDockPreview();
        self._bottomDockPreviewReflowFrames = Math.max(0, Number(self._bottomDockPreviewReflowFrames || 0) - 1);
        if (self._bottomDockPreviewReflowFrames <= 0) {
          self._bottomDockPreviewReflowRaf = 0;
          return;
        }
        self._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
      };
      this._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
    },

    cancelBottomDockPreviewReflow() {
      if (this._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewReflowRaf); } catch(_) {}
      }
      this._bottomDockPreviewReflowRaf = 0;
      this._bottomDockPreviewReflowFrames = 0;
    },

    scheduleBottomDockPreviewWidthSync() {
      if (this._bottomDockPreviewWidthRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewWidthRaf); } catch(_) {}
      }
      var self = this;
      var syncWidth = function() {
        self._bottomDockPreviewWidthRaf = 0;
        try {
          var bubble = document && typeof document.querySelector === 'function'
            ? document.querySelector('.bottom-dock-preview-bubble')
            : null;
          if (!bubble) return;
          var stack = (typeof bubble.querySelector === 'function')
            ? bubble.querySelector('.bottom-dock-preview-bubble-label-stack')
            : null;
          var contentWidth = Number(stack && (stack.scrollWidth || stack.offsetWidth) || 0);
          var bubbleStyle = (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function')
            ? window.getComputedStyle(bubble)
            : null;
          var paddingLeft = bubbleStyle ? Number(parseFloat(String(bubbleStyle.paddingLeft || '0')) || 0) : 0;
          var paddingRight = bubbleStyle ? Number(parseFloat(String(bubbleStyle.paddingRight || '0')) || 0) : 0;
          var borderLeft = bubbleStyle ? Number(parseFloat(String(bubbleStyle.borderLeftWidth || '0')) || 0) : 0;
          var borderRight = bubbleStyle ? Number(parseFloat(String(bubbleStyle.borderRightWidth || '0')) || 0) : 0;
          var nextWidth = contentWidth + paddingLeft + paddingRight + borderLeft + borderRight;
          if (!Number.isFinite(nextWidth) || nextWidth <= 0) return;
          self.bottomDockPreviewWidth = Math.max(0, Math.ceil(nextWidth));
        } catch(_) {}
      };
      if (typeof requestAnimationFrame === 'function') {
        this._bottomDockPreviewWidthRaf = requestAnimationFrame(syncWidth);
      } else {
        syncWidth();
      }
    },

    retriggerBottomDockPreviewLabelFx(nextLabel) {
      if (this._bottomDockPreviewLabelFxRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewLabelFxRaf); } catch(_) {}
      }
      if (this._bottomDockPreviewLabelFxTimer) {
        try { clearTimeout(this._bottomDockPreviewLabelFxTimer); } catch(_) {}
      }
      if (this._bottomDockPreviewLabelMorphTimer) {
        try { clearTimeout(this._bottomDockPreviewLabelMorphTimer); } catch(_) {}
      }
      this._bottomDockPreviewLabelFxRaf = 0;
      this._bottomDockPreviewLabelFxTimer = 0;
      this._bottomDockPreviewLabelMorphTimer = 0;
      this.bottomDockPreviewLabelFxReady = false;
      var self = this;
      var nextText = (typeof nextLabel === 'string')
        ? nextLabel
        : String(this.bottomDockPreviewText || '');
      var previousText = String(this.bottomDockPreviewText || '');
      this.bottomDockPreviewMorphFromText = previousText;
      this.bottomDockPreviewLabelMorphing = Boolean(previousText && nextText && previousText !== nextText);
      this.bottomDockPreviewText = nextText;
      this.scheduleBottomDockPreviewWidthSync();
      var commitLabelAndAnimateIn = function() {
        try {
          var node = document && typeof document.querySelector === 'function'
            ? document.querySelector('.bottom-dock-preview-bubble-label-stack')
            : null;
          if (node) void node.offsetWidth;
        } catch(_) {}
        if (typeof requestAnimationFrame === 'function') {
          self._bottomDockPreviewLabelFxRaf = requestAnimationFrame(function() {
            self._bottomDockPreviewLabelFxRaf = 0;
            self._bottomDockPreviewLabelFxTimer = window.setTimeout(function() {
              self._bottomDockPreviewLabelFxTimer = 0;
              self.bottomDockPreviewLabelFxReady = true;
              if (self._bottomDockPreviewLabelMorphTimer) {
                try { clearTimeout(self._bottomDockPreviewLabelMorphTimer); } catch(_) {}
              }
              self._bottomDockPreviewLabelMorphTimer = window.setTimeout(function() {
                self._bottomDockPreviewLabelMorphTimer = 0;
                self.bottomDockPreviewMorphFromText = '';
                self.bottomDockPreviewLabelMorphing = false;
                self.scheduleBottomDockPreviewWidthSync();
              }, 200);
            }, 16);
          });
        } else {
          self.bottomDockPreviewLabelFxReady = true;
        }
      };
      if (typeof requestAnimationFrame === 'function') {
        this._bottomDockPreviewLabelFxRaf = requestAnimationFrame(function() {
          self._bottomDockPreviewLabelFxRaf = 0;
          commitLabelAndAnimateIn();
        });
      } else {
        this.bottomDockPreviewLabelFxReady = true;
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewLabelMorphing = false;
      }
    },

    syncBottomDockPreview() {
      var key = String(this.bottomDockHoverId || '').trim();
      if (!key) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var text = this.bottomDockTileData(key, 'tooltip', '');
      var label = String(text || '').trim();
      if (!label) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var root = document.querySelector('.bottom-dock');
      var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (!root || !slot) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var wasVisible = Boolean(this.bottomDockPreviewVisible);
      var previousHoverKey = String(this.bottomDockPreviewHoverKey || '');
      var previousLabel = String(this.bottomDockPreviewText || '');
      var centerX = 0;
      var centerY = 0;
      var anchorY = 0;
      var anchorX = 0;
      var wallSide = this.bottomDockWallSide();
      var openSide = this.bottomDockOpenSide();
      var vertical = this.bottomDockIsVerticalSide(wallSide);
      var dockRect = (typeof root.getBoundingClientRect === 'function')
        ? root.getBoundingClientRect()
        : null;
      if (typeof slot.getBoundingClientRect === 'function' && dockRect) {
        var slotRect = slot.getBoundingClientRect();
        centerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        centerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(dockRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(dockRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(dockRect.left || 0) - 8;
        } else {
          anchorX = Number(dockRect.right || 0) + 8;
        }
      } else if (slot.offsetParent === root) {
        var rootRect = root.getBoundingClientRect();
        centerX = Number(rootRect.left || 0) + Number(slot.offsetLeft || 0) + (Number(slot.offsetWidth || 0) / 2);
        centerY = Number(rootRect.top || 0) + Number(slot.offsetTop || 0) + (Number(slot.offsetHeight || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(rootRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(rootRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(rootRect.left || 0) - 8;
        } else {
          anchorX = Number(rootRect.right || 0) + 8;
        }
      }
      var pointerX = Number(this.bottomDockPointerX || 0);
      var pointerY = Number(this.bottomDockPointerY || 0);
      if (!vertical && Number.isFinite(pointerX) && pointerX > 0) {
        if (dockRect) {
          var minX = Number(dockRect.left || 0);
          var maxX = Number(dockRect.right || 0);
          if (Number.isFinite(minX) && Number.isFinite(maxX) && maxX > minX) {
            pointerX = Math.max(minX, Math.min(maxX, pointerX));
          }
        }
        centerX = pointerX;
      }
      if (vertical && Number.isFinite(pointerY) && pointerY > 0) {
        if (dockRect) {
          var minY = Number(dockRect.top || 0);
          var maxY = Number(dockRect.bottom || 0);
          if (Number.isFinite(minY) && Number.isFinite(maxY) && maxY > minY) {
            pointerY = Math.max(minY, Math.min(maxY, pointerY));
          }
        }
        centerY = pointerY;
      }
      if (!Number.isFinite(centerX)) centerX = 0;
      if (!Number.isFinite(centerY)) centerY = 0;
      if (!Number.isFinite(anchorX)) anchorX = 0;
      if (!Number.isFinite(anchorY)) anchorY = 0;
      this.bottomDockPreviewX = vertical ? anchorX : centerX;
      this.bottomDockPreviewY = vertical ? centerY : anchorY;
      this.bottomDockPreviewHoverKey = key;
      this.bottomDockPreviewVisible = true;
      if (wasVisible && (key !== previousHoverKey || label !== previousLabel)) {
        this.retriggerBottomDockPreviewLabelFx(label);
      } else {
        this.bottomDockPreviewText = label;
        this.scheduleBottomDockPreviewWidthSync();
        if (!this.bottomDockPreviewLabelMorphing) {
          this.bottomDockPreviewMorphFromText = '';
        }
        if (!wasVisible && !this.bottomDockPreviewLabelFxReady) {
          this.bottomDockPreviewLabelFxReady = true;
        }
      }
    },

    bindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler || this._bottomDockPointerUpHandler) return;
      var self = this;
      this._bottomDockPointerMoveHandler = function(ev) { self.handleBottomDockPointerMove(ev); };
      this._bottomDockPointerUpHandler = function(ev) { self.endBottomDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockPointerUpHandler, true);
    },

    unbindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockPointerMoveHandler = null;
      this._bottomDockPointerUpHandler = null;
    },

    startBottomDockPointerDrag(id, ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      if (!key) return;
      var hostEl = ev && ev.currentTarget ? ev.currentTarget : null;
      if (hostEl && typeof hostEl.getBoundingClientRect === 'function') {
        try {
          var rect = hostEl.getBoundingClientRect();
          var width = Number(rect.width || 32);
          var height = Number(rect.height || 32);
          var baseWidth = Number(hostEl && hostEl.offsetWidth ? hostEl.offsetWidth : width || 32);
          var baseHeight = Number(hostEl && hostEl.offsetHeight ? hostEl.offsetHeight : height || 32);
          if (!Number.isFinite(width) || width <= 0) width = 32;
          if (!Number.isFinite(height) || height <= 0) height = 32;
          if (!Number.isFinite(baseWidth) || baseWidth <= 0) baseWidth = width;
          if (!Number.isFinite(baseHeight) || baseHeight <= 0) baseHeight = height;
          var expandedScale = this.bottomDockExpandedScale();
          var expandedWidth = baseWidth * expandedScale;
          var expandedHeight = baseHeight * expandedScale;
          this._bottomDockDragGhostWidth = Math.max(20, Math.min(112, Math.max(width, expandedWidth)));
          this._bottomDockDragGhostHeight = Math.max(20, Math.min(112, Math.max(height, expandedHeight)));
          var offsetX = Number(ev.clientX || 0) - Number(rect.left || 0);
          var offsetY = Number(ev.clientY || 0) - Number(rect.top || 0);
          var relX = Number.isFinite(offsetX) && width > 0 ? (offsetX / width) : 0.5;
          var relY = Number.isFinite(offsetY) && height > 0 ? (offsetY / height) : 0.5;
          relX = Math.max(0, Math.min(1, relX));
          relY = Math.max(0, Math.min(1, relY));
          this._bottomDockPointerGrabOffsetX = relX * this._bottomDockDragGhostWidth;
          this._bottomDockPointerGrabOffsetY = relY * this._bottomDockDragGhostHeight;
        } catch(_) {
          this._bottomDockPointerGrabOffsetX = 16;
          this._bottomDockPointerGrabOffsetY = 16;
          this._bottomDockDragGhostWidth = 32;
          this._bottomDockDragGhostHeight = 32;
        }
      } else {
        this._bottomDockPointerGrabOffsetX = 16;
        this._bottomDockPointerGrabOffsetY = 16;
        this._bottomDockDragGhostWidth = 32;
        this._bottomDockDragGhostHeight = 32;
      }
      try {
        if (hostEl && typeof hostEl.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          hostEl.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      this._bottomDockPointerActive = true;
      this._bottomDockPointerMoved = false;
      this._bottomDockPointerCandidateId = key;
      this._bottomDockPointerStartX = Number(ev.clientX || 0);
      this._bottomDockPointerStartY = Number(ev.clientY || 0);
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      this._bottomDockReorderLockUntil = 0;
      this.bindBottomDockPointerListeners();
    },

    activateBottomDockPointerDrag(ev) {
      if (this._bottomDockPointerMoved) return;
      var dragId = String(this._bottomDockPointerCandidateId || '').trim();
      if (!dragId) return;
      this._bottomDockPointerMoved = true;
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this._bottomDockRevealTargetDuringSettle = false;
      this.bottomDockDragId = dragId;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this.cleanupBottomDockDragGhost();
      this.captureBottomDockDragBoundaries(dragId);
      var originNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + dragId + '"]');
      if (!originNode || !document || !document.body) return;
      var dockEl = document.querySelector('.bottom-dock');
      if (dockEl && dockEl.style && typeof dockEl.style.setProperty === 'function') {
        dockEl.style.setProperty('--bottom-dock-drag-scale', String(this.readBottomDockScale(dockEl)));
      }
      var ghost = document.createElement('div');
      ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn dock-tile';
      var tone = '';
      var iconKind = '';
      try {
        tone = String(originNode.getAttribute('data-dock-tone') || '').trim();
        iconKind = String(originNode.getAttribute('data-dock-icon') || '').trim();
      } catch(_) {
        tone = '';
        iconKind = '';
      }
      if (tone) ghost.setAttribute('data-dock-tone', tone);
      if (iconKind) ghost.setAttribute('data-dock-icon', iconKind);
      if (originNode.classList && typeof originNode.classList.contains === 'function') {
        if (originNode.classList.contains('active')) ghost.classList.add('active');
      }
      ghost.setAttribute('aria-hidden', 'true');
      ghost.innerHTML = String(originNode.innerHTML || '');
      ghost.style.position = 'fixed';
      ghost.style.width = Math.round(Number(this._bottomDockDragGhostWidth || 32)) + 'px';
      ghost.style.height = Math.round(Number(this._bottomDockDragGhostHeight || 32)) + 'px';
      ghost.style.borderRadius = Math.round((Number(this._bottomDockDragGhostWidth || 32) / 32) * 11) + 'px';
      ghost.style.setProperty(
        '--dock-ghost-scale',
        String(Math.max(0.8, Math.min(4, Number(this._bottomDockDragGhostWidth || 32) / 32)))
      );
      var ghostUpDeg = Number(this.bottomDockUpDegForSide(this.bottomDockActiveSide()) || 0);
      var ghostTileRotation = Math.round(ghostUpDeg) + 'deg';
      var ghostIconRotation = '0deg';
      ghost.style.setProperty('--bottom-dock-tile-rotation-deg', ghostTileRotation);
      ghost.style.setProperty('--bottom-dock-icon-rotation-deg', ghostIconRotation);
      var ghostX = Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16);
      var ghostY = Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16);
      this._bottomDockGhostCurrentX = ghostX;
      this._bottomDockGhostCurrentY = ghostY;
      ghost.style.left = Math.round(ghostX) + 'px';
      ghost.style.top = Math.round(ghostY) + 'px';
      ghost.style.margin = '0';
      ghost.style.pointerEvents = 'none';
      ghost.style.opacity = '1';
      document.body.appendChild(ghost);
      this._bottomDockDragGhostEl = ghost;
      this.setBottomDockGhostTarget(ghostX, ghostY);
    },

    handleBottomDockPointerMove(ev) {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      var movedX = Math.abs(Number(ev.clientX || 0) - Number(this._bottomDockPointerStartX || 0));
      var movedY = Math.abs(Number(ev.clientY || 0) - Number(this._bottomDockPointerStartY || 0));
      if (!this._bottomDockPointerMoved) {
        if (movedX < 5 && movedY < 5) return;
        this.activateBottomDockPointerDrag(ev);
      }
      if (!this._bottomDockPointerMoved) return;
      if (ev && typeof ev.preventDefault === 'function' && ev.cancelable) ev.preventDefault();
      var ghost = this._bottomDockDragGhostEl;
      if (ghost) {
        this.setBottomDockGhostTarget(
          Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16),
          Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16)
        );
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var insertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, ev);
      if (Number.isFinite(insertionIndex)) {
        var normalizedIndex = Math.max(0, Math.round(Number(insertionIndex || 0)));
        var nowMs = Date.now();
        var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
        if (
          normalizedIndex !== Number(this._bottomDockLastInsertionIndex || -1) &&
          (!Number.isFinite(lockUntil) || lockUntil <= nowMs)
        ) {
          var changed = this.applyBottomDockReorderByIndex(dragId, normalizedIndex, true);
          this._bottomDockLastInsertionIndex = normalizedIndex;
          if (changed) {
            var moveDuration = this.bottomDockMoveDurationMs();
            var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.55)));
            this._bottomDockReorderLockUntil = nowMs + lockMs;
          }
        }
        return;
      }
      var targetId = '';
      var targetEl = null;
      try {
        var pointerEl = typeof document !== 'undefined' && typeof document.elementFromPoint === 'function'
          ? document.elementFromPoint(Number(ev.clientX || 0), Number(ev.clientY || 0))
          : null;
        targetEl = pointerEl && typeof pointerEl.closest === 'function'
          ? pointerEl.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    endBottomDockPointerDrag() {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerActive = false;
      this.unbindBottomDockPointerListeners();
      if (!this._bottomDockPointerMoved) {
        this._bottomDockPointerCandidateId = '';
        return;
      }
      var dragId = String(this.bottomDockDragId || this._bottomDockPointerCandidateId || '').trim();
      if (dragId) {
        var finalPointerEvent = {
          clientX: Number(this._bottomDockPointerLastX || 0),
          clientY: Number(this._bottomDockPointerLastY || 0)
        };
        var finalInsertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, finalPointerEvent);
        if (Number.isFinite(finalInsertionIndex)) {
          this.applyBottomDockReorderByIndex(dragId, finalInsertionIndex, false);
        } else if (this.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
          var appendTargetId = this.bottomDockAppendTargetId(dragId);
          if (appendTargetId) {
            this.handleBottomDockDragOver(appendTargetId, finalPointerEvent, true);
          }
        }
      }
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      var self = this;
      var finalizeDrag = function() {
        var dockEl = document.querySelector('.bottom-dock');
        if (dockEl && dockEl.style && typeof dockEl.style.removeProperty === 'function') {
          dockEl.style.removeProperty('--bottom-dock-drag-scale');
        }
        var dropX = Number(self._bottomDockPointerLastX || 0);
        var dropY = Number(self._bottomDockPointerLastY || 0);
        self.bottomDockDragId = '';
        self.bottomDockHoverId = '';
        self.bottomDockDragStartOrder = [];
        self._bottomDockPointerGrabOffsetX = 16;
        self._bottomDockPointerGrabOffsetY = 16;
        self._bottomDockDragGhostWidth = 32;
        self._bottomDockDragGhostHeight = 32;
        self._bottomDockPointerCandidateId = '';
        self._bottomDockPointerMoved = false;
        self._bottomDockDragBoundaries = [];
        self._bottomDockLastInsertionIndex = -1;
        self.reviveBottomDockHoverFromPoint(dropX, dropY);
        self._bottomDockPointerLastX = 0;
        self._bottomDockPointerLastY = 0;
      };
      this.settleBottomDockDragGhost(dragId, finalizeDrag);
    },

    shouldSuppressBottomDockClick() {
      var until = Number(this._bottomDockSuppressClickUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    clearBottomDockClickAnimation() {
      if (this._bottomDockClickAnimTimer) {
        try { clearTimeout(this._bottomDockClickAnimTimer); } catch(_) {}
      }
      this._bottomDockClickAnimTimer = 0;
      this.bottomDockClickAnimId = '';
    },

    triggerBottomDockClickAnimation(id, durationOverrideMs) {
      var key = String(id || '').trim();
      if (!key || typeof window === 'undefined' || typeof window.setTimeout !== 'function') return;
      this.clearBottomDockClickAnimation();
      this.bottomDockClickAnimId = key;
      var self = this;
      var durationMs = Number(durationOverrideMs);
      if (!Number.isFinite(durationMs) || durationMs < 120) {
        durationMs = Number(self._bottomDockClickAnimDurationMs || 980);
      }
      if (!Number.isFinite(durationMs) || durationMs < 120) durationMs = 980;
      if (typeof document !== 'undefined') {
        try {
          var tileNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
          if (tileNode && tileNode.style && typeof tileNode.style.setProperty === 'function') {
            tileNode.style.setProperty('--dock-click-duration', Math.round(durationMs) + 'ms');
          }
        } catch(_) {}
      }
      self._bottomDockClickAnimTimer = window.setTimeout(function() {
        if (typeof document !== 'undefined') {
          try {
            var activeNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
            if (activeNode && activeNode.style && typeof activeNode.style.removeProperty === 'function') {
              activeNode.style.removeProperty('--dock-click-duration');
            }
          } catch(_) {}
        }
        self._bottomDockClickAnimTimer = 0;
        self.bottomDockClickAnimId = '';
      }, durationMs);
    },

    bottomDockIsClickAnimating(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      return String(this.bottomDockClickAnimId || '').trim() === key;
    },

    handleBottomDockTileClick(id, targetPage, ev) {
      if (this.shouldSuppressBottomDockClick()) return;
      var key = String(id || '').trim();
      var pageKey = String(targetPage || '').trim();
      var clickAnimation = '';
      var clickDurationMs = 0;
      try {
        var triggerEl = ev && ev.currentTarget ? ev.currentTarget : null;
        clickAnimation = String(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-animation') || '')
            : ''
        ).trim();
        clickDurationMs = Number(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-duration-ms') || '')
            : ''
        );
      } catch(_) {
        clickAnimation = '';
        clickDurationMs = 0;
      }
      if (!Number.isFinite(clickDurationMs) || clickDurationMs < 120) clickDurationMs = 0;
      if (key && clickAnimation && clickAnimation !== 'none') {
        this.triggerBottomDockClickAnimation(key, clickDurationMs);
      }
      if (pageKey) this.navigate(pageKey);
    },

    bottomDockIsDraggingVisual(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      if (this._bottomDockRevealTargetDuringSettle) return false;
      return String(this.bottomDockDragId || '').trim() === key;
    },

    bottomDockIsNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 1;
    },

    bottomDockIsSecondNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 2;
    },

    bottomDockHoverWeight(id) {
      var key = String(id || '').trim();
      if (!key) return 0;
      var weights = this.bottomDockHoverWeightById && typeof this.bottomDockHoverWeightById === 'object'
        ? this.bottomDockHoverWeightById
        : null;
      if (weights && Object.prototype.hasOwnProperty.call(weights, key)) {
        var exact = Number(weights[key] || 0);
        if (Number.isFinite(exact)) return Math.max(0, Math.min(1, exact));
      }
      if (key === String(this.bottomDockHoverId || '').trim()) return 1;
      if (this.bottomDockIsNeighbor(key)) return 0.33;
      if (this.bottomDockIsSecondNeighbor(key)) return 0.11;
      return 0;
    },

    startBottomDockDrag(id, ev) {
      var key = String(id || '').trim();
      if (!key) return;
      this.cleanupBottomDockDragGhost();
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this.bottomDockDragId = key;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this._bottomDockReorderLockUntil = 0;
      this.captureBottomDockDragBoundaries(key);
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
        try {
          var dragNode = ev.currentTarget;
          if (dragNode && typeof ev.dataTransfer.setDragImage === 'function') {
            var rect = dragNode.getBoundingClientRect();
            var ghost = dragNode.cloneNode(true);
            if (ghost && document && document.body) {
              ghost.classList.add('bottom-dock-drag-ghost');
              ghost.style.position = 'fixed';
              ghost.style.left = '-9999px';
              ghost.style.top = '-9999px';
              ghost.style.margin = '0';
              ghost.style.transform = 'none';
              ghost.style.pointerEvents = 'none';
              ghost.style.opacity = '1';
              document.body.appendChild(ghost);
              this._bottomDockDragGhostEl = ghost;
              ev.dataTransfer.setDragImage(
                ghost,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            } else {
              ev.dataTransfer.setDragImage(
                dragNode,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            }
          }
        } catch(_) {}
        try { ev.dataTransfer.setData('application/x-infring-dock', key); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key); } catch(_) {}
      }
    },

    bottomDockShouldInsertAfter(targetId, ev, targetEl) {
      var key = String(targetId || '').trim();
      if (!key) return false;
      if (!ev) return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var node = targetEl || null;
      if (!node && typeof document !== 'undefined') {
        try {
          node = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var half = this.bottomDockAxisHalfExtent(width, height, basis).primary;
      if (!Number.isFinite(half) || half <= 0) half = Math.max(width, height) / 2;
      if (!Number.isFinite(half) || half <= 0) return false;
      var ratio = (pointerProj.primary - (centerProj.primary - half)) / (half * 2);
      return ratio >= 0.5;
    },

    captureBottomDockDragBoundaries(dragId) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock) {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var centers = [];
      var basis = this.bottomDockAxisBasis();
      try {
        var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || typeof node.getAttribute !== 'function') continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
          var rect = node.getBoundingClientRect();
          var width = Number(rect.width || 0);
          var height = Number(rect.height || 0);
          if (!Number.isFinite(width) || width <= 0) continue;
          if (!Number.isFinite(height) || height <= 0) continue;
          var centerX = Number(rect.left || 0) + (width / 2);
          var centerY = Number(rect.top || 0) + (height / 2);
          centers.push(this.bottomDockProjectPointToAxis(centerX, centerY, basis).primary);
        }
      } catch(_) {}
      centers.sort(function(a, b) { return a - b; });
      this._bottomDockDragBoundaries = centers;
      this._bottomDockLastInsertionIndex = -1;
      return centers;
    },

    bottomDockAppendTargetId(dragId) {
      var key = String(dragId || '').trim();
      if (!key) return '';
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var filtered = [];
      for (var i = 0; i < order.length; i += 1) {
        var id = String(order[i] || '').trim();
        if (!id || id === key) continue;
        filtered.push(id);
      }
      if (!filtered.length) return '';
      return String(filtered[filtered.length - 1] || '').trim();
    },

    bottomDockShouldAppendFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev || typeof document === 'undefined') return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var appendTargetId = this.bottomDockAppendTargetId(key);
      if (!appendTargetId) return false;
      var node = null;
      try {
        node = document.querySelector('.bottom-dock-btn[data-dock-id="' + appendTargetId + '"]');
      } catch(_) {
        node = null;
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var extent = this.bottomDockAxisHalfExtent(width, height, basis);
      var halfPrimary = Number(extent.primary || 0);
      var halfSecondary = Number(extent.secondary || 0);
      if (!Number.isFinite(halfPrimary) || halfPrimary <= 0) halfPrimary = Math.max(width, height) / 2;
      if (!Number.isFinite(halfSecondary) || halfSecondary <= 0) halfSecondary = Math.min(width, height) / 2;
      var secondaryPad = Math.max(18, halfSecondary * 0.75);
      if (Math.abs(pointerProj.secondary - centerProj.secondary) > (halfSecondary + secondaryPad)) return false;
      var threshold = centerProj.primary + halfPrimary - Math.min(18, halfPrimary * 0.7);
      return pointerProj.primary >= threshold;
    },

    bottomDockInsertionIndexFromCoords(dragId, clientXRaw, clientYRaw) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') return null;
      var clientX = Number(clientXRaw || 0);
      var clientY = Number(clientYRaw || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
      var dockRect = dock.getBoundingClientRect();
      var basis = this.bottomDockAxisBasis();
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var dockBounds = this.bottomDockProjectedRectBounds(dockRect, basis);
      if (!dockBounds) return null;
      if (
        pointerProj.secondary < (Number(dockBounds.secondaryMin || 0) - 24) ||
        pointerProj.secondary > (Number(dockBounds.secondaryMax || 0) + 24)
      ) return null;
      var centers = this.captureBottomDockDragBoundaries(key);
      if (centers.length === 0) return null;
      var insertionIndex = 0;
      for (var c = 0; c < centers.length; c += 1) {
        if (pointerProj.primary >= centers[c]) insertionIndex += 1;
      }
      insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
      return insertionIndex;
    },

    bottomDockGhostCenterPoint() {
      var x = Number(this._bottomDockGhostTargetX || this._bottomDockGhostCurrentX || 0);
      var y = Number(this._bottomDockGhostTargetY || this._bottomDockGhostCurrentY || 0);
      var width = Number(this._bottomDockDragGhostWidth || 0);
      var height = Number(this._bottomDockDragGhostHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = 32;
      if (!Number.isFinite(height) || height <= 0) height = 32;
      return {
        x: x + (width / 2),
        y: y + (height / 2)
      };
    },

    bottomDockInsertionIndexFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev) return null;
      var center = this.bottomDockGhostCenterPoint();
      return this.bottomDockInsertionIndexFromCoords(key, center.x, center.y);
    },

    applyBottomDockReorderByIndex(dragId, insertionIndex, animate) {
      var key = String(dragId || '').trim();
      if (!key) return false;
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = current.indexOf(key);
      if (fromIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      var idx = Number(insertionIndex);
      if (!Number.isFinite(idx)) return false;
      idx = Math.max(0, Math.min(next.length, Math.round(idx)));
      next.splice(idx, 0, key);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var doAnimate = Boolean(animate);
      var beforeRects = doAnimate ? this.bottomDockButtonRects() : null;
      this.bottomDockOrder = next;
      if (doAnimate && beforeRects) this.animateBottomDockFromRects(beforeRects);
      return true;
    },

    handleBottomDockContainerDragOver(ev) {
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    handleBottomDockContainerDrop(ev) {
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId) {
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDrop(targetId, ev, preferAfter);
        return;
      }
      if (this.bottomDockShouldAppendFromPointer(dragId, ev)) {
        var appendTargetId = this.bottomDockAppendTargetId(dragId);
        if (appendTargetId) {
          this.handleBottomDockDrop(appendTargetId, ev, true);
          return;
        }
      }
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
      this.bottomDockDragId = '';
      this.bottomDockDragStartOrder = [];
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
      this.reviveBottomDockHoverFromPoint(
        Number(ev && ev.clientX || 0),
        Number(ev && ev.clientY || 0)
      );
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockDragOver(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId || targetId === dragId) return;
      var nowMs = Date.now();
      var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
      if (Number.isFinite(lockUntil) && lockUntil > nowMs) return;
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var placeAfter = Boolean(preferAfter);
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var next = current.slice();
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return;
      var beforeRects = this.bottomDockButtonRects();
      this.bottomDockOrder = next;
      this.animateBottomDockFromRects(beforeRects);
      var moveDuration = this.bottomDockMoveDurationMs();
      var lockMs = Math.max(320, Math.min(520, Math.round(moveDuration + 60)));
      this._bottomDockReorderLockUntil = nowMs + lockMs;
    },

    handleBottomDockDrop(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId) {
        this._bottomDockSuppressClickUntil = Date.now() + 220;
        this.cleanupBottomDockDragGhost();
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      if (targetId === dragId) {
        var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
        var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
        if (JSON.stringify(current) !== JSON.stringify(start)) {
          this.bottomDockOrder = current;
          this.persistBottomDockOrder();
          this.bottomDockDragCommitted = true;
        }
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this._bottomDockSuppressClickUntil = Date.now() + 220;
        this.cleanupBottomDockDragGhost();
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
        return;
      }
      var next = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      var placeAfter = Boolean(preferAfter);
      if (fromIndex < 0 || toIndex < 0) {
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      this.bottomDockOrder = next;
      this.persistBottomDockOrder();
      this.bottomDockDragCommitted = true;
      this.bottomDockDragId = '';
      this.bottomDockDragStartOrder = [];
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
      this.reviveBottomDockHoverFromPoint(
        Number(ev && ev.clientX || 0),
        Number(ev && ev.clientY || 0)
      );
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endBottomDockDrag() {
      if (!this.bottomDockDragCommitted && Array.isArray(this.bottomDockDragStartOrder) && this.bottomDockDragStartOrder.length) {
        var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
        var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
        if (JSON.stringify(current) !== JSON.stringify(start)) {
          this.bottomDockOrder = current;
          this.persistBottomDockOrder();
          this.bottomDockDragCommitted = true;
        } else {
          var beforeRects = this.bottomDockButtonRects();
          this.bottomDockOrder = start;
          this.animateBottomDockFromRects(beforeRects);
        }
      }
      this.bottomDockDragId = '';
      this.bottomDockHoverId = '';
      this.bottomDockDragStartOrder = [];
      this.bottomDockDragCommitted = false;
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
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
