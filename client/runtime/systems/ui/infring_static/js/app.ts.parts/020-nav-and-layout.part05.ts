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
