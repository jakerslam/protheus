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
    bottomDockDragId: '',
    bottomDockDragStartOrder: [],
    bottomDockDragCommitted: false,
    bottomDockHoverId: '',
    _bottomDockDragGhostEl: null,
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
    _bottomDockRevealTargetDuringSettle: false,
    _bottomDockDragBoundaries: [],
    _bottomDockLastInsertionIndex: -1,
    _bottomDockReorderLockUntil: 0,

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

    bottomDockMoveDurationMs() {
      var raw = Number(this._bottomDockMoveDurationMs || 360);
      if (!Number.isFinite(raw)) raw = 360;
      return Math.max(120, Math.round(raw));
    },

    bottomDockExpandedScale() {
      var raw = Number(this._bottomDockExpandedScale || 1.54);
      if (!Number.isFinite(raw) || raw <= 1) raw = 1.54;
      return raw;
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

    bottomDockDefaultOrder() {
      return ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
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
        out[id] = { left: Number(rect.left || 0), top: Number(rect.top || 0) };
      }
      return out;
    },

    animateBottomDockFromRects(beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var durationMs = this.bottomDockMoveDurationMs();
      requestAnimationFrame(function() {
        var root = document.querySelector('.bottom-dock');
        if (!root) return;
        var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i++) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
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
              if (
                !el.classList.contains('dragging') &&
                !el.classList.contains('hovered') &&
                !el.classList.contains('neighbor-hover') &&
                !el.classList.contains('second-neighbor-hover')
              ) {
                el.style.transform = '';
              }
              el.style.transition = '';
            }, durationMs + 30);
          })(node);
        }
      });
    },

    setBottomDockHover(id) {
      if (String(this.bottomDockDragId || '').trim()) return;
      this.bottomDockHoverId = String(id || '').trim();
    },

    clearBottomDockHover(id) {
      if (!id) {
        this.bottomDockHoverId = '';
        return;
      }
      var key = String(id || '').trim();
      if (!this.bottomDockHoverId || key === this.bottomDockHoverId) {
        this.bottomDockHoverId = '';
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
      ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn';
      if (originNode.classList && typeof originNode.classList.contains === 'function') {
        if (originNode.classList.contains('messages-btn')) ghost.classList.add('messages-btn');
        if (originNode.classList.contains('apps-btn')) ghost.classList.add('apps-btn');
        if (originNode.classList.contains('settings-btn')) ghost.classList.add('settings-btn');
        if (originNode.classList.contains('automation-btn')) ghost.classList.add('automation-btn');
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
        if (this.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
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
        self.bottomDockDragId = '';
        self.bottomDockHoverId = '';
        self.bottomDockDragStartOrder = [];
        self._bottomDockPointerGrabOffsetX = 16;
        self._bottomDockPointerGrabOffsetY = 16;
        self._bottomDockDragGhostWidth = 32;
        self._bottomDockDragGhostHeight = 32;
        self._bottomDockPointerCandidateId = '';
        self._bottomDockPointerLastX = 0;
        self._bottomDockPointerLastY = 0;
        self._bottomDockPointerMoved = false;
        self._bottomDockDragBoundaries = [];
        self._bottomDockLastInsertionIndex = -1;
      };
      this.settleBottomDockDragGhost(dragId, finalizeDrag);
    },

    shouldSuppressBottomDockClick() {
      var until = Number(this._bottomDockSuppressClickUntil || 0);
      return Number.isFinite(until) && until > Date.now();
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

    startBottomDockDrag(id, ev) {
      var key = String(id || '').trim();
      if (!key) return;
      this.cleanupBottomDockDragGhost();
      this.bottomDockHoverId = '';
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
      if (!Number.isFinite(clientX)) return false;
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
      if (!Number.isFinite(width) || width <= 0) return false;
      var offset = clientX - Number(rect.left || 0);
      var ratio = offset / width;
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
      try {
        var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || typeof node.getAttribute !== 'function') continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
          var rect = node.getBoundingClientRect();
          var width = Number(rect.width || 0);
          if (!Number.isFinite(width) || width <= 0) continue;
          centers.push(Number(rect.left || 0) + (width / 2));
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
      var yMin = Number(rect.top || 0) - (height * 0.75);
      var yMax = Number(rect.bottom || 0) + (height * 0.75);
      if (clientY < yMin || clientY > yMax) return false;
      var thresholdX = Number(rect.right || 0) - Math.min(18, width * 0.35);
      return clientX >= thresholdX;
    },

    bottomDockInsertionIndexFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev || typeof document === 'undefined') return null;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
      var dockRect = dock.getBoundingClientRect();
      var yMin = Number(dockRect.top || 0) - 24;
      var yMax = Number(dockRect.bottom || 0) + 24;
      if (clientY < yMin || clientY > yMax) return null;
      var centers = Array.isArray(this._bottomDockDragBoundaries)
        ? this._bottomDockDragBoundaries.slice()
        : [];
      if (centers.length === 0) {
        centers = this.captureBottomDockDragBoundaries(key);
      }
      if (centers.length === 0) return null;
      var insertionIndex = 0;
      for (var c = 0; c < centers.length; c += 1) {
        if (clientX >= centers[c]) insertionIndex += 1;
      }
      insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
      return insertionIndex;
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
      var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.95)));
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
