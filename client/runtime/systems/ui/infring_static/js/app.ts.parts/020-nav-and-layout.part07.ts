    readChatSidebarElement() {
      if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
      try { return document.querySelector('.sidebar'); } catch(_) {}
      return null;
    },
    readChatSidebarHeight() {
      var node = this.readChatSidebarElement();
      var height = Number(node && node.offsetHeight || 0);
      if (!Number.isFinite(height) || height <= 0) {
        height = Math.max(180, Math.round(this.taskbarReadViewportHeight() * 0.52));
      }
      return height;
    },
    readChatSidebarWidth() {
      var node = this.readChatSidebarElement();
      var width = Number(node && node.offsetWidth || 0);
      if (Number.isFinite(width) && width > 0) return width;
      var fallback = this.sidebarCollapsed ? 72 : 248;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var key = this.sidebarCollapsed ? '--sidebar-collapsed' : '--sidebar-width';
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue(key) || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    readChatSidebarPulltabWidth() {
      var fallback = 22;
      if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
        try {
          var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--sidebar-pulltab-width') || '').trim();
          var parsed = parseFloat(raw);
          if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
        } catch(_) {}
      }
      return Math.max(1, Math.round(fallback));
    },
    chatSidebarClampLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = Number(leftRaw);
      if (!Number.isFinite(left)) left = minLeft;
      return Math.max(minLeft, Math.min(maxLeft, left));
    },
    chatSidebarResolvedLeft() {
      if (this.chatSidebarDragActive) return this.chatSidebarClampLeft(this.chatSidebarDragLeft);
      var ratio = 0;
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
      } catch(_) {}
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      return this.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
    },
    chatSidebarPersistPlacementFromLeft(leftRaw) {
      var wallGap = this.overlayWallGapPx();
      var minLeft = wallGap;
      var maxLeft = this.chatOverlayViewportWidth() - wallGap - this.readChatSidebarWidth() - this.readChatSidebarPulltabWidth();
      if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
      var left = this.chatSidebarClampLeft(leftRaw);
      var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 0;
      ratio = Math.max(0, Math.min(1, ratio));
      try {
        localStorage.setItem('infring-chat-sidebar-placement-x', String(ratio));
      } catch(_) {}
    },
    chatSidebarClampTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = Number(topRaw);
      if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.5);
      return Math.max(minTop, Math.min(maxTop, top));
    },
    chatSidebarResolvedTop() {
      if (this.chatSidebarDragActive) return this.chatSidebarClampTop(this.chatSidebarDragTop);
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var ratio = Number(this.chatSidebarPlacementY);
      if (!Number.isFinite(ratio)) ratio = 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      return this.chatSidebarClampTop(minTop + ((maxTop - minTop) * ratio));
    },
    chatSidebarPersistPlacementFromTop(topRaw) {
      var bounds = this.chatOverlayVerticalBounds();
      var height = this.readChatSidebarHeight();
      var minTop = Number(bounds.minTop || 0);
      var maxTop = Number(bounds.maxBottom || 0) - height;
      if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
      var top = this.chatSidebarClampTop(topRaw);
      var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.5;
      ratio = Math.max(0, Math.min(1, ratio));
      this.chatSidebarPlacementY = ratio;
      try {
        localStorage.setItem('infring-chat-sidebar-placement-y', String(ratio));
      } catch(_) {}
    },
    chatSidebarContainerStyle() {
      if (this.page !== 'chat') return '';
      var top = this.chatSidebarResolvedTop();
      var left = this.chatSidebarResolvedLeft();
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      return (
        'left:' + Math.round(left) + 'px;' +
        'top:' + Math.round(top) + 'px;' +
        'bottom:auto;' +
        'height:fit-content;' +
        'min-height:calc(56px * 3);' +
        'max-height:80vh;' +
        'transform:none;' +
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;'
      );
    },
    chatSidebarNavShellStyle() {
      return this.page === 'chat'
        ? 'flex:0 1 auto;min-height:0;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarNavStyle() {
      return this.page === 'chat'
        ? 'height:auto;flex:0 1 auto;max-height:calc(80vh - 16px);'
        : '';
    },
    chatSidebarPulltabStyle() {
      if (this.page !== 'chat') return '';
      var durationMs = this.chatSidebarDragActive ? 0 : this.dragSurfaceMoveDurationMs(this._chatSidebarMoveDurationMs, 280);
      return [
        'position:absolute;',
        'left:100%;',
        'right:auto;',
        'top:50%;',
        'transform:translateY(-50%);',
        '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;'
      ].join('');
    },
    shouldIgnoreChatSidebarDragTarget(target) {
      if (!target || typeof target.closest !== 'function') return true;
      if (target.closest('.sidebar-pulltab')) return false;
      return Boolean(target.closest('input,textarea,select,[contenteditable="true"]'));
    },

    bindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler || this._chatSidebarPointerUpHandler) return;
      var self = this;
      this._chatSidebarPointerMoveHandler = function(ev) { self.handleChatSidebarPointerMove(ev); };
      this._chatSidebarPointerUpHandler = function() { self.endChatSidebarPointerDrag(); };
      window.addEventListener('pointermove', this._chatSidebarPointerMoveHandler, true);
      window.addEventListener('pointerup', this._chatSidebarPointerUpHandler, true);
      window.addEventListener('pointercancel', this._chatSidebarPointerUpHandler, true);
      window.addEventListener('mousemove', this._chatSidebarPointerMoveHandler, true);
      window.addEventListener('mouseup', this._chatSidebarPointerUpHandler, true);
    },

    unbindChatSidebarPointerListeners() {
      if (this._chatSidebarPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._chatSidebarPointerMoveHandler, true); } catch(_) {}
      }
      if (this._chatSidebarPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._chatSidebarPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._chatSidebarPointerUpHandler, true); } catch(_) {}
      }
      this._chatSidebarPointerMoveHandler = null;
      this._chatSidebarPointerUpHandler = null;
    },

    startChatSidebarPointerDrag(ev) {
      if (!ev || this.page !== 'chat') return;
      var button = Number(ev.button);
      if (Number.isFinite(button) && button !== 0) return;
      var target = ev && ev.target ? ev.target : null;
      if (this.shouldIgnoreChatSidebarDragTarget(target)) return;
      this._chatSidebarPointerActive = true;
      this._chatSidebarPointerMoved = false;
      this._chatSidebarPointerFromPulltab = Boolean(target && typeof target.closest === 'function' && target.closest('.sidebar-pulltab'));
      this._chatSidebarPointerStartX = Number(ev.clientX || 0);
      this._chatSidebarPointerStartY = Number(ev.clientY || 0);
      this._chatSidebarPointerOriginLeft = this.chatSidebarResolvedLeft();
      this._chatSidebarPointerOriginTop = this.chatSidebarResolvedTop();
      this.chatSidebarDragLeft = this._chatSidebarPointerOriginLeft;
      this.chatSidebarDragTop = this._chatSidebarPointerOriginTop;
      this.bindChatSidebarPointerListeners();
      try {
        if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          ev.currentTarget.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
    },

    handleChatSidebarPointerMove(ev) {
      if (!this._chatSidebarPointerActive || this.page !== 'chat') return;
      var nextX = Number(ev.clientX || 0);
      var nextY = Number(ev.clientY || 0);
      var movedX = Math.abs(nextX - Number(this._chatSidebarPointerStartX || 0));
      var movedY = Math.abs(nextY - Number(this._chatSidebarPointerStartY || 0));
      if (!this._chatSidebarPointerMoved) {
        if (movedX < 4 && movedY < 4) return;
        this._chatSidebarPointerMoved = true;
        this.chatSidebarDragActive = true;
        this.hideDashboardPopupBySource('sidebar');
      }
      var candidateLeft = Number(this._chatSidebarPointerOriginLeft || 0) + (nextX - Number(this._chatSidebarPointerStartX || 0));
      var candidateTop = Number(this._chatSidebarPointerOriginTop || 0) + (nextY - Number(this._chatSidebarPointerStartY || 0));
      this.chatSidebarDragLeft = this.chatSidebarClampLeft(candidateLeft);
      this.chatSidebarDragTop = this.chatSidebarClampTop(candidateTop);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    endChatSidebarPointerDrag() {
      if (!this._chatSidebarPointerActive) return;
      this._chatSidebarPointerActive = false;
      this.unbindChatSidebarPointerListeners();
      if (!this._chatSidebarPointerMoved) {
        this.chatSidebarDragActive = false;
        this._chatSidebarPointerFromPulltab = false;
        return;
      }
      this._chatSidebarPointerMoved = false;
      var finalLeft = this.chatSidebarClampLeft(this.chatSidebarDragLeft);
      var finalTop = this.chatSidebarClampTop(this.chatSidebarDragTop);
      this.chatSidebarDragLeft = finalLeft;
      this.chatSidebarDragTop = finalTop;
      this.chatSidebarPersistPlacementFromLeft(finalLeft);
      this.chatSidebarPersistPlacementFromTop(finalTop);
      this.chatSidebarDragActive = false;
      if (this._chatSidebarPointerFromPulltab) {
        this._sidebarToggleSuppressUntil = Date.now() + 260;
      }
      this._chatSidebarPointerFromPulltab = false;
    },

    shouldSuppressSidebarToggle() {
      var until = Number(this._sidebarToggleSuppressUntil || 0);
      return Number.isFinite(until) && until > Date.now();
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
